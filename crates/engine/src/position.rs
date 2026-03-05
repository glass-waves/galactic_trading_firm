use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use types::action::{ExitReason, Position, TradeDirection};

/// completed trade record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub ticker: String,
    pub direction: TradeDirection,
    pub entry_price: f64,
    pub exit_price: f64,
    pub size: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_time: DateTime<Utc>,
    pub pnl: f64,
    pub pnl_pct: f64,
    pub hold_duration_ms: i64,
    pub exit_reason: ExitReason,
    pub high_water_mark: f64,
    pub low_water_mark: f64,
}

/// manages open positions and lifecycle transitions.
pub struct PositionManager {
    position: Option<Position>,
}

impl PositionManager {
    pub fn new() -> Self {
        Self { position: None }
    }

    pub fn current_position(&self) -> Option<&Position> {
        self.position.as_ref()
    }

    pub fn has_position(&self) -> bool {
        self.position.is_some()
    }

    pub fn open_position(
        &mut self,
        ticker: String,
        direction: TradeDirection,
        entry_price: f64,
        size: f64,
        entry_time: DateTime<Utc>,
    ) -> Result<(), String> {
        if self.position.is_some() {
            return Err("cannot open position: one already exists".to_string());
        }

        self.position = Some(Position {
            ticker,
            direction,
            entry_price,
            current_price: entry_price,
            size,
            entry_time,
            unrealized_pnl: 0.0,
            unrealized_pnl_pct: 0.0,
            high_water_mark: entry_price,
            low_water_mark: entry_price,
            hold_duration_ms: 0,
        });

        Ok(())
    }

    pub fn update_on_tick(&mut self, current_price: f64, current_time: DateTime<Utc>) {
        if let Some(ref mut pos) = self.position {
            pos.current_price = current_price;

            if current_price > pos.high_water_mark {
                pos.high_water_mark = current_price;
            }
            if current_price < pos.low_water_mark {
                pos.low_water_mark = current_price;
            }

            match pos.direction {
                TradeDirection::Long => {
                    pos.unrealized_pnl = (current_price - pos.entry_price) * pos.size;
                    pos.unrealized_pnl_pct = (current_price - pos.entry_price) / pos.entry_price;
                }
                TradeDirection::Short => {
                    pos.unrealized_pnl = (pos.entry_price - current_price) * pos.size;
                    pos.unrealized_pnl_pct = (pos.entry_price - current_price) / pos.entry_price;
                }
            }

            pos.hold_duration_ms = (current_time - pos.entry_time).num_milliseconds();
        }
    }

    pub fn close_position(
        &mut self,
        exit_price: f64,
        exit_time: DateTime<Utc>,
        exit_reason: ExitReason,
    ) -> Option<TradeRecord> {
        let pos = self.position.take()?;

        let pnl = match pos.direction {
            TradeDirection::Long => (exit_price - pos.entry_price) * pos.size,
            TradeDirection::Short => (pos.entry_price - exit_price) * pos.size,
        };
        let pnl_pct = match pos.direction {
            TradeDirection::Long => (exit_price - pos.entry_price) / pos.entry_price,
            TradeDirection::Short => (pos.entry_price - exit_price) / pos.entry_price,
        };

        Some(TradeRecord {
            ticker: pos.ticker,
            direction: pos.direction,
            entry_price: pos.entry_price,
            exit_price,
            size: pos.size,
            entry_time: pos.entry_time,
            exit_time,
            pnl,
            pnl_pct,
            hold_duration_ms: (exit_time - pos.entry_time).num_milliseconds(),
            exit_reason,
            high_water_mark: pos.high_water_mark,
            low_water_mark: pos.low_water_mark,
        })
    }

    /// remove the current position without creating a trade record.
    /// used by backtest replay to undo same-bar fills for next-bar execution.
    pub fn cancel_position(&mut self) -> Option<Position> {
        self.position.take()
    }

    /// restore a previously cancelled position.
    pub fn restore_position(&mut self, position: Position) {
        self.position = Some(position);
    }
}

impl Default for PositionManager {
    fn default() -> Self {
        Self::new()
    }
}
