"""parameterized queries for agent tools.

all queries use parameterized sql — never arbitrary sql.
agents are read-only for trade data, write-only for memos and config proposals.
"""

from __future__ import annotations

from datetime import date, datetime

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncEngine

from agents.models import ChangelogEntry, DailyPerformance, TradeRecord


async def get_recent_trades(
    engine: AsyncEngine,
    ticker: str | None = None,
    limit: int = 20,
    since: datetime | None = None,
) -> list[TradeRecord]:
    """fetch recent completed trades, optionally filtered by ticker and time."""
    query = """
        SELECT id, ticker, direction, entry_price, exit_price, position_size,
               pnl_dollars, pnl_percent, hold_duration_ms, exit_reason,
               entry_fill_at, exit_fill_at, entry_score_composite, config_version_id
        FROM trades
        WHERE 1=1
    """
    params: dict = {"limit": limit}

    if ticker is not None:
        query += " AND ticker = :ticker"
        params["ticker"] = ticker

    if since is not None:
        query += " AND exit_fill_at >= :since"
        params["since"] = since

    query += " ORDER BY exit_fill_at DESC LIMIT :limit"

    async with engine.connect() as conn:
        result = await conn.execute(text(query), params)
        rows = result.mappings().all()
        return [TradeRecord(**dict(row)) for row in rows]


async def get_daily_performance(
    engine: AsyncEngine,
    trading_day: date | None = None,
    ticker: str | None = None,
) -> list[DailyPerformance]:
    """fetch daily performance summary. defaults to today if no date given."""
    query = """
        SELECT
            date_trunc('day', entry_fill_at)::date AS trading_day,
            ticker,
            COUNT(*) AS total_trades,
            SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END) AS winning_trades,
            ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
            ROUND(SUM(pnl_dollars)::numeric, 2) AS total_pnl,
            ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
                / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
            ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms
        FROM trades
        WHERE 1=1
    """
    params: dict = {}

    if trading_day is not None:
        query += " AND date_trunc('day', entry_fill_at)::date = :trading_day"
        params["trading_day"] = trading_day

    if ticker is not None:
        query += " AND ticker = :ticker"
        params["ticker"] = ticker

    query += " GROUP BY trading_day, ticker ORDER BY trading_day DESC"

    async with engine.connect() as conn:
        result = await conn.execute(text(query), params)
        rows = result.mappings().all()
        return [DailyPerformance(**dict(row)) for row in rows]


async def get_config_changelog(
    engine: AsyncEngine,
    since: datetime | None = None,
    limit: int = 50,
) -> list[ChangelogEntry]:
    """fetch config changelog entries, optionally filtered by time."""
    query = """
        SELECT id, config_version_id, created_at, changed_by::text,
               change_category::text, target_timescale::text,
               target_tool_id, target_tool_type, target_param,
               old_value, new_value, reason
        FROM config_changelog
        WHERE 1=1
    """
    params: dict = {"limit": limit}

    if since is not None:
        query += " AND created_at >= :since"
        params["since"] = since

    query += " ORDER BY created_at DESC LIMIT :limit"

    async with engine.connect() as conn:
        result = await conn.execute(text(query), params)
        rows = result.mappings().all()
        return [ChangelogEntry(**dict(row)) for row in rows]


async def get_performance_by_exit_reason(
    engine: AsyncEngine,
    since: datetime | None = None,
) -> list[dict]:
    """fetch performance breakdown by exit reason."""
    query = """
        SELECT
            exit_reason::text,
            COUNT(*) AS total_trades,
            ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
            ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
                / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
            ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms
        FROM trades
        WHERE 1=1
    """
    params: dict = {}

    if since is not None:
        query += " AND exit_fill_at >= :since"
        params["since"] = since

    query += " GROUP BY exit_reason"

    async with engine.connect() as conn:
        result = await conn.execute(text(query), params)
        return [dict(row) for row in result.mappings().all()]


async def get_checkin_memos_since_last_pm(engine: AsyncEngine) -> list[dict]:
    """fetch check-in observation memos since the last completed full PM cycle."""
    query = text("""
        SELECT am.id, am.agent::text, am.evolution_cycle_id, am.memo_type::text,
               am.confidence_score, am.volatility_regime, am.directional_bias,
               am.signal_quality, am.flags, am.reasoning, am.created_at,
               am.trades_reviewed, am.period_win_rate, am.period_pnl
        FROM agent_memos am
        WHERE am.memo_type = 'observation'
          AND am.created_at > COALESCE(
              (SELECT MAX(ec.completed_at)
               FROM evolution_cycles ec
               WHERE ec.cycle_type = 'full_pm' AND ec.completed_at IS NOT NULL),
              '1970-01-01'::timestamptz
          )
        ORDER BY am.created_at DESC
    """)

    async with engine.connect() as conn:
        result = await conn.execute(query)
        return [dict(row) for row in result.mappings().all()]


async def get_recommendation_memos(
    engine: AsyncEngine,
    cycle_id: int,
) -> list[dict]:
    """fetch recommendation memos for a specific evolution cycle."""
    query = text("""
        SELECT am.id, am.agent::text, am.evolution_cycle_id, am.memo_type::text,
               am.confidence_score, am.volatility_regime, am.directional_bias,
               am.signal_quality, am.flags, am.reasoning, am.created_at,
               am.trades_reviewed, am.period_win_rate, am.period_pnl
        FROM agent_memos am
        WHERE am.memo_type = 'recommendation'
          AND am.evolution_cycle_id = :cycle_id
        ORDER BY am.created_at ASC
    """)

    async with engine.connect() as conn:
        result = await conn.execute(query, {"cycle_id": cycle_id})
        return [dict(row) for row in result.mappings().all()]
