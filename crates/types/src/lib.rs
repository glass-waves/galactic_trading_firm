pub mod market;
pub mod indicator;
pub mod action;
pub mod scoring;
pub mod config;
pub mod registry;
pub mod proposal;
pub mod adapter;
pub mod tick_result;
pub mod test_fixtures;

pub use market::{Candle, MarketState, Timescale};
pub use indicator::{Indicator, IndicatorConfig, IndicatorOutput};
pub use action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position, TradeDirection};
pub use scoring::{AggregationMethod, ScoringConfig, TimescaleScores};
pub use config::{SessionConfig, StrategyConfig};
pub use registry::{ActionRegistry, IndicatorRegistry, ToolBelt};
pub use tick_result::{TickEvent, TickResult};
