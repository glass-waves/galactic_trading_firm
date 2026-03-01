"""shared agent invocation logic.

provides a base for all agents with common functionality:
- claude agent sdk integration via in-process MCP tools
- token/cost tracking per invocation
- structured output parsing via pydantic
"""

from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any

from claude_agent_sdk import (
    ClaudeAgentOptions,
    ResultMessage,
    SdkMcpTool,
    create_sdk_mcp_server,
    query,
)
from sqlalchemy.ext.asyncio import AsyncEngine

from agents.models import (
    AgentMemo,
    AgentType,
    MemoType,
    TokenUsage,
)
from agents.tools.memo_writer import write_memo
from agents.tools.sql_queries import (
    get_config_changelog,
    get_daily_performance,
    get_recent_trades,
    get_performance_by_exit_reason,
    get_checkin_memos_since_last_pm,
    get_recommendation_memos,
)
from agents.tools.config_ops import get_current_config, propose_config

logger = logging.getLogger(__name__)

PROMPTS_DIR = Path(__file__).parent / "prompts"

# haiku 4.5 model id
HAIKU_MODEL = "claude-haiku-4-5-20251001"
SONNET_MODEL = "claude-sonnet-4-5-20250514"

# --- shared read-only tool definitions ---

_READ_TOOLS: list[dict[str, Any]] = [
    {
        "name": "get_recent_trades",
        "description": "Fetch recent completed trades. Returns trade records with entry/exit prices, P&L, hold duration, and exit reason.",
        "input_schema": {
            "type": "object",
            "properties": {
                "ticker": {
                    "type": "string",
                    "description": "Filter by ticker symbol (e.g., 'SPY'). Omit for all tickers.",
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of trades to return. Default: 20.",
                    "default": 20,
                },
            },
            "required": [],
        },
    },
    {
        "name": "get_daily_performance",
        "description": "Fetch daily performance summary with total trades, win rate, total P&L, and average hold time.",
        "input_schema": {
            "type": "object",
            "properties": {
                "ticker": {
                    "type": "string",
                    "description": "Filter by ticker symbol. Omit for all tickers.",
                },
            },
            "required": [],
        },
    },
    {
        "name": "get_current_config",
        "description": "Read the currently active (promoted) strategy configuration.",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": [],
        },
    },
    {
        "name": "get_config_changelog",
        "description": "Fetch recent config changes. Shows what parameters were modified and why.",
        "input_schema": {
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Maximum changelog entries to return. Default: 20.",
                    "default": 20,
                },
            },
            "required": [],
        },
    },
    {
        "name": "get_performance_by_exit_reason",
        "description": "Fetch trade performance broken down by exit reason (trailing stop, hard stop, session close, etc.).",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": [],
        },
    },
]

_WRITE_OBSERVATION_MEMO_TOOL: dict[str, Any] = {
    "name": "write_observation_memo",
    "description": "Write your structured observation memo. Call this once you have completed your analysis.",
    "input_schema": {
        "type": "object",
        "properties": {
            "confidence_score": {
                "type": "number",
                "description": "Your confidence in this observation, 0.0 to 1.0.",
            },
            "volatility_regime": {
                "type": "string",
                "enum": ["low", "normal", "high", "extreme"],
                "description": "Current volatility regime assessment.",
            },
            "directional_bias": {
                "type": "string",
                "enum": ["strong_long", "lean_long", "neutral", "lean_short", "strong_short"],
                "description": "Current directional bias assessment.",
            },
            "signal_quality": {
                "type": "string",
                "enum": ["strong", "moderate", "weak", "conflicting"],
                "description": "Quality of current trading signals.",
            },
            "flags": {
                "type": "object",
                "description": "Boolean flags for notable patterns. Keys like 'divergence_detected', 'volume_anomaly', 'level_rejection'.",
            },
            "reasoning": {
                "type": "string",
                "description": "Your detailed analysis and reasoning. This is read by the PM agent for context.",
            },
            "trades_reviewed": {
                "type": "integer",
                "description": "Number of trades you reviewed.",
            },
            "period_win_rate": {
                "type": "number",
                "description": "Win rate for the period you reviewed.",
            },
            "period_pnl": {
                "type": "number",
                "description": "Total P&L for the period you reviewed.",
            },
        },
        "required": ["confidence_score", "volatility_regime", "directional_bias", "signal_quality", "reasoning"],
    },
}

