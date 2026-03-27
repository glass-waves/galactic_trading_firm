pub mod scoring;
pub mod position;
pub mod tick_loop;

pub use scoring::{compute_composite, compute_composite_with_indicators};
pub use position::{PositionManager, TradeRecord};
pub use tick_loop::{TradingEngine, WindowExitOverrides};
