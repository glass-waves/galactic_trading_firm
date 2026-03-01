pub mod atr_trailing_stop;
pub mod fixed_pct_stop;
pub mod session_close;
pub mod max_hold_timeout;

pub use atr_trailing_stop::{atr_trailing_stop_factory, AtrTrailingStop};
pub use fixed_pct_stop::{fixed_pct_stop_factory, FixedPctStop};
pub use session_close::{session_close_factory, SessionCloseExit};
pub use max_hold_timeout::{max_hold_timeout_factory, MaxHoldTimeout};
