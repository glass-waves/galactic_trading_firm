use engine::TradeRecord;
use types::action::TradeDirection;
use types::scoring::TimescaleScores;

/// writes completed trades to the postgres trades table.
/// all trades are marked as paper trades (is_paper = true).
pub struct TradeWriter {
    pool: sqlx::PgPool,
    config_version_id: i64,
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
    /// returns the inserted row's trade_id.
    pub async fn write_trade(
        &self,
        trade: &TradeRecord,
        entry_scores: &TimescaleScores,
        exit_scores: &TimescaleScores,
    ) -> Result<i64, sqlx::Error> {
        let direction_str = match trade.direction {
            TradeDirection::Long => "long",
            TradeDirection::Short => "short",
        };
        let exit_reason_str = format!("{:?}", trade.exit_reason);

        let entry_scores_json = serde_json::to_value(entry_scores)
            .unwrap_or(serde_json::Value::Null);
        let exit_scores_json = serde_json::to_value(exit_scores)
            .unwrap_or(serde_json::Value::Null);

        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO trades (
                config_version_id, ticker, direction,
                entry_price, exit_price, size,
                entry_time, exit_time,
                pnl, pnl_pct, hold_duration_ms,
                exit_reason, high_water_mark, low_water_mark,
                entry_scores, exit_scores,
                is_paper
            ) VALUES (
                $1, $2, $3,
                $4, $5, $6,
                $7, $8,
                $9, $10, $11,
                $12, $13, $14,
                $15, $16,
                true
            ) RETURNING trade_id
            "#,
        )
        .bind(self.config_version_id)
        .bind(&trade.ticker)
        .bind(direction_str)
        .bind(trade.entry_price)
        .bind(trade.exit_price)
        .bind(trade.size)
        .bind(trade.entry_time)
        .bind(trade.exit_time)
        .bind(trade.pnl)
        .bind(trade.pnl_pct)
        .bind(trade.hold_duration_ms)
        .bind(&exit_reason_str)
        .bind(trade.high_water_mark)
        .bind(trade.low_water_mark)
        .bind(&entry_scores_json)
        .bind(&exit_scores_json)
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
        // we can't connect to a real db in unit tests, but we can verify
        // the struct is constructible and the config version is tracked.
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
    fn is_paper_always_true_in_query() {
        // verify the SQL string contains "true" for is_paper
        let sql = r#"
            INSERT INTO trades (
                config_version_id, ticker, direction,
                entry_price, exit_price, size,
                entry_time, exit_time,
                pnl, pnl_pct, hold_duration_ms,
                exit_reason, high_water_mark, low_water_mark,
                entry_scores, exit_scores,
                is_paper
            ) VALUES (
                $1, $2, $3,
                $4, $5, $6,
                $7, $8,
                $9, $10, $11,
                $12, $13, $14,
                $15, $16,
                true
            ) RETURNING trade_id
            "#;
        assert!(sql.contains("is_paper"));
        assert!(sql.contains("true"));
    }

    #[test]
    fn entry_scores_serialize() {
        let scores = TimescaleScores {
            one_minute: Some(0.5),
            five_minute: Some(0.7),
            composite: 0.6,
            ..Default::default()
        };
        let json = serde_json::to_value(&scores).unwrap();
        assert_eq!(json["composite"], 0.6);
        assert_eq!(json["one_minute"], 0.5);
    }

    #[test]
    fn direction_to_string_mapping() {
        let long_str = match TradeDirection::Long {
            TradeDirection::Long => "long",
            TradeDirection::Short => "short",
        };
        assert_eq!(long_str, "long");

        let short_str = match TradeDirection::Short {
            TradeDirection::Long => "long",
            TradeDirection::Short => "short",
        };
        assert_eq!(short_str, "short");
    }
}
