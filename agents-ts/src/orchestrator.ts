/**
 * main scheduler, budget tracking, cycle dispatch.
 *
 * orchestrates the two-tier evolution cycle:
 * - full PM cycles (opus 4.6): 1x daily at 15:30 ET, analysis + PM agents, config authority
 * - mid-day analysis (sonnet 4.6): 1x daily at 12:30 ET, single analysis agent, no config authority
 *
 * enforces a $5/day budget cap under anthropic tier 1.
 */

import { existsSync } from "node:fs";
import { join } from "node:path";
import { parseArgs } from "node:util";
import type pg from "pg";
import { Cron } from "croner";

import { logger } from "./logger.js";

import {
  type AgentType,
  type CycleType,
  type DailyBudget,
  type TokenUsage,
  createDailyBudget,
  createTokenUsage,
  estimateCost,
} from "./models.js";
import { getPool, closePool } from "./db.js";
import {
  runPmAgent,
  runAnalysisAgent,
  SONNET_MODEL,
  OPUS_MODEL,
} from "./agent-base.js";
import {
  runBacktestValidation,
  validateBacktestResult,
  DEFAULT_DATA_DIR,
} from "./tools/backtest-runner.js";
import { computeConfigDiff } from "./tools/config-diff.js";
import {
  getCurrentConfig,
  getConfigVersion,
  updateConfigStatus,
  writeChangelogEntries,
} from "./tools/config-ops.js";

// agent lists — 2 agents per PM cycle, 1 per check-in
export const CHECKIN_AGENTS: AgentType[] = ["agent_analysis"];
export const PM_AGENTS: AgentType[] = ["agent_analysis", "agent_pm"];

// default budget cap per day (usd)
const DEFAULT_BUDGET_LIMIT = 5.0;

