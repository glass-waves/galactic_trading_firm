/**
 * config diff computation.
 *
 * compares two config blobs field by field and produces atomic change
 * dicts suitable for insertion into the config_changelog table.
 */

// valid change categories matching the config_changelog.change_category enum
export const CHANGE_CATEGORIES = new Set([
  "knob_tuned",
  "tool_enabled",
  "tool_disabled",
  "weight_adjusted",
  "threshold_adjusted",
  "session_rule_changed",
  "scoring_changed",
]);

export interface ConfigChange {
  change_category: string;
  target_timescale: string | null;
  target_tool_id: string | null;
  target_tool_type: string | null;
  target_param: string;
  old_value: unknown;
  new_value: unknown;
  reason: string;
}

function change(opts: {
  category: string;
  param: string;
  old_value?: unknown;
  new_value?: unknown;
  timescale?: string | null;
  tool_id?: string | null;
  tool_type?: string | null;
  reason?: string;
}): ConfigChange {
  return {
    change_category: opts.category,
    target_timescale: opts.timescale ?? null,
    target_tool_id: opts.tool_id ?? null,
    target_tool_type: opts.tool_type ?? null,
    target_param: opts.param,
    old_value: opts.old_value ?? null,
    new_value: opts.new_value ?? null,
    reason: opts.reason ?? "config mutation by PM agent",
  };
}

function diffScoring(
  old: Record<string, unknown>,
  new_: Record<string, unknown>,
): ConfigChange[] {
  const changes: ConfigChange[] = [];

  // thresholds
  for (const key of ["entry_threshold", "exit_threshold"]) {
    const oldVal = old[key];
    const newVal = new_[key];
    if (oldVal !== newVal) {
      changes.push(
        change({
          category: "threshold_adjusted",
          param: key,
          old_value: oldVal,
          new_value: newVal,
        }),
      );
    }
  }

  // aggregation method
  if (old.aggregation !== new_.aggregation) {
    changes.push(
      change({
        category: "scoring_changed",
        param: "aggregation",
        old_value: old.aggregation,
        new_value: new_.aggregation,
      }),
    );
  }

  // hard gate timescales
  const oldGates = (old.hard_gate_timescales as string[] | undefined) ?? [];
  const newGates = (new_.hard_gate_timescales as string[] | undefined) ?? [];
  const oldGatesSorted = [...oldGates].map(String).sort();
  const newGatesSorted = [...newGates].map(String).sort();
  if (JSON.stringify(oldGatesSorted) !== JSON.stringify(newGatesSorted)) {
    changes.push(
      change({
        category: "scoring_changed",
        param: "hard_gate_timescales",
        old_value: oldGates,
        new_value: newGates,
      }),
    );
  }

  // timescale weights
  const oldWeights =
    (old.timescale_weights as Record<string, number> | undefined) ?? {};
  const newWeights =
    (new_.timescale_weights as Record<string, number> | undefined) ?? {};
  const allTimescales = new Set([
    ...Object.keys(oldWeights),
    ...Object.keys(newWeights),
  ]);
  for (const ts of [...allTimescales].sort()) {
    const oldW = oldWeights[ts];
    const newW = newWeights[ts];
    if (oldW !== newW) {
      changes.push(
        change({
          category: "weight_adjusted",
          param: "timescale_weight",
          timescale: ts,
          old_value: oldW,
          new_value: newW,
        }),
      );
    }
  }

  return changes;
}

function diffSession(
  old: Record<string, unknown>,
  new_: Record<string, unknown>,
): ConfigChange[] {
  const changes: ConfigChange[] = [];
  const allKeys = new Set([...Object.keys(old), ...Object.keys(new_)]);
  for (const key of [...allKeys].sort()) {
    if (old[key] !== new_[key]) {
      changes.push(
        change({
          category: "session_rule_changed",
          param: key,
          old_value: old[key],
          new_value: new_[key],
        }),
      );
    }
  }
  return changes;
}

interface ToolConfig {
  instance_id: string;
  indicator_type?: string;
  action_type?: string;
  timescale?: string;
  enabled?: boolean;
  weight?: number;
  params?: Record<string, unknown>;
  [key: string]: unknown;
}

function getToolType(tool: ToolConfig, toolKind: string): string | undefined {
  return (
    (tool[`${toolKind}_type`] as string | undefined) ??
    tool.action_type ??
    tool.indicator_type
  );
}

