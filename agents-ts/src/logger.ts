/**
 * file logging utility.
 *
 * mirrors console output to a JSON-lines log file at {LOG_DIR}/agents_{date}.log.
 * appendFileSync is fine here — agents are cron jobs, not latency-sensitive.
 */

import { appendFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const LOG_DIR = process.env.LOG_DIR ?? "logs";

// ensure log dir exists on module load
mkdirSync(LOG_DIR, { recursive: true });

function logFilePath(): string {
  const today = new Date().toISOString().slice(0, 10);
  return join(LOG_DIR, `agents_${today}.log`);
}

function appendToFile(
  level: string,
  message: string,
  extra?: Record<string, unknown>,
): void {
  const entry: Record<string, unknown> = {
    timestamp: new Date().toISOString(),
    level,
    message,
  };
  if (extra) {
    entry.extra = extra;
  }
  try {
    appendFileSync(logFilePath(), `${JSON.stringify(entry)}\n`);
  } catch {
    // best-effort — don't crash if file write fails
  }
}

export const logger = {
  info(message: string, extra?: Record<string, unknown>): void {
    console.log(message);
    appendToFile("INFO", message, extra);
  },
  warn(message: string, extra?: Record<string, unknown>): void {
    console.warn(message);
    appendToFile("WARN", message, extra);
  },
  error(message: string, extra?: Record<string, unknown>): void {
    console.error(message);
    appendToFile("ERROR", message, extra);
  },
};
