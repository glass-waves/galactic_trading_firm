//! persists live engine state so an outside observer (dashboard, watchdog,
//! tuning agent) can see what the trader is doing between completed trades.
//!
//! two tables:
//! - `engine_state`: one row per ticker, upserted on every bar and on the
//!   heartbeat timer. acts as the process heartbeat.
//! - `entry_block_events`: append-only record of why entries were not taken
//!   (session gates, reject gates, near-miss windows), throttled by the caller.

use chrono::{DateTime, Utc};
use types::scoring::TimescaleScores;

use crate::trade_writer::direction_to_str;

/// snapshot of one ticker's open position for the state table.
#[derive(Debug, Clone)]
pub struct PositionSnapshot {
    pub direction: types::action::TradeDirection,
    pub entry_price: f64,
    pub size: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub hold_ms: i64,
    pub opened_at: DateTime<Utc>,
    pub entry_reason: Option<String>,
}

/// one row of `engine_state`.
#[derive(Debug, Clone)]
pub struct EngineStateRow {
    pub ticker: String,
    pub last_bar_at: Option<DateTime<Utc>>,
    pub last_price: Option<f64>,
    pub scores: TimescaleScores,
    pub position: Option<PositionSnapshot>,
    /// process-wide realized P&L today.
    pub daily_pnl: f64,
    /// this ticker's realized P&L today (engine-level).
    pub ticker_realized_pnl: f64,
    pub loss_breaker_active: bool,
    pub entry_blocked_by: Option<String>,
    pub near_miss: Option<String>,
    pub feed_stale: bool,
    pub config_version_id: i64,
    pub pending_config_version_id: Option<i64>,
    pub process_started_at: DateTime<Utc>,
    pub broker_mode: String,
}

pub async fn upsert_engine_state(
    pool: &sqlx::PgPool,
    row: &EngineStateRow,
) -> Result<(), sqlx::Error> {
    let pos = row.position.as_ref();
    sqlx::query(
        r#"
        INSERT INTO engine_state (
            ticker, updated_at, last_bar_at, last_price,
            composite, score_1min, score_5min, score_hourly,
            position_direction, position_entry_price, position_size,
            position_unrealized_pnl, position_unrealized_pnl_pct,
            position_hold_ms, position_opened_at, position_entry_reason,
            daily_pnl, ticker_realized_pnl, loss_breaker_active,
            entry_blocked_by, near_miss, feed_stale,
            config_version_id, pending_config_version_id,
            process_started_at, broker_mode
        ) VALUES (
            $1, now(), $2, $3,
            $4, $5, $6, $7,
            $8, $9, $10,
            $11, $12,
            $13, $14, $15,
            $16, $17, $18,
            $19, $20, $21,
            $22, $23,
            $24, $25
        )
        ON CONFLICT (ticker) DO UPDATE SET
            updated_at = now(),
            last_bar_at = EXCLUDED.last_bar_at,
            last_price = EXCLUDED.last_price,
            composite = EXCLUDED.composite,
            score_1min = EXCLUDED.score_1min,
            score_5min = EXCLUDED.score_5min,
            score_hourly = EXCLUDED.score_hourly,
            position_direction = EXCLUDED.position_direction,
            position_entry_price = EXCLUDED.position_entry_price,
            position_size = EXCLUDED.position_size,
            position_unrealized_pnl = EXCLUDED.position_unrealized_pnl,
            position_unrealized_pnl_pct = EXCLUDED.position_unrealized_pnl_pct,
            position_hold_ms = EXCLUDED.position_hold_ms,
            position_opened_at = EXCLUDED.position_opened_at,
            position_entry_reason = EXCLUDED.position_entry_reason,
            daily_pnl = EXCLUDED.daily_pnl,
            ticker_realized_pnl = EXCLUDED.ticker_realized_pnl,
            loss_breaker_active = EXCLUDED.loss_breaker_active,
            entry_blocked_by = EXCLUDED.entry_blocked_by,
            near_miss = EXCLUDED.near_miss,
            feed_stale = EXCLUDED.feed_stale,
            config_version_id = EXCLUDED.config_version_id,
            pending_config_version_id = EXCLUDED.pending_config_version_id,
            process_started_at = EXCLUDED.process_started_at,
            broker_mode = EXCLUDED.broker_mode
        "#,
    )
    .bind(&row.ticker)
    .bind(row.last_bar_at)
    .bind(row.last_price)
    .bind(row.scores.composite)
    .bind(row.scores.one_minute)
    .bind(row.scores.five_minute)
    .bind(row.scores.one_hour)
    .bind(pos.map(|p| direction_to_str(&p.direction)))
    .bind(pos.map(|p| p.entry_price))
    .bind(pos.map(|p| p.size))
    .bind(pos.map(|p| p.unrealized_pnl))
    .bind(pos.map(|p| p.unrealized_pnl_pct))
    .bind(pos.map(|p| p.hold_ms))
    .bind(pos.map(|p| p.opened_at))
    .bind(pos.and_then(|p| p.entry_reason.clone()))
    .bind(row.daily_pnl)
    .bind(row.ticker_realized_pnl)
    .bind(row.loss_breaker_active)
    .bind(&row.entry_blocked_by)
    .bind(&row.near_miss)
    .bind(row.feed_stale)
    .bind(row.config_version_id)
    .bind(row.pending_config_version_id)
    .bind(row.process_started_at)
    .bind(&row.broker_mode)
    .execute(pool)
    .await?;
    Ok(())
}

/// remove state rows for tickers no longer traded (after a config change).
pub async fn delete_engine_state(pool: &sqlx::PgPool, ticker: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM engine_state WHERE ticker = $1")
        .bind(ticker)
        .execute(pool)
        .await?;
    Ok(())
}

/// kind of entry-block event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// a session gate or reject gate prevented entry evaluation.
    Gate,
    /// windows were evaluated, at least one met its composite floor, none fired.
    NearMiss,
}

impl BlockKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BlockKind::Gate => "gate",
            BlockKind::NearMiss => "near_miss",
        }
    }
}

/// one `entry_block_events` row.
#[derive(Debug, Clone)]
pub struct BlockEvent<'a> {
    pub ticker: &'a str,
    pub ts: DateTime<Utc>,
    pub kind: BlockKind,
    pub reason: &'a str,
    pub scores: &'a TimescaleScores,
    pub last_price: f64,
    pub config_version_id: i64,
}

pub async fn write_entry_block_event(
    pool: &sqlx::PgPool,
    ev: &BlockEvent<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO entry_block_events (
            ts, ticker, kind, reason, composite, score_1min, score_5min, score_hourly,
            last_price, config_version_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(ev.ts)
    .bind(ev.ticker)
    .bind(ev.kind.as_str())
    .bind(ev.reason)
    .bind(ev.scores.composite)
    .bind(ev.scores.one_minute)
    .bind(ev.scores.five_minute)
    .bind(ev.scores.one_hour)
    .bind(ev.last_price)
    .bind(ev.config_version_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_kind_strings() {
        assert_eq!(BlockKind::Gate.as_str(), "gate");
        assert_eq!(BlockKind::NearMiss.as_str(), "near_miss");
    }
}
