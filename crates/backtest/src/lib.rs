pub mod alpaca_loader;
pub mod config_loader;
pub mod replay;
pub mod report;
pub mod splits;

pub use report::{
    BacktestMetrics, BacktestResult, ConfigComparison, EquityPoint, ExitReasonBreakdown,
    compare_configs, comparison_summary, compute_metrics, summary, to_json, trades_to_csv,
};
pub use replay::{
    BacktestConfig, BacktestCostConfig, BacktestData, TickEquityPoint,
    run_backtest, load_candles_from_csv,
};
pub use splits::{SplitRecord, load_splits_csv, unadjust_to_date};
