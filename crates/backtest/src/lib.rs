pub mod replay;
pub mod report;

pub use report::{
    BacktestMetrics, BacktestResult, ConfigComparison, EquityPoint, ExitReasonBreakdown,
    compare_configs, comparison_summary, compute_metrics, summary, to_json, trades_to_csv,
};
pub use replay::{BacktestConfig, BacktestData, run_backtest, load_candles_from_csv};