export interface PmCycleOptions {
  tradingDate?: string;   // override todayStr() for simulation
  budgetLimit?: number;   // override $5 default
  model?: string;         // override OPUS_MODEL
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

export async function getOrCreateDailyBudget(
  pool: pg.Pool,
  tradingDate?: string,
): Promise<DailyBudget> {
  const d = tradingDate ?? todayStr();

  const result = await pool.query(
    "SELECT * FROM daily_budget WHERE trading_date = $1",
    [d],
  );

  if (result.rows.length > 0) {
    const row = result.rows[0];
    return createDailyBudget({
      trading_date: String(row.trading_date),
      total_input_tokens: row.total_input_tokens,
      total_output_tokens: row.total_output_tokens,
      total_cost_usd: Number(row.total_cost_usd),
      full_pm_cycles: row.full_pm_cycles,
      checkin_cycles: row.checkin_cycles,
      budget_limit_usd: Number(row.budget_limit_usd),
      budget_exhausted: row.budget_exhausted,
    });
  }

  // create new budget row for today
  await pool.query(
    "INSERT INTO daily_budget (trading_date, budget_limit_usd) VALUES ($1, $2)",
    [d, DEFAULT_BUDGET_LIMIT],
  );
  return createDailyBudget({ trading_date: d, budget_limit_usd: DEFAULT_BUDGET_LIMIT });
}

export async function checkBudget(
  pool: pg.Pool,
  tradingDate?: string,
  budgetLimit?: number,
): Promise<[boolean, DailyBudget]> {
  const budget = await getOrCreateDailyBudget(pool, tradingDate);
  const limit = budgetLimit ?? budget.budget_limit_usd;
  const canProceed =
    !budget.budget_exhausted && budget.total_cost_usd < limit;
  return [canProceed, budget];
}

export async function updateBudget(
  pool: pg.Pool,
  usage: TokenUsage,
  cycleType: CycleType = "checkin",
  tradingDate?: string,
): Promise<DailyBudget> {
  const d = tradingDate ?? todayStr();
  const cost = estimateCost(usage);
  const cycleCol =
    cycleType === "checkin" ? "checkin_cycles" : "full_pm_cycles";

  await pool.query(
    `
    UPDATE daily_budget SET
      total_input_tokens = total_input_tokens + $1,
      total_output_tokens = total_output_tokens + $2,
      total_cost_usd = total_cost_usd + $3,
      ${cycleCol} = ${cycleCol} + 1,
      budget_exhausted = (total_cost_usd + $3) >= budget_limit_usd
    WHERE trading_date = $4
    `,
    [usage.input_tokens, usage.output_tokens, cost, d],
  );

  return getOrCreateDailyBudget(pool, d);
}

export async function createEvolutionCycle(
  pool: pg.Pool,
  cycleType: CycleType,
  agents: AgentType[],
  model: string = SONNET_MODEL,
  tradingDate?: string,
): Promise<number> {
  const d = tradingDate ?? todayStr();

  const result = await pool.query(
    `
    INSERT INTO evolution_cycles (
      trading_date, cycle_type, model_used, agents_triggered
    ) VALUES (
      $1, $2::cycle_type, $3,
      $4::agent_type[]
    )
    RETURNING id
    `,
    [d, cycleType, model, agents],
  );
  return result.rows[0].id as number;
}

export async function completeEvolutionCycle(
  pool: pg.Pool,
  cycleId: number,
  agentsCompleted: AgentType[],
  usage: TokenUsage,
  configsProposed = 0,
  configsPromoted = 0,
  configsRejected = 0,
): Promise<void> {
  const cost = estimateCost(usage);

  await pool.query(
    `
    UPDATE evolution_cycles SET
      completed_at = now(),
      agents_completed = $1::agent_type[],
      input_tokens_used = $2,
      output_tokens_used = $3,
      estimated_cost_usd = $4,
      configs_proposed = $5,
      configs_promoted = $6,
      configs_rejected = $7
    WHERE id = $8
    `,
    [
      agentsCompleted,
      usage.input_tokens,
      usage.output_tokens,
      cost,
      configsProposed,
      configsPromoted,
      configsRejected,
      cycleId,
    ],
  );
}

export async function runCheckinCycle(
  pool?: pg.Pool,
): Promise<TokenUsage | null> {
  const ownPool = pool === undefined;
  if (pool === undefined) pool = getPool();

  try {
    // 1. check budget
    const [canProceed, budget] = await checkBudget(pool);
    if (!canProceed) {
      logger.warn(
        `budget exhausted for ${budget.trading_date}: ` +
          `$${budget.total_cost_usd.toFixed(2)} / $${budget.budget_limit_usd.toFixed(2)}`,
      );
      return null;
    }

    // 2. create evolution cycle record — mid-day analysis agent
    const agents = CHECKIN_AGENTS;
    const cycleId = await createEvolutionCycle(pool, "checkin", agents, SONNET_MODEL);
    logger.info(`starting mid-day analysis cycle ${cycleId}`);

    // 3. run mid-day analysis agent (sonnet, covers all timescales)
    const totalUsage = createTokenUsage();
    const agentsCompleted: AgentType[] = [];

    try {
      logger.info("running mid-day analysis agent (sonnet)...");
      const usage = await runAnalysisAgent(pool, cycleId, SONNET_MODEL, 15);
      totalUsage.input_tokens += usage.input_tokens;
      totalUsage.output_tokens += usage.output_tokens;
      agentsCompleted.push("agent_analysis");
    } catch (err) {
      logger.error(`mid-day analysis agent failed: ${err}`);
    }

    // 4. complete the evolution cycle
    await completeEvolutionCycle(pool, cycleId, agentsCompleted, totalUsage);

    // 5. update daily budget
    const updatedBudget = await updateBudget(pool, totalUsage, "checkin");
    const cost = estimateCost(totalUsage);
    logger.info(
      `mid-day analysis cycle ${cycleId} complete: ` +
        `${agentsCompleted.length}/${agents.length} agents, ` +
        `$${cost.toFixed(4)}, ` +
        `budget: $${updatedBudget.total_cost_usd.toFixed(2)}/$${updatedBudget.budget_limit_usd.toFixed(2)}`,
    );

    return totalUsage;
  } finally {
    if (ownPool) await closePool();
  }
}

export async function runFullPmCycle(
  pool?: pg.Pool,
  options?: PmCycleOptions,
): Promise<TokenUsage | null> {
  const ownPool = pool === undefined;
  if (pool === undefined) pool = getPool();
  const tradingDate = options?.tradingDate;
  const budgetLimit = options?.budgetLimit;
  const model = options?.model ?? OPUS_MODEL;

  try {
    // 1. check budget
    const [canProceed, budget] = await checkBudget(pool, tradingDate, budgetLimit);
    if (!canProceed) {
      logger.warn(
        `budget exhausted for ${budget.trading_date}: ` +
          `$${budget.total_cost_usd.toFixed(2)} / $${budget.budget_limit_usd.toFixed(2)}`,
      );
      return null;
    }

    // 2. create evolution cycle record — analysis + PM agents
    const agents = PM_AGENTS;
    const cycleId = await createEvolutionCycle(
      pool,
      "full_pm",
      agents,
      model,
      tradingDate,
    );
    logger.info(`starting full PM cycle ${cycleId}`);

    // 3. run analysis agent (covers all timescales in one pass)
    const totalUsage = createTokenUsage({ model });
    const agentsCompleted: AgentType[] = [];

    const [analysisOk] = await checkBudget(pool, tradingDate, budgetLimit);
    if (analysisOk) {
      try {
        logger.info("running analysis agent...");
        const usage = await runAnalysisAgent(pool, cycleId, model);
        totalUsage.input_tokens += usage.input_tokens;
        totalUsage.output_tokens += usage.output_tokens;
        agentsCompleted.push("agent_analysis");
      } catch (err) {
        logger.error(`analysis agent failed: ${err}`);
      }
    } else {
      logger.warn("budget exhausted before analysis agent");
    }

    // 4. run PM agent
    let configsProposed = 0;
    let configsPromoted = 0;
    let configsRejected = 0;

    const [canRunPm] = await checkBudget(pool, tradingDate, budgetLimit);
    if (canRunPm) {
      try {
        logger.info("running PM agent...");
        const [pmUsage, proposedVersionId] = await runPmAgent(pool, cycleId, model);
        totalUsage.input_tokens += pmUsage.input_tokens;
        totalUsage.output_tokens += pmUsage.output_tokens;
        agentsCompleted.push("agent_pm");

        // 5. handle config proposal
        if (proposedVersionId !== null) {
          configsProposed = 1;
          logger.info(`PM proposed config version ${proposedVersionId}`);

          // update status to backtesting
          await updateConfigStatus(pool, proposedVersionId, "backtesting");

          // when simulating a past date, skip backtest validation and auto-promote
          if (tradingDate) {
            logger.info(`walk-forward mode (${tradingDate}): auto-promoting config ${proposedVersionId}`);
            const proposed = await getConfigVersion(pool, proposedVersionId);
            const currentConfig = await getCurrentConfig(pool);
            const configBlob = proposed?.config_blob
              ? (proposed.config_blob as Record<string, unknown>)
              : {};
            await promoteConfig(
              pool,
              proposedVersionId,
              configBlob,
              currentConfig,
            );
            configsPromoted = 1;
          } else {

          // get backtest data directory
          const dataDir =
            process.env.BACKTEST_DATA_DIR ?? DEFAULT_DATA_DIR;

          if (dataDir && existsSync(dataDir)) {
            // run backtest validation
            const proposed = await getConfigVersion(pool, proposedVersionId);
            if (proposed?.config_blob) {
              const configBlob = proposed.config_blob as Record<string, unknown>;
              // get current config metrics for comparison
              const currentConfig = await getCurrentConfig(pool);
              let currentMetrics: Record<string, unknown> | null = null;
              if (currentConfig) {
                currentMetrics = {
                  sharpe_ratio: currentConfig.backtest_sharpe ?? 0,
                  win_rate: currentConfig.backtest_win_rate ?? 0,
                };
              }

              // find a data file for any configured ticker
              const tickers = (configBlob.tickers as string[]) ?? ["SPY"];
              const ticker = tickers[0] ?? "SPY";
              const dataFile = join(dataDir, `${ticker}.csv`);

              if (existsSync(dataFile)) {
                try {
                  const btResult = await runBacktestValidation(
                    configBlob,
                    dataFile,
                    ticker,
                  );
                  if (
                    btResult &&
                    typeof btResult === "object" &&
                    "metrics" in btResult
                  ) {
                    const metrics = btResult.metrics as Record<string, unknown>;
                    const [passed, reasons] = validateBacktestResult(
                      currentMetrics,
                      metrics,
                    );
                    if (passed) {
                      logger.info("backtest passed, promoting config");
                      await promoteConfig(
                        pool,
                        proposedVersionId,
                        configBlob,
                        currentConfig,
                        metrics,
                      );
                      configsPromoted = 1;
                    } else {
                      logger.warn(`backtest failed: ${reasons.join(", ")}`);
                      await updateConfigStatus(
                        pool,
                        proposedVersionId,
                        "rejected",
                      );
                      configsRejected = 1;
                    }
                  } else {
                    logger.error("backtest returned no metrics, rejecting");
                    await updateConfigStatus(
                      pool,
                      proposedVersionId,
                      "rejected",
                    );
                    configsRejected = 1;
                  }
                } catch (err) {
                  logger.error(`backtest error, rejecting config: ${err}`);
                  await updateConfigStatus(
                    pool,
                    proposedVersionId,
                    "rejected",
                  );
                  configsRejected = 1;
                }
              } else {
                logger.warn(
                  `no data file at ${dataFile}, auto-promoting`,
                );
                await promoteConfig(
                  pool,
                  proposedVersionId,
                  configBlob,
                  currentConfig,
                );
                configsPromoted = 1;
              }
            } else {
              logger.error("could not read proposed config, rejecting");
              await updateConfigStatus(
                pool,
                proposedVersionId,
                "rejected",
              );
              configsRejected = 1;
            }
          } else {
            // no data directory configured — auto-promote
            logger.warn("no BACKTEST_DATA_DIR configured, auto-promoting");
            const proposed = await getConfigVersion(pool, proposedVersionId);
            const currentConfig = await getCurrentConfig(pool);
            const configBlob = proposed?.config_blob
              ? (proposed.config_blob as Record<string, unknown>)
              : {};
            await promoteConfig(
              pool,
              proposedVersionId,
              configBlob,
              currentConfig,
            );
            configsPromoted = 1;
          }
          } // close else (non-walk-forward path)
        } else {
          logger.info("PM decided to hold steady (no config proposal)");
        }
      } catch (err) {
        logger.error(`PM agent failed: ${err}`);
      }
    }

    // 6. complete the evolution cycle
    await completeEvolutionCycle(
      pool,
      cycleId,
      agentsCompleted,
      totalUsage,
      configsProposed,
      configsPromoted,
      configsRejected,
    );

    // 7. update daily budget
    const updatedBudget = await updateBudget(pool, totalUsage, "full_pm", tradingDate);
    const cost = estimateCost(totalUsage);
    logger.info(
      `full PM cycle ${cycleId} complete: ` +
        `${agentsCompleted.length}/${agents.length} agents, ` +
        `proposed=${configsProposed} promoted=${configsPromoted} rejected=${configsRejected}, ` +
        `$${cost.toFixed(4)}, ` +
        `budget: $${updatedBudget.total_cost_usd.toFixed(2)}/$${updatedBudget.budget_limit_usd.toFixed(2)}`,
    );

    return totalUsage;
  } finally {
    if (ownPool) await closePool();
  }
}

async function promoteConfig(
  pool: pg.Pool,
  versionId: number,
  configBlob: Record<string, unknown>,
  currentConfig: Record<string, unknown> | null,
  backtestMetrics?: Record<string, unknown>,
): Promise<void> {
  await updateConfigStatus(pool, versionId, "promoted", backtestMetrics);

  // compute diff and write changelog
  const oldConfig =
    currentConfig && typeof currentConfig === "object" && "config" in currentConfig
      ? (currentConfig.config as Record<string, unknown>)
      : {};
  const changes = computeConfigDiff(oldConfig, configBlob);
  if (changes.length > 0) {
    await writeChangelogEntries(pool, versionId, changes, "agent_pm");
    logger.info(
      `wrote ${changes.length} changelog entries for config version ${versionId}`,
    );
  }
}

// --- scheduler wrappers ---

export async function scheduledCheckin(): Promise<void> {
  try {
    await runCheckinCycle();
  } catch (err) {
    logger.error(`scheduled check-in cycle failed: ${err}`);
  }
}

export async function scheduledPmCycle(): Promise<void> {
  try {
    await runFullPmCycle();
  } catch (err) {
    logger.error(`scheduled PM cycle failed: ${err}`);
  }
}

export async function runScheduled(): Promise<void> {
  const controller = new AbortController();

  const checkinJob = new Cron(
    "30 12 * * 1-5",
    { timezone: "America/New_York" },
    () => { scheduledCheckin(); },
  );

  const pmJob = new Cron(
    "30 15 * * 1-5",
    { timezone: "America/New_York" },
    () => { scheduledPmCycle(); },
  );

  const shutdown = () => {
    controller.abort();
  };

  process.on("SIGTERM", shutdown);
  process.on("SIGINT", shutdown);

  logger.info("scheduler started — mid-day analysis at 12:30 ET, PM at 15:30 ET");

  await new Promise<void>((resolve) => {
    controller.signal.addEventListener("abort", () => resolve());
  });

  logger.info("shutdown signal received, stopping scheduler");
  checkinJob.stop();
  pmJob.stop();
  await closePool();
  logger.info("scheduler stopped");
}

export async function runScheduledCheckins(
  intervalMinutes = 15,
  maxCycles?: number,
): Promise<void> {
  const pool = getPool();
  let cyclesRun = 0;

  try {
    while (maxCycles === undefined || cyclesRun < maxCycles) {
      const usage = await runCheckinCycle(pool);
      cyclesRun++;

      if (usage === null) {
        logger.info("budget exhausted, stopping scheduled check-ins");
        break;
      }

      if (maxCycles === undefined || cyclesRun < maxCycles) {
        logger.info(
          `sleeping ${intervalMinutes} minutes until next check-in...`,
        );
        await new Promise((r) => setTimeout(r, intervalMinutes * 60 * 1000));
      }
    }
  } finally {
    await closePool();
  }
}

export function main(): void {
  const { values } = parseArgs({
    options: {
      mode: {
        type: "string",
        default: "once",
      },
      interval: {
        type: "string",
        default: "15",
      },
    },
  });

  const mode = values.mode ?? "once";
  const interval = parseInt(values.interval ?? "15", 10);

  if (mode === "once") {
    runCheckinCycle().catch((err) => logger.error(String(err)));
  } else if (mode === "pm") {
    runFullPmCycle().catch((err) => logger.error(String(err)));
  } else if (mode === "scheduled") {
    runScheduled().catch((err) => logger.error(String(err)));
  } else if (mode === "checkin") {
    runScheduledCheckins(interval).catch((err) => logger.error(String(err)));
  } else {
    logger.error(`unknown mode: ${mode}`);
    process.exit(1);
  }
}

// run if this is the entry point
const isMain =
  typeof process !== "undefined" &&
  process.argv[1] &&
  (process.argv[1].endsWith("orchestrator.js") ||
    process.argv[1].endsWith("orchestrator.ts"));
if (isMain) {
  main();
}
