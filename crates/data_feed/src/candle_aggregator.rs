//! the candle aggregator lives in the engine crate so the backtest replay and
//! the live feed build multi-timescale windows with the same code.
pub use engine::candle_aggregator::*;
