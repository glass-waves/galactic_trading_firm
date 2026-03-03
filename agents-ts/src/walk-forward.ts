/**
 * walk-forward simulation script.
 *
 * loops through historical trading dates, running backtest then PM cycle
 * for each day. agents iterate on config day-by-day — each day's backtest
 * results feed into the next PM cycle, which may tune the config.
 *
 * usage:
 *   npx tsx src/walk-forward.ts --start 2025-10-01 --end 2025-10-14
 *   npx tsx src/walk-forward.ts --start 2025-10-01 --end 2025-10-14 --lookback-days 5 --model sonnet
 */

import { execSync } from "node:child_process";
import { parseArgs } from "node:util";
import pg from "pg";

import { logger } from "./logger.js";
import { runFullPmCycle } from "./orchestrator.js";
import { SONNET_MODEL, OPUS_MODEL } from "./agent-base.js";
import { getDatabaseUrl } from "./db.js";

const { Pool } = pg;

interface DaySummary {
  date: string;
  trades: number;
  pnl: number;
  winRate: number;
  configVersion: number | null;
  promoted: boolean;
}

function generateWeekdays(start: string, end: string): string[] {
  const dates: string[] = [];
  const current = new Date(`${start}T00:00:00`);
  const endDate = new Date(`${end}T00:00:00`);

  while (current <= endDate) {
    const day = current.getDay();
    // skip saturday (6) and sunday (0)
    if (day !== 0 && day !== 6) {
      dates.push(current.toISOString().slice(0, 10));
    }
    current.setDate(current.getDate() + 1);
  }

  return dates;
}

function resolveModel(modelArg: string | undefined): string {
  if (!modelArg || modelArg === "sonnet") return SONNET_MODEL;
  if (modelArg === "opus") return OPUS_MODEL;
  return modelArg;
}

async function queryDaySummary(
  pool: pg.Pool,
  date: string,
): Promise<{ trades: number; pnl: number; winRate: number }> {
  const result = await pool.query(
    `
    SELECT
      count(*) as trade_count,
      coalesce(sum(pnl_dollars), 0) as total_pnl,
      case when count(*) > 0
        then count(*) filter (where pnl_dollars > 0)::float / count(*)
        else 0
      end as win_rate
    FROM trades
    WHERE entry_fill_at::date = $1
    `,
    [date],
  );

  const row = result.rows[0];
  return {
    trades: Number(row.trade_count),
    pnl: Number(row.total_pnl),
    winRate: Number(row.win_rate),
  };
}

async function queryLatestConfig(
  pool: pg.Pool,
): Promise<{ id: number; promoted: boolean } | null> {
  const result = await pool.query(
    `
    SELECT id, status, promoted_at
    FROM config_versions
    ORDER BY id DESC
    LIMIT 1
    `,
  );

  if (result.rows.length === 0) return null;
  const row = result.rows[0];
  return {
    id: Number(row.id),
    promoted: row.status === "promoted",
  };
}

function formatPnl(pnl: number): string {
  const sign = pnl >= 0 ? "+" : "-";
  return `${sign}$${Math.abs(pnl).toFixed(2)}`;
}

function printSummaryTable(summaries: DaySummary[]): void {
  console.log("\n" + "=".repeat(75));
  console.log("walk-forward simulation summary");
  console.log("=".repeat(75));
  console.log(
    `${"date".padEnd(12)} ${"trades".padStart(7)} ${"P&L".padStart(12)} ${"win rate".padStart(9)} ${"config".padStart(8)} ${"promoted".padStart(9)}`,
  );
  console.log("-".repeat(75));

  let totalTrades = 0;
  let totalPnl = 0;

  for (const s of summaries) {
    totalTrades += s.trades;
    totalPnl += s.pnl;

    console.log(
      `${s.date.padEnd(12)} ${String(s.trades).padStart(7)} ${formatPnl(s.pnl).padStart(12)} ${(s.winRate * 100).toFixed(1).padStart(8)}% ${(s.configVersion !== null ? `v${s.configVersion}` : "—").padStart(8)} ${(s.promoted ? "yes" : "no").padStart(9)}`,
    );
  }

  console.log("-".repeat(75));
  console.log(
    `${"total".padEnd(12)} ${String(totalTrades).padStart(7)} ${formatPnl(totalPnl).padStart(12)}`,
  );
  console.log("=".repeat(75));
}

