use engine::TradeRecord;
use types::action::{ExitReason, TradeDirection};
use types::scoring::TimescaleScores;

/// writes completed trades to the postgres trades table.
/// all trades are marked as paper trades (is_paper = true).
pub struct TradeWriter {
    pool: sqlx::PgPool,
    config_version_id: i64,
}

fn direction_to_str(d: &TradeDirection) -> &'static str {
    match d {
        TradeDirection::Long => "long",
        TradeDirection::Short => "short",
    }
}

fn exit_reason_to_str(r: &ExitReason) -> &'static str {
    match r {
        ExitReason::TrailingStop => "trailing_stop",
        ExitReason::HardStop => "hard_stop",
        ExitReason::TakeProfit => "take_profit",
        ExitReason::MaxHoldTimeout => "max_hold_timeout",
        ExitReason::SessionClose => "session_close",
        ExitReason::FilterAlignment => "filter_alignment",
        ExitReason::ManualOverride => "manual_override",
        ExitReason::ConfigChange => "config_change",
    }
}

impl TradeWriter {
    pub fn new(pool: sqlx::PgPool, config_version_id: i64) -> Self {
        Self {
            pool,
            config_version_id,
        }
    }

    pub fn set_config_version(&mut self, version_id: i64) {
        self.config_version_id = version_id;
    }

    /// write a completed trade with entry and exit scores to the trades table.
    /// returns the inserted row's id.
    pub async fn write_trade(
        &self,
        trade: &TradeRecord,
        entry_scores: &TimescaleScores,
        exit_scores: &TimescaleScores,
    ) -> Result<i64, sqlx::Error> {
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
                is_paper
            ) VALUES (
                $1, $2::trade_direction, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13::exit_reason,
                $14,
                $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26,
                true
            ) RETURNING id
            "#,
        )
        .bind(&trade.ticker)
        .bind(direction_to_str(&trade.direction))
        .bind(trade.entry_price)
        .bind(trade.exit_price)
        .bind(trade.size)
        .bind(trade.entry_time)  // entry_signal_at
        .bind(trade.entry_time)  // entry_fill_at (same for paper trading)
        .bind(trade.exit_time)   // exit_signal_at
        .bind(trade.exit_time)   // exit_fill_at (same for paper trading)
        .bind(trade.pnl)
        .bind(trade.pnl_pct)
        .bind(trade.hold_duration_ms)
        .bind(exit_reason_to_str(&trade.exit_reason))
        .bind(self.config_version_id)
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
        let writer = TradeWriter::new(pool, 42);
        assert_eq!(writer.config_version_id, 42);
    }

    #[tokio::test]
    async fn set_config_version() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let mut writer = TradeWriter::new(pool, 1);
        writer.set_config_version(99);
        assert_eq!(writer.config_version_id, 99);
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
    }
}