_WRITE_RECOMMENDATION_MEMO_TOOL: dict[str, Any] = {
    "name": "write_recommendation_memo",
    "description": "Write your structured recommendation memo with parameter change suggestions. Call this once you have completed your analysis.",
    "input_schema": {
        "type": "object",
        "properties": {
            "confidence_score": {
                "type": "number",
                "description": "Your confidence in these recommendations, 0.0 to 1.0.",
            },
            "volatility_regime": {
                "type": "string",
                "enum": ["low", "normal", "high", "extreme"],
                "description": "Current volatility regime assessment.",
            },
            "directional_bias": {
                "type": "string",
                "enum": ["strong_long", "lean_long", "neutral", "lean_short", "strong_short"],
                "description": "Current directional bias assessment.",
            },
            "signal_quality": {
                "type": "string",
                "enum": ["strong", "moderate", "weak", "conflicting"],
                "description": "Quality of current trading signals.",
            },
            "flags": {
                "type": "object",
                "description": "Boolean flags for notable patterns.",
            },
            "reasoning": {
                "type": "string",
                "description": "Your detailed analysis with specific parameter change suggestions. Cite trade IDs and metrics.",
            },
            "trades_reviewed": {
                "type": "integer",
                "description": "Number of trades you reviewed.",
            },
            "period_win_rate": {
                "type": "number",
                "description": "Win rate for the period you reviewed.",
            },
            "period_pnl": {
                "type": "number",
                "description": "Total P&L for the period you reviewed.",
            },
        },
        "required": ["confidence_score", "volatility_regime", "directional_bias", "signal_quality", "reasoning"],
    },
}

# tool definitions for claude's tool use api
CHECKIN_TOOLS: list[dict[str, Any]] = [*_READ_TOOLS, _WRITE_OBSERVATION_MEMO_TOOL]

RECOMMENDATION_TOOLS: list[dict[str, Any]] = [*_READ_TOOLS, _WRITE_RECOMMENDATION_MEMO_TOOL]

PM_TOOLS: list[dict[str, Any]] = [
    *_READ_TOOLS,
    {
        "name": "get_checkin_memos",
        "description": "Fetch recent check-in observation memos since the last PM cycle. Use this to understand what the timescale agents have been observing.",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": [],
        },
    },
    {
        "name": "get_recommendation_memos",
        "description": "Fetch recommendation memos from this PM cycle's timescale agents. These contain specific parameter change suggestions.",
        "input_schema": {
            "type": "object",
            "properties": {},
            "required": [],
        },
    },
    {
        "name": "propose_config_mutation",
        "description": "Propose a new strategy config. Provide a complete config blob with your changes. The config will be validated via backtest before promotion.",
        "input_schema": {
            "type": "object",
            "properties": {
                "config_blob": {
                    "type": "object",
                    "description": "The complete new strategy config blob. Copy the current config and modify specific fields.",
                },
                "mutation_reason": {
                    "type": "string",
                    "description": "Explanation of what you changed and why, citing evidence.",
                },
            },
            "required": ["config_blob", "mutation_reason"],
        },
    },
    {
        "name": "write_pm_memo",
        "description": "Write a PM decision memo explaining your analysis and decision (hold steady or why you proposed changes).",
        "input_schema": {
            "type": "object",
            "properties": {
                "confidence_score": {
                    "type": "number",
                    "description": "Your confidence in this decision, 0.0 to 1.0.",
                },
                "reasoning": {
                    "type": "string",
                    "description": "Your detailed analysis, evidence reviewed, and decision rationale.",
                },
                "trades_reviewed": {
                    "type": "integer",
                    "description": "Number of trades you reviewed.",
                },
                "period_win_rate": {
                    "type": "number",
                    "description": "Win rate for the period you reviewed.",
                },
                "period_pnl": {
                    "type": "number",
                    "description": "Total P&L for the period you reviewed.",
                },
            },
            "required": ["confidence_score", "reasoning"],
        },
    },
]


def load_prompt(prompt_name: str) -> str:
    """load a system prompt from the prompts directory."""
    prompt_path = PROMPTS_DIR / f"{prompt_name}.md"
    if not prompt_path.exists():
        raise FileNotFoundError(f"prompt not found: {prompt_path}")
    return prompt_path.read_text()


