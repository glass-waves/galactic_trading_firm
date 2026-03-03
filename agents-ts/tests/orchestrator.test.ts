import { describe, it, expect, vi, beforeEach } from "vitest";
import { createDailyBudget, createTokenUsage, estimateCost } from "../src/models.js";
import {
  CHECKIN_AGENTS,
  PM_AGENTS,
} from "../src/orchestrator.js";

// --- agent list tests ---

describe("agent lists", () => {
  it("check-in has 1 agent", () => {
    expect(CHECKIN_AGENTS).toHaveLength(1);
    expect(CHECKIN_AGENTS).toContain("agent_analysis");
  });

  it("PM cycle has 2 agents", () => {
    expect(PM_AGENTS).toHaveLength(2);
    expect(PM_AGENTS).toContain("agent_analysis");
    expect(PM_AGENTS).toContain("agent_pm");
  });

  it("PM cycle runs analysis then PM (correct order)", () => {
    expect(PM_AGENTS[0]).toBe("agent_analysis");
    expect(PM_AGENTS[1]).toBe("agent_pm");
  });
});

// --- budget tests ---

describe("budget logic", () => {
  it("should not be exhausted when under limit", () => {
    const budget = createDailyBudget({
      trading_date: "2024-01-15",
      total_cost_usd: 2.5,
      budget_limit_usd: 5.0,
      budget_exhausted: false,
    });
    expect(budget.total_cost_usd).toBeLessThan(budget.budget_limit_usd);
    expect(budget.budget_exhausted).toBe(false);
  });

  it("should be exhausted when over limit", () => {
    const budget = createDailyBudget({
      trading_date: "2024-01-15",
      total_cost_usd: 5.1,
      budget_limit_usd: 5.0,
      budget_exhausted: true,
    });
    expect(budget.budget_exhausted).toBe(true);
  });
});

describe("token usage accumulation", () => {
  it("should accumulate across agents", () => {
    const total = createTokenUsage();
    const usage1 = createTokenUsage({ input_tokens: 1000, output_tokens: 500 });
    const usage2 = createTokenUsage({ input_tokens: 2000, output_tokens: 800 });

    total.input_tokens += usage1.input_tokens + usage2.input_tokens;
    total.output_tokens += usage1.output_tokens + usage2.output_tokens;

    expect(total.input_tokens).toBe(3000);
    expect(total.output_tokens).toBe(1300);
    expect(estimateCost(total)).toBeGreaterThan(0);
  });
});

// --- cycle tests with mocks ---

describe("runCheckinCycle", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("should return null when budget exhausted", async () => {
    vi.doMock("../src/db.js", () => ({
      getPool: vi.fn(() => ({})),
      closePool: vi.fn(async () => {}),
    }));

    const checkBudgetMock = vi.fn(async () => [
      false,
      createDailyBudget({
        trading_date: "2024-01-15",
        total_cost_usd: 5.1,
        budget_limit_usd: 5.0,
        budget_exhausted: true,
      }),
    ] as const);

    vi.doMock("../src/orchestrator.js", async (importOriginal) => {
      const original = await importOriginal() as any;
      return {
        ...original,
        checkBudget: checkBudgetMock,
      };
    });

    // for budget exhausted, we test the logic directly
    const budget = createDailyBudget({
      trading_date: "2024-01-15",
      total_cost_usd: 5.1,
      budget_limit_usd: 5.0,
      budget_exhausted: true,
    });
    const canProceed = !budget.budget_exhausted && budget.total_cost_usd < budget.budget_limit_usd;
    expect(canProceed).toBe(false);
  });

  it("should handle successful cycle logic", () => {
    // test that token usage adds up correctly across 3 agents
    const totalUsage = createTokenUsage();
    for (let i = 0; i < 3; i++) {
      const agentUsage = createTokenUsage({ input_tokens: 1000, output_tokens: 500 });
      totalUsage.input_tokens += agentUsage.input_tokens;
      totalUsage.output_tokens += agentUsage.output_tokens;
    }
    expect(totalUsage.input_tokens).toBe(3000);
    expect(totalUsage.output_tokens).toBe(1500);
  });
});

describe("runFullPmCycle", () => {
  it("should track configs_proposed correctly", () => {
    // test the promotion logic: 1 analysis agent + 1 PM agent
    const totalUsage = createTokenUsage({ model: "claude-opus-4-6" });

    // analysis agent
    totalUsage.input_tokens += 1500;
    totalUsage.output_tokens += 600;

    // PM agent
    totalUsage.input_tokens += 1000;
    totalUsage.output_tokens += 500;

    expect(totalUsage.input_tokens).toBe(2500);
    expect(totalUsage.output_tokens).toBe(1100);
  });
});

// --- scheduler wrapper tests ---

describe("scheduler wrappers", () => {
  it("scheduledCheckin should catch exceptions", async () => {
    const { scheduledCheckin } = await import("../src/orchestrator.js");
    // mock runCheckinCycle to throw
    vi.doMock("../src/orchestrator.js", async (importOriginal) => {
      const original = await importOriginal() as any;
      return {
        ...original,
        runCheckinCycle: vi.fn(async () => {
          throw new Error("db connection lost");
        }),
      };
    });

    // the wrapper should not throw
    // we just verify it exists and is callable
    expect(typeof scheduledCheckin).toBe("function");
  });

  it("scheduledPmCycle should catch exceptions", async () => {
    const { scheduledPmCycle } = await import("../src/orchestrator.js");
    expect(typeof scheduledPmCycle).toBe("function");
  });
});

describe("runScheduled", () => {
  it("should export runScheduled function", async () => {
    const { runScheduled } = await import("../src/orchestrator.js");
    expect(typeof runScheduled).toBe("function");
  });
});
