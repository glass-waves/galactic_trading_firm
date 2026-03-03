import { describe, it, expect } from "vitest";
import { computeConfigDiff } from "../../src/tools/config-diff.js";

function baseConfig() {
  return {
    tickers: ["SPY", "QQQ"],
    scoring: {
      entry_threshold: 0.65,
      exit_threshold: -0.3,
      aggregation: "WeightedSum",
      timescale_weights: {
        FiveMinute: 0.5,
        OneMinute: 0.3,
        Hourly: 0.2,
      },
      hard_gate_timescales: ["OneMinute"],
    },
    session: {
      no_new_entries_after: "15:30",
      force_exit_by: "15:55",
      avoid_first_minutes: 5,
      max_concurrent_positions: 3,
    },
    indicators: [
      {
        indicator_type: "rsi",
        instance_id: "rsi_14",
        timescale: "FiveMinute",
        enabled: true,
        weight: 1.0,
        params: { period: 14 },
      },
      {
        indicator_type: "ema",
        instance_id: "ema_20",
        timescale: "FiveMinute",
        enabled: true,
        weight: 0.8,
        params: { period: 20 },
      },
    ],
    actions: [
      {
        action_type: "score_threshold_entry",
        instance_id: "entry_1",
        enabled: true,
        params: {},
      },
    ],
  };
}

describe("computeConfigDiff", () => {
  it("should return no changes for identical configs", () => {
    const config = baseConfig();
    const changes = computeConfigDiff(config, config);
    expect(changes).toEqual([]);
  });

  it("should detect knob_tuned", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    (new_.indicators[0].params as Record<string, unknown>).period = 21;
    const changes = computeConfigDiff(old, new_);
    expect(changes).toHaveLength(1);
    expect(changes[0].change_category).toBe("knob_tuned");
    expect(changes[0].target_param).toBe("period");
    expect(changes[0].old_value).toBe(14);
    expect(changes[0].new_value).toBe(21);
    expect(changes[0].target_tool_id).toBe("rsi_14");
  });

  it("should detect tool_enabled", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    old.indicators[1].enabled = false;
    new_.indicators[1].enabled = true;
    const changes = computeConfigDiff(old, new_);
    const enabledChanges = changes.filter(
      (c) =>
        c.target_tool_id === "ema_20" && c.target_param === "enabled",
    );
    expect(enabledChanges).toHaveLength(1);
    expect(enabledChanges[0].change_category).toBe("tool_enabled");
  });

  it("should detect tool_disabled", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    old.indicators[1].enabled = true;
    new_.indicators[1].enabled = false;
    const changes = computeConfigDiff(old, new_);
    const disabledChanges = changes.filter(
      (c) =>
        c.target_tool_id === "ema_20" && c.target_param === "enabled",
    );
    expect(disabledChanges).toHaveLength(1);
    expect(disabledChanges[0].change_category).toBe("tool_disabled");
  });

  it("should detect weight_adjusted", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    new_.scoring.timescale_weights.FiveMinute = 0.6;
    const changes = computeConfigDiff(old, new_);
    const weightChanges = changes.filter(
      (c) => c.change_category === "weight_adjusted",
    );
    expect(weightChanges).toHaveLength(1);
    expect(weightChanges[0].target_timescale).toBe("FiveMinute");
    expect(weightChanges[0].old_value).toBe(0.5);
    expect(weightChanges[0].new_value).toBe(0.6);
  });

  it("should detect threshold_adjusted", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    new_.scoring.entry_threshold = 0.7;
    const changes = computeConfigDiff(old, new_);
    const thresholdChanges = changes.filter(
      (c) => c.change_category === "threshold_adjusted",
    );
    expect(thresholdChanges).toHaveLength(1);
    expect(thresholdChanges[0].target_param).toBe("entry_threshold");
    expect(thresholdChanges[0].old_value).toBe(0.65);
    expect(thresholdChanges[0].new_value).toBe(0.7);
  });

  it("should detect multiple changes", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    (new_.indicators[0].params as Record<string, unknown>).period = 21;
    new_.scoring.entry_threshold = 0.7;
    new_.session.avoid_first_minutes = 10;
    const changes = computeConfigDiff(old, new_);
    expect(changes).toHaveLength(3);
    const categories = new Set(changes.map((c) => c.change_category));
    expect(categories.has("knob_tuned")).toBe(true);
    expect(categories.has("threshold_adjusted")).toBe(true);
    expect(categories.has("session_rule_changed")).toBe(true);
  });

  it("should detect nested param change", () => {
    const old = baseConfig();
    const new_ = baseConfig();
    (new_.indicators[0].params as Record<string, unknown>).upper_band = 2.5;
    const changes = computeConfigDiff(old, new_);
    const paramChanges = changes.filter(
      (c) => c.target_param === "upper_band",
    );
    expect(paramChanges).toHaveLength(1);
    expect(paramChanges[0].change_category).toBe("knob_tuned");
    expect(paramChanges[0].old_value).toBeNull();
    expect(paramChanges[0].new_value).toBe(2.5);
  });
});
