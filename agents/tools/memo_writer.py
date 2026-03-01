"""write structured memos to agent_memos table.

provides tools for agents to persist structured observations
and recommendations for consumption by the PM agent.
"""

from __future__ import annotations

import json

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncEngine

from agents.models import AgentMemo


async def write_memo(engine: AsyncEngine, memo: AgentMemo) -> int:
    """write an agent memo to the database. returns the new memo id."""
    query = text("""
        INSERT INTO agent_memos (
            agent, evolution_cycle_id, memo_type,
            confidence_score, volatility_regime, directional_bias,
            signal_quality, flags, reasoning,
            proposed_config_version_id,
            review_period_start, review_period_end,
            trades_reviewed, period_win_rate, period_sharpe, period_pnl
        ) VALUES (
            :agent::agent_type, :evolution_cycle_id, :memo_type::memo_type,
            :confidence_score, :volatility_regime, :directional_bias,
            :signal_quality, :flags::jsonb, :reasoning,
            :proposed_config_version_id,
            :review_period_start, :review_period_end,
            :trades_reviewed, :period_win_rate, :period_sharpe, :period_pnl
        )
        RETURNING id
    """)

    params = {
        "agent": memo.agent.value,
        "evolution_cycle_id": memo.evolution_cycle_id,
        "memo_type": memo.memo_type.value,
        "confidence_score": memo.confidence_score,
        "volatility_regime": memo.volatility_regime.value if memo.volatility_regime else None,
        "directional_bias": memo.directional_bias.value if memo.directional_bias else None,
        "signal_quality": memo.signal_quality.value if memo.signal_quality else None,
        "flags": json.dumps(memo.flags),
        "reasoning": memo.reasoning,
        "proposed_config_version_id": memo.proposed_config_version_id,
        "review_period_start": memo.review_period_start,
        "review_period_end": memo.review_period_end,
        "trades_reviewed": memo.trades_reviewed,
        "period_win_rate": memo.period_win_rate,
        "period_sharpe": memo.period_sharpe,
        "period_pnl": memo.period_pnl,
    }

    async with engine.begin() as conn:
        result = await conn.execute(query, params)
        row = result.fetchone()
        return row[0]