function diffTools(
  oldTools: ToolConfig[],
  newTools: ToolConfig[],
  toolKind: string,
): ConfigChange[] {
  const changes: ConfigChange[] = [];

  const oldById = new Map<string, ToolConfig>();
  for (const t of oldTools) {
    if (t.instance_id) oldById.set(t.instance_id, t);
  }
  const newById = new Map<string, ToolConfig>();
  for (const t of newTools) {
    if (t.instance_id) newById.set(t.instance_id, t);
  }

  const allIds = new Set([...oldById.keys(), ...newById.keys()]);

  for (const toolId of [...allIds].sort()) {
    const oldTool = oldById.get(toolId);
    const newTool = newById.get(toolId);

    if (oldTool === undefined && newTool !== undefined) {
      // new tool added (enabled)
      changes.push(
        change({
          category: "tool_enabled",
          param: "enabled",
          tool_id: toolId,
          tool_type: getToolType(newTool, toolKind),
          timescale: newTool.timescale,
          old_value: null,
          new_value: true,
        }),
      );
      continue;
    }

    if (oldTool !== undefined && newTool === undefined) {
      // tool removed (disabled)
      changes.push(
        change({
          category: "tool_disabled",
          param: "enabled",
          tool_id: toolId,
          tool_type: getToolType(oldTool, toolKind),
          timescale: oldTool.timescale,
          old_value: true,
          new_value: null,
        }),
      );
      continue;
    }

    // both exist — check for changes
    if (oldTool === undefined || newTool === undefined) continue;

    const toolType = getToolType(newTool, toolKind);
    const timescale = newTool.timescale;

    // enabled/disabled toggle
    const oldEnabled = oldTool.enabled ?? true;
    const newEnabled = newTool.enabled ?? true;
    if (oldEnabled !== newEnabled) {
      const cat = newEnabled ? "tool_enabled" : "tool_disabled";
      changes.push(
        change({
          category: cat,
          param: "enabled",
          tool_id: toolId,
          tool_type: toolType,
          timescale,
          old_value: oldEnabled,
          new_value: newEnabled,
        }),
      );
    }

    // weight change (indicators only)
    if ("weight" in oldTool || "weight" in newTool) {
      const oldW = oldTool.weight;
      const newW = newTool.weight;
      if (oldW !== newW) {
        changes.push(
          change({
            category: "weight_adjusted",
            param: "weight",
            tool_id: toolId,
            tool_type: toolType,
            timescale,
            old_value: oldW,
            new_value: newW,
          }),
        );
      }
    }

    // params diff
    const oldParams = oldTool.params ?? {};
    const newParams = newTool.params ?? {};
    const allParamKeys = new Set([
      ...Object.keys(oldParams),
      ...Object.keys(newParams),
    ]);
    for (const pkey of [...allParamKeys].sort()) {
      if (oldParams[pkey] !== newParams[pkey]) {
        changes.push(
          change({
            category: "knob_tuned",
            param: pkey,
            tool_id: toolId,
            tool_type: toolType,
            timescale,
            old_value: oldParams[pkey],
            new_value: newParams[pkey],
          }),
        );
      }
    }
  }

  return changes;
}

export function computeConfigDiff(
  oldConfig: Record<string, unknown>,
  newConfig: Record<string, unknown>,
): ConfigChange[] {
  const changes: ConfigChange[] = [];

  // compare tickers
  const oldTickers = new Set(
    (oldConfig.tickers as string[] | undefined) ?? [],
  );
  const newTickers = new Set(
    (newConfig.tickers as string[] | undefined) ?? [],
  );
  if (JSON.stringify([...oldTickers].sort()) !== JSON.stringify([...newTickers].sort())) {
    changes.push(
      change({
        category: "session_rule_changed",
        param: "tickers",
        old_value: [...oldTickers].sort(),
        new_value: [...newTickers].sort(),
      }),
    );
  }

  // compare scoring
  changes.push(
    ...diffScoring(
      (oldConfig.scoring as Record<string, unknown>) ?? {},
      (newConfig.scoring as Record<string, unknown>) ?? {},
    ),
  );

  // compare session rules
  changes.push(
    ...diffSession(
      (oldConfig.session as Record<string, unknown>) ?? {},
      (newConfig.session as Record<string, unknown>) ?? {},
    ),
  );

  // compare indicators
  changes.push(
    ...diffTools(
      (oldConfig.indicators as ToolConfig[]) ?? [],
      (newConfig.indicators as ToolConfig[]) ?? [],
      "indicator",
    ),
  );

  // compare actions
  changes.push(
    ...diffTools(
      (oldConfig.actions as ToolConfig[]) ?? [],
      (newConfig.actions as ToolConfig[]) ?? [],
      "action",
    ),
  );

  return changes;
}
