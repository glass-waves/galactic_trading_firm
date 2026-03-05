use std::fmt;

use engine::position::TradeRecord;
use types::StrategyConfig;
use types::action::{ExitReason, TradeDirection};
use types::scoring::TimescaleScores;

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    DatabaseError(sqlx::Error),
    DeserializationError(serde_json::Error),
    ValidationError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "no promoted config found"),
            ConfigError::DatabaseError(e) => write!(f, "database error: {e}"),
            ConfigError::DeserializationError(e) => write!(f, "config deserialization error: {e}"),
            ConfigError::ValidationError(msg) => write!(f, "config validation error: {msg}"),
        }
    }
}

impl From<sqlx::Error> for ConfigError {
    fn from(e: sqlx::Error) -> Self {
        ConfigError::DatabaseError(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::DeserializationError(e)
    }
}

pub async fn load_promoted_config(pool: &sqlx::PgPool) -> Result<StrategyConfig, ConfigError> {
    let (config, _id) = load_promoted_config_with_id(pool).await?;
    Ok(config)
}

pub async fn load_promoted_config_with_id(
    pool: &sqlx::PgPool,
) -> Result<(StrategyConfig, i64), ConfigError> {
    let row: Option<(i64, serde_json::Value)> = sqlx::query_as(
        "SELECT id, config_blob FROM config_versions WHERE status = 'promoted' ORDER BY promoted_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let (id, config_blob) = row.ok_or(ConfigError::NotFound)?;
    let config: StrategyConfig = serde_json::from_value(config_blob)?;

    if config.indicators.is_empty() {
        return Err(ConfigError::ValidationError(
            "config must have at least one indicator".to_string(),
        ));
    }
    if config.actions.is_empty() {
        return Err(ConfigError::ValidationError(
            "config must have at least one action".to_string(),
        ));
    }

    Ok((config, id))
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
        ExitReason::ScoreExit => "score_exit",
        ExitReason::DailyLossLimit => "daily_loss_limit",
    }
}

pub async fn write_backtest_trades(
    pool: &sqlx::PgPool,
    config_version_id: i64,
    trades: &[TradeRecord],
    trade_scores: &[(TimescaleScores, TimescaleScores)],
) -> Result<usize, sqlx::Error> {
    let mut count = 0usize;
    for (i, trade) in trades.iter().enumerate() {
        let direction_str = direction_to_str(&trade.direction);
        let exit_reason_str = exit_reason_to_str(&trade.exit_reason);

        let (entry_scores, exit_scores) = trade_scores
            .get(i)
            .cloned()
            .unwrap_or_default();

        sqlx::query(
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
                commission, is_paper
            ) VALUES (
                $1, $2::trade_direction, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13::exit_reason,
                $14,
                $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26,
                $27, $28
            )
            "#,
        )
        .bind(&trade.ticker)
        .bind(direction_str)
        .bind(trade.entry_price)
        .bind(trade.exit_price)
        .bind(trade.size)
        .bind(trade.entry_time)
        .bind(trade.entry_time) // entry_fill_at = entry_signal_at for backtest
        .bind(trade.exit_time)
        .bind(trade.exit_time) // exit_fill_at = exit_signal_at for backtest
        .bind(trade.pnl)
        .bind(trade.pnl_pct)
        .bind(trade.hold_duration_ms)
        .bind(exit_reason_str)
        .bind(config_version_id)
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
        .bind(0.0_f64) // commission
        .bind(true)     // is_paper
        .execute(pool)
        .await?;

        count += 1;
    }
    Ok(count)
}
