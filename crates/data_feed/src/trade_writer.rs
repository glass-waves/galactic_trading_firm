use engine::TradeRecord;
use types::action::{ExitReason, TradeDirection};

use crate::live_session::TradeWithScores;

/// writes completed trades to the postgres trades table.
/// all trades are marked as paper trades (is_paper = true); `source`
/// distinguishes live paper trading from demo/synthetic runs.
pub struct TradeWriter {
    pool: sqlx::PgPool,
    source: String,
}

pub fn direction_to_str(d: &TradeDirection) -> &'static str {
    match d {
        TradeDirection::Long => "long",
        TradeDirection::Short => "short",
    }
}

pub fn exit_reason_to_str(r: &ExitReason) -> &'static str {
    match r {
        ExitReason::TrailingStop => "trailing_stop",
        ExitReason::HardStop => "hard_stop",
        ExitReason::TakeProfit => "take_profit",
        ExitReason::MaxHoldTimeout => "max_hold_timeout",
        ExitReason::SessionClose => "session_close",
        ExitReason::FilterAlignment => "filter_alignment",
        ExitReason::ManualOverride => "manual_override",
        ExitReason::ConfigChange => "config_change",
        ExitReason::ScoreExit => "score_exit",
        ExitReason::DailyLossLimit => "daily_loss_limit",
    }
}

impl TradeWriter {
    /// `source` is written to `trades.source` ("paper" or "demo").
    pub fn new(pool: sqlx::PgPool, source: &str) -> Self {
        Self {
            pool,
            source: source.to_string(),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// write a completed trade with entry and exit scores to the trades table.
    /// the trade is attributed to the config version the producing engine was
    /// built from. returns the inserted row's id.
    pub async fn write_trade(
        &self,
        tws: &TradeWithScores,
        broker_exit_price: Option<f64>,
    ) -> Result<i64, sqlx::Error> {
        let trade: &TradeRecord = &tws.trade;
        let entry_scores = &tws.entry_scores;
        let exit_scores = &tws.exit_scores;
        let entry_reason = if tws.entry_reason.is_empty() {
            None
        } else {
            Some(tws.entry_reason.as_str())
        };

        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO trades (
                ticker, direction, entry_price, exit_price, position_size,
                entry_signal_at, entry_fill_at, exit_signal_at, exit_fill_at,
                pnl_dollars, pnl_percent, hold_duration_ms, exit_reason,
                config_version_id,
                entry_score_1min, entry_score_5min, entry_score_hourly,
                entry_score_daily, entry_score_monthly, entry_score_composite,
                exit_score_1min, exit_score_5min, exit_score_hourly,
                exit_score_daily, exit_score_monthly, exit_score_composite,
                is_paper, entry_reason, source, broker_entry_price, broker_exit_price
            ) VALUES (
                $1, $2::trade_direction, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13::exit_reason,
                $14,
                $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26,
                true, $27, $28, $29, $30
            ) RETURNING id
            "#,
        )
        .bind(&trade.ticker)
        .bind(direction_to_str(&trade.direction))
        .bind(trade.entry_price)
        .bind(trade.exit_price)
        .bind(trade.size)
        .bind(trade.entry_time) // entry_signal_at
        .bind(trade.entry_time) // entry_fill_at (same bar for paper trading)
        .bind(trade.exit_time) // exit_signal_at
        .bind(trade.exit_time) // exit_fill_at
        .bind(trade.pnl)
        .bind(trade.pnl_pct)
        .bind(trade.hold_duration_ms)
        .bind(exit_reason_to_str(&trade.exit_reason))
        .bind(tws.config_version_id)
        .bind(entry_scores.one_minute)
        .bind(entry_scores.five_minute)
        .bind(entry_scores.one_hour)
        .bind(entry_scores.one_day)
        .bind(entry_scores.one_month)
        .bind(entry_scores.composite)
        .bind(exit_scores.one_minute)
        .bind(exit_scores.five_minute)
        .bind(exit_scores.one_hour)
        .bind(exit_scores.one_day)
        .bind(exit_scores.one_month)
        .bind(exit_scores.composite)
        .bind(entry_reason)
        .bind(&self.source)
        .bind(tws.broker_entry_price)
        .bind(broker_exit_price)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn trade_writer_construction() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let writer = TradeWriter::new(pool, "paper");
        assert_eq!(writer.source(), "paper");
    }

    #[test]
    fn direction_str_mapping() {
        assert_eq!(direction_to_str(&TradeDirection::Long), "long");
        assert_eq!(direction_to_str(&TradeDirection::Short), "short");
    }

    #[test]
    fn exit_reason_str_mapping() {
        assert_eq!(exit_reason_to_str(&ExitReason::TrailingStop), "trailing_stop");
        assert_eq!(exit_reason_to_str(&ExitReason::HardStop), "hard_stop");
        assert_eq!(exit_reason_to_str(&ExitReason::TakeProfit), "take_profit");
        assert_eq!(exit_reason_to_str(&ExitReason::MaxHoldTimeout), "max_hold_timeout");
        assert_eq!(exit_reason_to_str(&ExitReason::SessionClose), "session_close");
        assert_eq!(exit_reason_to_str(&ExitReason::FilterAlignment), "filter_alignment");
        assert_eq!(exit_reason_to_str(&ExitReason::ManualOverride), "manual_override");
        assert_eq!(exit_reason_to_str(&ExitReason::ConfigChange), "config_change");
        assert_eq!(exit_reason_to_str(&ExitReason::ScoreExit), "score_exit");
        assert_eq!(exit_reason_to_str(&ExitReason::DailyLossLimit), "daily_loss_limit");
    }
}
