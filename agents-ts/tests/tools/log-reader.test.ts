import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mkdirSync, writeFileSync, rmSync, existsSync } from "node:fs";
import { join } from "node:path";

// set LOG_DIR before importing the module
const TEST_LOG_DIR = join(import.meta.dirname, "..", ".test-logs");
process.env.LOG_DIR = TEST_LOG_DIR;

import { readLogs } from "../../src/tools/log-reader.js";

function jsonLine(fields: Record<string, unknown>): string {
  return JSON.stringify(fields);
}

describe("readLogs", () => {
  beforeEach(() => {
    mkdirSync(TEST_LOG_DIR, { recursive: true });
  });

  afterEach(() => {
    if (existsSync(TEST_LOG_DIR)) {
      rmSync(TEST_LOG_DIR, { recursive: true, force: true });
    }
  });

  it("should return error when log directory is missing", () => {
    rmSync(TEST_LOG_DIR, { recursive: true, force: true });
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "paper_trader" }));
    expect(result.error).toContain("log directory not found");
  });

  it("should return error when no matching files exist", () => {
    writeFileSync(join(TEST_LOG_DIR, "agents_2025-06-14.log"), "line\n");
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "paper_trader" }));
    expect(result.error).toContain("no log files matching");
    expect(result.available_files).toContain("agents_2025-06-14.log");
  });

  it("should read all lines from a matching file", () => {
    const lines = [
      jsonLine({ level: "INFO", message: "started" }),
      jsonLine({ level: "INFO", message: "tick", ticker: "SPY" }),
      jsonLine({ level: "WARN", message: "slow tick" }),
    ].join("\n") + "\n";

    writeFileSync(join(TEST_LOG_DIR, "paper_trader_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "paper_trader" }));
    expect(result.file).toBe("paper_trader_2025-06-15.log");
    expect(result.total_lines).toBe(3);
    expect(result.filtered_lines).toBe(3);
    expect(result.returned_lines).toBe(3);
  });

  it("should filter by level", () => {
    const lines = [
      jsonLine({ level: "INFO", message: "ok" }),
      jsonLine({ level: "WARN", message: "warning" }),
      jsonLine({ level: "ERROR", message: "bad" }),
      jsonLine({ level: "INFO", message: "ok2" }),
    ].join("\n") + "\n";

    writeFileSync(join(TEST_LOG_DIR, "agents_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "agents", level: "WARN" }));
    expect(result.filtered_lines).toBe(1);
    expect(result.lines).toHaveLength(1);
    expect(JSON.parse(result.lines[0]).message).toBe("warning");
  });

  it("should filter by ticker", () => {
    const lines = [
      jsonLine({ level: "INFO", message: "tick", ticker: "SPY" }),
      jsonLine({ level: "INFO", message: "tick", ticker: "QQQ" }),
      jsonLine({ level: "INFO", message: "tick", ticker: "SPY" }),
    ].join("\n") + "\n";

    writeFileSync(join(TEST_LOG_DIR, "paper_trader_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "paper_trader", ticker: "QQQ" }));
    expect(result.filtered_lines).toBe(1);
    expect(result.lines).toHaveLength(1);
  });

  it("should respect tail limit", () => {
    const lines = Array.from({ length: 50 }, (_, i) =>
      jsonLine({ level: "INFO", message: `line ${i}` }),
    ).join("\n") + "\n";

    writeFileSync(join(TEST_LOG_DIR, "backtest_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "backtest", tail: 10 }));
    expect(result.total_lines).toBe(50);
    expect(result.returned_lines).toBe(10);
    // should be the last 10 lines
    expect(JSON.parse(result.lines[0]).message).toBe("line 40");
    expect(JSON.parse(result.lines[9]).message).toBe("line 49");
  });

  it("should combine level and ticker filters", () => {
    const lines = [
      jsonLine({ level: "INFO", message: "tick", ticker: "SPY" }),
      jsonLine({ level: "WARN", message: "slow", ticker: "SPY" }),
      jsonLine({ level: "WARN", message: "slow", ticker: "QQQ" }),
      jsonLine({ level: "INFO", message: "tick", ticker: "QQQ" }),
    ].join("\n") + "\n";

    writeFileSync(join(TEST_LOG_DIR, "paper_trader_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({
      date: "2025-06-15",
      source: "paper_trader",
      level: "WARN",
      ticker: "SPY",
    }));
    expect(result.filtered_lines).toBe(1);
    expect(JSON.parse(result.lines[0]).message).toBe("slow");
  });

  it("should handle plain text lines with fallback matching", () => {
    const lines = "INFO starting up\nWARN something\nERROR crash SPY\n";
    writeFileSync(join(TEST_LOG_DIR, "paper_trader_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "paper_trader", level: "ERROR" }));
    expect(result.filtered_lines).toBe(1);
    expect(result.lines[0]).toContain("crash SPY");
  });

  it("should default tail to 200", () => {
    const lines = Array.from({ length: 300 }, (_, i) =>
      jsonLine({ level: "INFO", message: `line ${i}` }),
    ).join("\n") + "\n";

    writeFileSync(join(TEST_LOG_DIR, "agents_2025-06-15.log"), lines);
    const result = JSON.parse(readLogs({ date: "2025-06-15", source: "agents" }));
    expect(result.total_lines).toBe(300);
    expect(result.returned_lines).toBe(200);
  });
});
