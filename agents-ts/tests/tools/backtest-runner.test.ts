import { describe, it, expect } from "vitest";
import { validateBacktestResult } from "../../src/tools/backtest-runner.js";
import { createValidationThresholds } from "../../src/models.js";

describe("validateBacktestResult", () => {
  it("should pass when within thresholds", () => {
    const current = { sharpe_ratio: 1.5, win_rate: 0.55, total_trades: 50 };
    const proposed = {
      sharpe_ratio: 1.4,
      win_rate: 0.52,
      max_drawdown_pct: 0.08,
      total_trades: 40,
    };
    const [passed, reasons] = validateBacktestResult(current, proposed);
    expect(passed).toBe(true);
    expect(reasons).toEqual([]);
  });

  it("should fail on sharpe degradation", () => {
    const current = { sharpe_ratio: 2.0 };
    const proposed = {
      sharpe_ratio: 1.0,
      win_rate: 0.5,
      max_drawdown_pct: 0.05,
      total_trades: 20,
    };
    const [passed, reasons] = validateBacktestResult(current, proposed);
    expect(passed).toBe(false);
    expect(reasons.some((r) => r.includes("sharpe degradation"))).toBe(true);
  });

  it("should fail on low win rate", () => {
    const proposed = {
      sharpe_ratio: 1.0,
      win_rate: 0.2,
      max_drawdown_pct: 0.05,
      total_trades: 20,
    };
    const [passed, reasons] = validateBacktestResult(null, proposed);
    expect(passed).toBe(false);
    expect(reasons.some((r) => r.includes("win rate"))).toBe(true);
  });

  it("should fail on high drawdown", () => {
    const proposed = {
      sharpe_ratio: 1.0,
      win_rate: 0.5,
      max_drawdown_pct: 0.25,
      total_trades: 20,
    };
    const [passed, reasons] = validateBacktestResult(null, proposed);
    expect(passed).toBe(false);
    expect(reasons.some((r) => r.includes("drawdown"))).toBe(true);
  });

  it("should fail on insufficient trades", () => {
    const proposed = {
      sharpe_ratio: 1.0,
      win_rate: 0.5,
      max_drawdown_pct: 0.05,
      total_trades: 2,
    };
    const [passed, reasons] = validateBacktestResult(null, proposed);
    expect(passed).toBe(false);
    expect(reasons.some((r) => r.includes("insufficient trades"))).toBe(true);
  });

  it("should respect custom thresholds", () => {
    const thresholds = createValidationThresholds({
      max_sharpe_degradation: 0.1,
      min_win_rate: 0.6,
      max_drawdown_pct: 0.05,
      min_trades: 10,
    });
    const proposed = {
      sharpe_ratio: 1.0,
      win_rate: 0.55,
      max_drawdown_pct: 0.06,
      total_trades: 8,
    };
    const [passed, reasons] = validateBacktestResult(
      null,
      proposed,
      thresholds,
    );
    expect(passed).toBe(false);
    // should have multiple failures
    expect(reasons.length).toBeGreaterThanOrEqual(2);
  });

  it("should skip sharpe check when no current metrics", () => {
    const proposed = {
      sharpe_ratio: 0.1,
      win_rate: 0.5,
      max_drawdown_pct: 0.05,
      total_trades: 20,
    };
    const [passed, reasons] = validateBacktestResult(null, proposed);
    expect(passed).toBe(true);
    expect(reasons.every((r) => !r.includes("sharpe"))).toBe(true);
  });
});
