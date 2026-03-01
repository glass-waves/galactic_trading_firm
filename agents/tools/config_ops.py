"""config version read/write operations.

provides tools for agents to:
- read the current promoted config
- propose new config versions (creates config_versions row with status='proposed')
- update config version status (backtesting → promoted/rejected)
- write changelog entries from computed diffs
- read config changelog for recent changes
"""

from __future__ import annotations

import json
from typing import Any

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncEngine


async def get_current_config(engine: AsyncEngine) -> dict[str, Any] | None:
    """read the currently active (promoted) config blob.

    returns the config as a dict, or None if no promoted config exists.
    """
    query = text("""
        SELECT id, config_blob, promoted_at, created_by::text, mutation_reason
        FROM config_versions
        WHERE status = 'promoted'
        ORDER BY promoted_at DESC
        LIMIT 1
    """)

    async with engine.connect() as conn:
        result = await conn.execute(query)
        row = result.mappings().first()
        if row is None:
            return None
        return {
            "config_version_id": row["id"],
            "config": row["config_blob"],
            "promoted_at": str(row["promoted_at"]),
            "created_by": row["created_by"],
            "mutation_reason": row["mutation_reason"],
        }


async def get_config_version(engine: AsyncEngine, version_id: int) -> dict[str, Any] | None:
    """read a specific config version by id."""
    query = text("""
        SELECT id, config_blob, status::text, created_at, promoted_at,
               created_by::text, mutation_reason,
               backtest_sharpe, backtest_win_rate, backtest_total_trades
        FROM config_versions
        WHERE id = :version_id
    """)

    async with engine.connect() as conn:
        result = await conn.execute(query, {"version_id": version_id})
        row = result.mappings().first()
        if row is None:
            return None
        return dict(row)


async def propose_config(
    engine: AsyncEngine,
    config_blob: dict[str, Any],
    parent_version_id: int | None,
    mutation_reason: str,
    created_by: str = "agent_pm",
) -> int:
    """insert a new config_versions row with status='proposed'. returns version_id."""
    query = text("""
        INSERT INTO config_versions (
            config_blob, status, parent_version_id,
            mutation_reason, created_by
        ) VALUES (
            :config_blob::jsonb, 'proposed'::config_status,
            :parent_version_id, :mutation_reason, :created_by::agent_type
        )
        RETURNING id
    """)

    async with engine.begin() as conn:
        result = await conn.execute(query, {
            "config_blob": json.dumps(config_blob),
            "parent_version_id": parent_version_id,
            "mutation_reason": mutation_reason,
            "created_by": created_by,
        })
        row = result.fetchone()
        return row[0]


async def update_config_status(
    engine: AsyncEngine,
    version_id: int,
    new_status: str,
    backtest_results: dict[str, Any] | None = None,
) -> None:
    """update config version status. if promoting, sets promoted_at and supersedes old promoted."""
    async with engine.begin() as conn:
        if new_status == "promoted":
            # supersede any currently promoted config
            await conn.execute(text("""
                UPDATE config_versions
                SET status = 'superseded'::config_status
                WHERE status = 'promoted'
            """))

            params: dict[str, Any] = {
                "version_id": version_id,
                "new_status": new_status,
            }

            if backtest_results:
                await conn.execute(text("""
                    UPDATE config_versions SET
                        status = :new_status::config_status,
                        promoted_at = now(),
                        backtest_sharpe = :sharpe,
                        backtest_win_rate = :win_rate,
                        backtest_total_trades = :total_trades
                    WHERE id = :version_id
                """), {
                    **params,
                    "sharpe": backtest_results.get("sharpe_ratio"),
                    "win_rate": backtest_results.get("win_rate"),
                    "total_trades": backtest_results.get("total_trades"),
                })
            else:
                await conn.execute(text("""
                    UPDATE config_versions SET
                        status = :new_status::config_status,
                        promoted_at = now()
                    WHERE id = :version_id
                """), params)
        else:
            await conn.execute(text("""
                UPDATE config_versions SET
                    status = :new_status::config_status
                WHERE id = :version_id
            """), {"version_id": version_id, "new_status": new_status})


async def write_changelog_entries(
    engine: AsyncEngine,
    version_id: int,
    changes: list[dict[str, Any]],
    changed_by: str = "agent_pm",
    source_memo_id: int | None = None,
) -> list[int]:
    """batch insert config_changelog rows from computed diff. returns list of ids."""
    if not changes:
        return []

    query = text("""
        INSERT INTO config_changelog (
            config_version_id, changed_by,
            change_category, target_timescale,
            target_tool_id, target_tool_type, target_param,
            old_value, new_value, reason, source_memo_id
        ) VALUES (
            :config_version_id, :changed_by::agent_type,
            :change_category::change_category, :target_timescale,
            :target_tool_id, :target_tool_type, :target_param,
            :old_value, :new_value, :reason, :source_memo_id
        )
        RETURNING id
    """)

    ids: list[int] = []
    async with engine.begin() as conn:
        for change in changes:
            result = await conn.execute(query, {
                "config_version_id": version_id,
                "changed_by": changed_by,
                "change_category": change["change_category"],
                "target_timescale": change.get("target_timescale"),
                "target_tool_id": change.get("target_tool_id"),
                "target_tool_type": change.get("target_tool_type"),
                "target_param": change.get("target_param"),
                "old_value": json.dumps(change.get("old_value")),
                "new_value": json.dumps(change.get("new_value")),
                "reason": change.get("reason", ""),
                "source_memo_id": source_memo_id,
            })
            row = result.fetchone()
            ids.append(row[0])

    return ids
