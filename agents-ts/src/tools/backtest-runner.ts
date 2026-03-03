/**
 * backtest validation via rust CLI binary.
 *
 * runs proposed configs through the backtest engine and validates
 * results against thresholds before promoting.
 */

import { execFile } from "node:child_process";
import { writeFile, unlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import {
  type ValidationThresholds,
  createValidationThresholds,
} from "../models.js";

/** default data directory for historical candle CSVs */
export const DEFAULT_DATA_DIR = process.env.BACKTEST_DATA_DIR ?? "";

export async function runBacktestValidation(
  proposedConfig: Record<string, unknown>,
  dataPath: string,
  ticker: string,
  timeoutSeconds = 120,
): Promise<Record<string, unknown> | null> {
  const configPath = join(tmpdir(), `config-${randomUUID()}.json`);

  try {
    await writeFile(configPath, JSON.stringify(proposedConfig));

    const result = await new Promise<{ stdout: string; stderr: string }>(
      (resolve, reject) => {
        execFile(
          "cargo",
          [
            "run",
            "-p",
            "backtest",
            "--",
            "--config",
            configPath,
            "--data",
            dataPath,
            "--ticker",
            ticker,
          ],
          { timeout: timeoutSeconds * 1000 },
          (error, stdout, stderr) => {
            if (error) {
              reject(error);
              return;
            }
            resolve({ stdout, stderr });
          },
        );
      },
    );

    return JSON.parse(result.stdout) as Record<string, unknown>;
  } catch (error: unknown) {
    const message =
      error instanceof Error ? error.message : String(error);
    console.error(`backtest error: ${message}`);
    return null;
  } finally {
    try {
      await unlink(configPath);
    } catch {
      // ignore cleanup errors
    }
  }
}

export function validateBacktestResult(
  currentMetrics: Record<string, unknown> | null,
  proposedMetrics: Record<string, unknown>,
  thresholds?: ValidationThresholds,
): [boolean, string[]] {
  const t = thresholds ?? createValidationThresholds();
  const reasons: string[] = [];

  // min trades gate
  const totalTrades = (proposedMetrics.total_trades as number) ?? 0;
  if (totalTrades < t.min_trades) {
    reasons.push(
      `insufficient trades: ${totalTrades} < ${t.min_trades}`,
    );
  }

  // min win rate gate
  const winRate = (proposedMetrics.win_rate as number) ?? 0.0;
  if (winRate < t.min_win_rate) {
    reasons.push(
      `win rate too low: ${(winRate * 100).toFixed(2)}% < ${(t.min_win_rate * 100).toFixed(2)}%`,
    );
  }

  // max drawdown gate
  const maxDd = (proposedMetrics.max_drawdown_pct as number) ?? 0.0;
  if (maxDd > t.max_drawdown_pct) {
    reasons.push(
      `max drawdown too high: ${(maxDd * 100).toFixed(2)}% > ${(t.max_drawdown_pct * 100).toFixed(2)}%`,
    );
  }

  // sharpe degradation gate (only if we have current metrics to compare)
  if (currentMetrics !== null) {
    const currentSharpe = (currentMetrics.sharpe_ratio as number) ?? 0.0;
    const proposedSharpe = (proposedMetrics.sharpe_ratio as number) ?? 0.0;
    const degradation = currentSharpe - proposedSharpe;
    if (degradation > t.max_sharpe_degradation) {
      reasons.push(
        `sharpe degradation too large: ${degradation.toFixed(3)} > ${t.max_sharpe_degradation.toFixed(3)}`,
      );
    }
  }

  return [reasons.length === 0, reasons];
}