async def execute_tool(
    tool_name: str,
    tool_input: dict[str, Any],
    engine: AsyncEngine,
    cycle_id: int,
    agent_type: AgentType,
) -> str:
    """execute a tool call and return the result as a json string."""
    try:
        if tool_name == "get_recent_trades":
            trades = await get_recent_trades(
                engine,
                ticker=tool_input.get("ticker"),
                limit=tool_input.get("limit", 20),
            )
            return json.dumps([t.model_dump(mode="json") for t in trades])

        elif tool_name == "get_daily_performance":
            perf = await get_daily_performance(
                engine,
                ticker=tool_input.get("ticker"),
            )
            return json.dumps([p.model_dump(mode="json") for p in perf])

        elif tool_name == "get_current_config":
            config = await get_current_config(engine)
            return json.dumps(config) if config else json.dumps({"error": "no promoted config found"})

        elif tool_name == "get_config_changelog":
            changelog = await get_config_changelog(
                engine,
                limit=tool_input.get("limit", 20),
            )
            return json.dumps([c.model_dump(mode="json") for c in changelog])

        elif tool_name == "get_performance_by_exit_reason":
            perf = await get_performance_by_exit_reason(engine)
            return json.dumps(perf)

        elif tool_name == "write_observation_memo":
            memo = AgentMemo(
                agent=agent_type,
                evolution_cycle_id=cycle_id,
                memo_type=MemoType.observation,
                confidence_score=tool_input.get("confidence_score"),
                volatility_regime=tool_input.get("volatility_regime"),
                directional_bias=tool_input.get("directional_bias"),
                signal_quality=tool_input.get("signal_quality"),
                flags=tool_input.get("flags", {}),
                reasoning=tool_input.get("reasoning", ""),
                trades_reviewed=tool_input.get("trades_reviewed"),
                period_win_rate=tool_input.get("period_win_rate"),
                period_pnl=tool_input.get("period_pnl"),
            )
            memo_id = await write_memo(engine, memo)
            return json.dumps({"status": "success", "memo_id": memo_id})

        elif tool_name == "write_recommendation_memo":
            memo = AgentMemo(
                agent=agent_type,
                evolution_cycle_id=cycle_id,
                memo_type=MemoType.recommendation,
                confidence_score=tool_input.get("confidence_score"),
                volatility_regime=tool_input.get("volatility_regime"),
                directional_bias=tool_input.get("directional_bias"),
                signal_quality=tool_input.get("signal_quality"),
                flags=tool_input.get("flags", {}),
                reasoning=tool_input.get("reasoning", ""),
                trades_reviewed=tool_input.get("trades_reviewed"),
                period_win_rate=tool_input.get("period_win_rate"),
                period_pnl=tool_input.get("period_pnl"),
            )
            memo_id = await write_memo(engine, memo)
            return json.dumps({"status": "success", "memo_id": memo_id})

        elif tool_name == "get_checkin_memos":
            memos = await get_checkin_memos_since_last_pm(engine)
            return json.dumps(memos, default=str)

        elif tool_name == "get_recommendation_memos":
            memos = await get_recommendation_memos(engine, cycle_id)
            return json.dumps(memos, default=str)

        elif tool_name == "propose_config_mutation":
            config_blob = tool_input["config_blob"]
            mutation_reason = tool_input["mutation_reason"]
            # get current config to find parent version id
            current = await get_current_config(engine)
            parent_id = current.get("config_version_id") if current else None
            version_id = await propose_config(
                engine, config_blob, parent_id, mutation_reason,
                created_by=agent_type.value,
            )
            return json.dumps({"status": "success", "version_id": version_id})

        elif tool_name == "write_pm_memo":
            memo = AgentMemo(
                agent=agent_type,
                evolution_cycle_id=cycle_id,
                memo_type=MemoType.recommendation,
                confidence_score=tool_input.get("confidence_score"),
                reasoning=tool_input.get("reasoning", ""),
                trades_reviewed=tool_input.get("trades_reviewed"),
                period_win_rate=tool_input.get("period_win_rate"),
                period_pnl=tool_input.get("period_pnl"),
            )
            memo_id = await write_memo(engine, memo)
            return json.dumps({"status": "success", "memo_id": memo_id})

        else:
            return json.dumps({"error": f"unknown tool: {tool_name}"})

    except Exception as e:
        logger.exception(f"tool execution error: {tool_name}")
        return json.dumps({"error": str(e)})


def _build_mcp_tools(
    engine: AsyncEngine,
    cycle_id: int,
    agent_type: AgentType,
    tool_defs: list[dict[str, Any]],
    capture: dict[str, Any] | None = None,
) -> list[SdkMcpTool]:
    """build SdkMcpTool instances from tool definition dicts.

    each tool's handler calls execute_tool() and wraps the result in MCP format.
    for propose_config_mutation, the handler also writes the version_id to capture.
    """
    mcp_tools: list[SdkMcpTool] = []

    for tool_def in tool_defs:
        _name = tool_def["name"]

        async def handler(args: dict[str, Any], _name: str = _name) -> dict[str, Any]:
            result = await execute_tool(_name, args, engine, cycle_id, agent_type)
            # capture proposed version id if applicable
            if _name == "propose_config_mutation" and capture is not None:
                try:
                    result_data = json.loads(result)
                    if "version_id" in result_data:
                        capture["proposed_version_id"] = result_data["version_id"]
                except (json.JSONDecodeError, KeyError):
                    pass
            return {"content": [{"type": "text", "text": result}]}

        mcp_tools.append(
            SdkMcpTool(
                name=_name,
                description=tool_def["description"],
                input_schema=tool_def["input_schema"],
                handler=handler,
            )
        )

    return mcp_tools


