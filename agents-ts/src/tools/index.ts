export { computeConfigDiff, CHANGE_CATEGORIES } from "./config-diff.js";
export type { ConfigChange } from "./config-diff.js";
export {
  runBacktestValidation,
  validateBacktestResult,
  DEFAULT_DATA_DIR,
} from "./backtest-runner.js";
export {
  getRecentTrades,
  getDailyPerformance,
  getConfigChangelog,
  getPerformanceByExitReason,
  getCheckinMemosSinceLastPm,
  getAnalysisMemos,
  getActiveBeliefs,
  writeBelief,
} from "./sql-queries.js";
export { writeMemo } from "./memo-writer.js";
export {
  getCurrentConfig,
  getConfigVersion,
  proposeConfig,
  updateConfigStatus,
  writeChangelogEntries,
} from "./config-ops.js";
export { readLogs } from "./log-reader.js";