async function main(): Promise<void> {
  const { values } = parseArgs({
    options: {
      start: { type: "string" },
      end: { type: "string" },
      "lookback-days": { type: "string", default: "5" },
      model: { type: "string", default: "sonnet" },
      "budget-limit": { type: "string", default: "50" },
    },
  });

  if (!values.start || !values.end) {
    console.error("usage: walk-forward --start YYYY-MM-DD --end YYYY-MM-DD [--lookback-days N] [--model sonnet|opus]");
    process.exit(1);
  }

  const startDate = values.start;
  const endDate = values.end;
  const lookbackDays = parseInt(values["lookback-days"] ?? "5", 10);
  const model = resolveModel(values.model);
  const budgetLimit = parseFloat(values["budget-limit"] ?? "50");

  const dates = generateWeekdays(startDate, endDate);

  if (dates.length === 0) {
    console.error("no weekdays in the specified range");
    process.exit(1);
  }

  console.log(`walk-forward simulation: ${startDate} → ${endDate}`);
  console.log(`  dates: ${dates.length} trading days`);
  console.log(`  lookback: ${lookbackDays} days`);
  console.log(`  model: ${model}`);
  console.log(`  budget limit: $${budgetLimit}/day`);
  console.log("");

  const pool = new Pool({
    connectionString: getDatabaseUrl(),
    max: 5,
  });

  const summaries: DaySummary[] = [];

  try {
    for (const date of dates) {
      console.log(`\n${"=".repeat(60)}`);
      console.log(`day: ${date}`);
      console.log("=".repeat(60));

      // 1. clear prior backtest trades for this date (idempotent reruns)
      await pool.query(
        `DELETE FROM trades WHERE entry_fill_at::date = $1 AND is_paper = true`,
        [date],
      );

      // 2. run backtest with --write-db
      console.log("\nrunning backtest...");
      try {
        const cmd = `cargo run -p backtest -- --date ${date} --lookback-days ${lookbackDays} --write-db`;
        const output = execSync(cmd, {
          cwd: process.env.CARGO_WORKSPACE ?? process.cwd().replace(/\/agents-ts$/, ""),
          encoding: "utf-8",
          stdio: ["pipe", "pipe", "pipe"],
          timeout: 300_000, // 5 min timeout
        });
        console.log(output.trim());
      } catch (err) {
        const execErr = err as { stderr?: string; message?: string };
        console.error(`backtest failed: ${execErr.stderr ?? execErr.message}`);
        // still continue to query what we have
      }

      // 3. run PM cycle for this date
      console.log("\nrunning PM cycle...");
      try {
        const usage = await runFullPmCycle(pool, {
          tradingDate: date,
          budgetLimit,
          model,
        });
        if (usage) {
          console.log(`PM cycle complete: ${usage.input_tokens} input, ${usage.output_tokens} output tokens`);
        } else {
          console.log("PM cycle skipped (budget exhausted)");
        }
      } catch (err) {
        console.error(`PM cycle failed: ${err}`);
      }

      // 4. query day's trade summary
      const { trades, pnl, winRate } = await queryDaySummary(pool, date);

      // 5. check latest config version
      const latestConfig = await queryLatestConfig(pool);

      const summary: DaySummary = {
        date,
        trades,
        pnl,
        winRate,
        configVersion: latestConfig?.id ?? null,
        promoted: latestConfig?.promoted ?? false,
      };
      summaries.push(summary);

      console.log(`\nday summary: ${trades} trades, ${formatPnl(pnl)}, win rate ${(winRate * 100).toFixed(1)}%`);
    }

    // print final summary table
    printSummaryTable(summaries);
  } finally {
    await pool.end();
  }
}

main().catch((err) => {
  logger.error(`walk-forward failed: ${err}`);
  process.exit(1);
});