async def _run_agent_sdk(
    agent_type: AgentType,
    prompt_name: str,
    engine: AsyncEngine,
    cycle_id: int,
    tool_defs: list[dict[str, Any]],
    initial_message: str,
    model: str = HAIKU_MODEL,
    max_turns: int = 10,
    capture: dict[str, Any] | None = None,
) -> TokenUsage:
    """run an agent using the claude agent SDK.

    builds MCP tools from tool_defs, creates an in-process MCP server,
    and iterates the SDK query() async generator to completion.
    """
    system_prompt = load_prompt(prompt_name)
    total_usage = TokenUsage(model=model)

    mcp_tools = _build_mcp_tools(engine, cycle_id, agent_type, tool_defs, capture)
    server = create_sdk_mcp_server(name="trading", version="1.0.0", tools=mcp_tools)

    allowed_tools = [f"mcp__trading__{t['name']}" for t in tool_defs]

    options = ClaudeAgentOptions(
        model=model,
        system_prompt=system_prompt,
        mcp_servers={"trading": server},
        allowed_tools=allowed_tools,
        permission_mode="bypassPermissions",
        max_turns=max_turns,
    )

    async for msg in query(prompt=initial_message, options=options):
        if isinstance(msg, ResultMessage):
            usage = msg.usage or {}
            total_usage.input_tokens += usage.get("input_tokens", 0)
            total_usage.output_tokens += usage.get("output_tokens", 0)

    logger.info(
        f"agent {agent_type.value}: {total_usage.input_tokens} input tokens, "
        f"{total_usage.output_tokens} output tokens, "
        f"${total_usage.estimated_cost_usd:.4f}"
    )

    return total_usage


async def run_checkin_agent(
    agent_type: AgentType,
    prompt_name: str,
    engine: AsyncEngine,
    cycle_id: int,
    model: str = HAIKU_MODEL,
    max_turns: int = 10,
) -> TokenUsage:
    """run a single check-in agent cycle.

    loads the system prompt, runs the SDK agent with check-in tools,
    and tracks token usage.

    returns the total token usage for this agent run.
    """
    return await _run_agent_sdk(
        agent_type=agent_type,
        prompt_name=prompt_name,
        engine=engine,
        cycle_id=cycle_id,
        tool_defs=CHECKIN_TOOLS,
        initial_message=(
            "Run your check-in analysis now. Use the available tools to gather data, "
            "then write your observation memo."
        ),
        model=model,
        max_turns=max_turns,
    )


async def run_recommendation_agent(
    agent_type: AgentType,
    prompt_name: str,
    engine: AsyncEngine,
    cycle_id: int,
    model: str = SONNET_MODEL,
    max_turns: int = 10,
) -> TokenUsage:
    """run a single recommendation agent cycle.

    like run_checkin_agent but uses SONNET_MODEL and RECOMMENDATION_TOOLS.
    produces recommendation memos with parameter change suggestions.
    """
    return await _run_agent_sdk(
        agent_type=agent_type,
        prompt_name=prompt_name,
        engine=engine,
        cycle_id=cycle_id,
        tool_defs=RECOMMENDATION_TOOLS,
        initial_message=(
            "Run your recommendation analysis now. Use the available tools to gather data, "
            "then write your recommendation memo with specific parameter change suggestions."
        ),
        model=model,
        max_turns=max_turns,
    )


async def run_pm_agent(
    engine: AsyncEngine,
    cycle_id: int,
    model: str = SONNET_MODEL,
    max_turns: int = 15,
) -> tuple[TokenUsage, int | None]:
    """run the PM agent cycle.

    reads recommendation and check-in memos, reviews performance,
    and either proposes a config mutation or writes a hold-steady memo.

    returns (usage, proposed_config_version_id | None).
    """
    capture: dict[str, Any] = {"proposed_version_id": None}

    usage = await _run_agent_sdk(
        agent_type=AgentType.agent_pm,
        prompt_name="agent_pm",
        engine=engine,
        cycle_id=cycle_id,
        tool_defs=PM_TOOLS,
        initial_message=(
            "Run your PM analysis now. Review the recommendation memos from this cycle's "
            "timescale agents and recent check-in observations. Analyze trade performance "
            "and decide whether to propose config changes or hold steady."
        ),
        model=model,
        max_turns=max_turns,
        capture=capture,
    )

    logger.info(f"PM agent: proposed_version_id={capture['proposed_version_id']}")

    return usage, capture["proposed_version_id"]
