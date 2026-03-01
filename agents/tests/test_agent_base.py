"""tests for agent base with mocked claude agent SDK."""

import json
import os
from unittest.mock import AsyncMock, patch

import pytest

from agents.agent_base import (
    CHECKIN_TOOLS,
    PM_TOOLS,
    RECOMMENDATION_TOOLS,
    _build_mcp_tools,
    execute_tool,
    load_prompt,
    run_checkin_agent,
    run_pm_agent,
    run_recommendation_agent,
)
from agents.models import AgentType


def test_load_prompt_exists():
    prompt = load_prompt("checkin_1min")
    assert "1-minute" in prompt
    assert "observation" in prompt.lower()
    assert "no config modification authority" in prompt.lower()


def test_load_prompt_5min():
    prompt = load_prompt("checkin_5min")
    assert "5-minute" in prompt


def test_load_prompt_hourly():
    prompt = load_prompt("checkin_hourly")
    assert "hourly" in prompt


def test_load_prompt_missing():
    with pytest.raises(FileNotFoundError):
        load_prompt("nonexistent_prompt")


def test_load_prompt_recommend_1min():
    prompt = load_prompt("recommend_1min")
    assert "1-minute" in prompt
    assert "recommendation" in prompt.lower()


def test_load_prompt_recommend_5min():
    prompt = load_prompt("recommend_5min")
    assert "5-minute" in prompt
    assert "recommendation" in prompt.lower()


def test_load_prompt_recommend_hourly():
    prompt = load_prompt("recommend_hourly")
    assert "hourly" in prompt
    assert "recommendation" in prompt.lower()


def test_load_prompt_agent_pm():
    prompt = load_prompt("agent_pm")
    assert "portfolio manager" in prompt.lower()
    assert "propose" in prompt.lower()
    assert "config" in prompt.lower()


def test_checkin_tools_have_correct_structure():
    assert len(CHECKIN_TOOLS) == 6
    tool_names = {t["name"] for t in CHECKIN_TOOLS}
    assert "get_recent_trades" in tool_names
    assert "get_daily_performance" in tool_names
    assert "get_current_config" in tool_names
    assert "get_config_changelog" in tool_names
    assert "get_performance_by_exit_reason" in tool_names
    assert "write_observation_memo" in tool_names


def test_recommendation_tools_structure():
    assert len(RECOMMENDATION_TOOLS) == 6
    tool_names = {t["name"] for t in RECOMMENDATION_TOOLS}
    assert "get_recent_trades" in tool_names
    assert "get_daily_performance" in tool_names
    assert "get_current_config" in tool_names
    assert "get_config_changelog" in tool_names
    assert "get_performance_by_exit_reason" in tool_names
    assert "write_recommendation_memo" in tool_names
    # should NOT have observation memo
    assert "write_observation_memo" not in tool_names


def test_pm_tools_structure():
    tool_names = {t["name"] for t in PM_TOOLS}
    # read tools
    assert "get_recent_trades" in tool_names
    assert "get_daily_performance" in tool_names
    assert "get_current_config" in tool_names
    assert "get_config_changelog" in tool_names
    assert "get_performance_by_exit_reason" in tool_names
    # PM-specific tools
    assert "get_checkin_memos" in tool_names
    assert "get_recommendation_memos" in tool_names
    assert "propose_config_mutation" in tool_names
    assert "write_pm_memo" in tool_names
    assert len(PM_TOOLS) == 9


def test_each_tool_has_input_schema():
    for tool in CHECKIN_TOOLS:
        assert "name" in tool
        assert "description" in tool
        assert "input_schema" in tool
        assert tool["input_schema"]["type"] == "object"


def test_write_observation_memo_schema():
    memo_tool = next(t for t in CHECKIN_TOOLS if t["name"] == "write_observation_memo")
    required = memo_tool["input_schema"]["required"]
    assert "confidence_score" in required
    assert "volatility_regime" in required
    assert "directional_bias" in required
    assert "signal_quality" in required
    assert "reasoning" in required


@pytest.mark.asyncio
async def test_execute_tool_unknown():
    engine = AsyncMock()
    result = await execute_tool("nonexistent_tool", {}, engine, 1, AgentType.agent_1min)
    assert "unknown tool" in result


@pytest.mark.asyncio
async def test_execute_tool_write_recommendation_memo():
    engine = AsyncMock()
    with patch("agents.agent_base.write_memo", new_callable=AsyncMock) as mock_write:
        mock_write.return_value = 99
        result = await execute_tool(
            "write_recommendation_memo",
            {
                "confidence_score": 0.8,
                "volatility_regime": "normal",
                "directional_bias": "neutral",
                "signal_quality": "moderate",
                "reasoning": "test recommendation",
            },
            engine,
            cycle_id=1,
            agent_type=AgentType.agent_5min,
        )
        data = json.loads(result)
        assert data["status"] == "success"
        assert data["memo_id"] == 99
        # verify memo_type is recommendation
        memo_arg = mock_write.call_args[0][1]
        assert memo_arg.memo_type.value == "recommendation"


@pytest.mark.asyncio
async def test_execute_tool_get_checkin_memos():
    engine = AsyncMock()
    with patch("agents.agent_base.get_checkin_memos_since_last_pm", new_callable=AsyncMock) as mock_get:
        mock_get.return_value = [{"id": 1, "reasoning": "test"}]
        result = await execute_tool(
            "get_checkin_memos", {}, engine, 1, AgentType.agent_pm
        )
        data = json.loads(result)
        assert len(data) == 1
        assert data[0]["id"] == 1


@pytest.mark.asyncio
async def test_execute_tool_get_recommendation_memos():
    engine = AsyncMock()
    with patch("agents.agent_base.get_recommendation_memos", new_callable=AsyncMock) as mock_get:
        mock_get.return_value = [{"id": 2, "reasoning": "rec test"}]
        result = await execute_tool(
            "get_recommendation_memos", {}, engine, 42, AgentType.agent_pm
        )
        data = json.loads(result)
        assert len(data) == 1
        assert data[0]["id"] == 2
        # verify cycle_id was passed through
        mock_get.assert_called_once_with(engine, 42)


@pytest.mark.asyncio
async def test_execute_tool_propose_config_mutation():
    engine = AsyncMock()
    with patch("agents.agent_base.get_current_config", new_callable=AsyncMock) as mock_config, \
         patch("agents.agent_base.propose_config", new_callable=AsyncMock) as mock_propose:
        mock_config.return_value = {"config_version_id": 10, "config": {}}
        mock_propose.return_value = 11
        result = await execute_tool(
            "propose_config_mutation",
            {"config_blob": {"test": True}, "mutation_reason": "testing"},
            engine,
            cycle_id=1,
            agent_type=AgentType.agent_pm,
        )
        data = json.loads(result)
        assert data["status"] == "success"
        assert data["version_id"] == 11
        mock_propose.assert_called_once_with(
            engine, {"test": True}, 10, "testing", created_by="agent_pm"
        )


@pytest.mark.asyncio
async def test_execute_tool_write_pm_memo():
    engine = AsyncMock()
    with patch("agents.agent_base.write_memo", new_callable=AsyncMock) as mock_write:
        mock_write.return_value = 55
        result = await execute_tool(
            "write_pm_memo",
            {"confidence_score": 0.7, "reasoning": "holding steady"},
            engine,
            cycle_id=1,
            agent_type=AgentType.agent_pm,
        )
        data = json.loads(result)
        assert data["status"] == "success"
        assert data["memo_id"] == 55


# --- _build_mcp_tools tests ---


def test_build_mcp_tools_creates_correct_tools():
    """verify _build_mcp_tools returns SdkMcpTool instances with correct names."""
    from claude_agent_sdk import SdkMcpTool

    engine = AsyncMock()
    tools = _build_mcp_tools(engine, 1, AgentType.agent_1min, CHECKIN_TOOLS)
    assert len(tools) == len(CHECKIN_TOOLS)
    for mcp_tool, tool_def in zip(tools, CHECKIN_TOOLS):
        assert isinstance(mcp_tool, SdkMcpTool)
        assert mcp_tool.name == tool_def["name"]
        assert mcp_tool.description == tool_def["description"]


@pytest.mark.asyncio
async def test_build_mcp_tools_capture_version_id():
    """verify propose_config_mutation handler populates capture dict."""
    engine = AsyncMock()
    capture = {"proposed_version_id": None}

    # only build the propose_config_mutation tool
    propose_def = next(t for t in PM_TOOLS if t["name"] == "propose_config_mutation")
    tools = _build_mcp_tools(engine, 1, AgentType.agent_pm, [propose_def], capture)
    assert len(tools) == 1

    # call the handler with mocked execute_tool
    with patch("agents.agent_base.execute_tool", new_callable=AsyncMock) as mock_exec:
        mock_exec.return_value = json.dumps({"status": "success", "version_id": 42})
        result = await tools[0].handler({"config_blob": {}, "mutation_reason": "test"})

    assert capture["proposed_version_id"] == 42
    assert result["content"][0]["type"] == "text"


# --- SDK query() mock tests ---


def _make_result_message(input_tokens=500, output_tokens=200):
    """create a ResultMessage for mocking query()."""
    from claude_agent_sdk import ResultMessage
    return ResultMessage(
        subtype="result",
        duration_ms=1000,
        duration_api_ms=800,
        is_error=False,
        num_turns=2,
        session_id="test-session",
        total_cost_usd=0.001,
        usage={"input_tokens": input_tokens, "output_tokens": output_tokens},
        result="Analysis complete.",
        structured_output=None,
    )


async def _mock_query_factory(input_tokens=500, output_tokens=200):
    """async generator that yields a single ResultMessage."""
    yield _make_result_message(input_tokens, output_tokens)


@pytest.mark.asyncio
async def test_run_checkin_agent_end_turn():
    """test that agent completes via SDK query()."""
    async def mock_query(*, prompt, options=None, transport=None):
        yield _make_result_message(500, 200)

    mock_engine = AsyncMock()

    with patch("agents.agent_base.query", mock_query), \
         patch("agents.agent_base.create_sdk_mcp_server"):
        usage = await run_checkin_agent(
            agent_type=AgentType.agent_1min,
            prompt_name="checkin_1min",
            engine=mock_engine,
            cycle_id=1,
        )

    assert usage.input_tokens == 500
    assert usage.output_tokens == 200


@pytest.mark.asyncio
async def test_run_checkin_agent_tool_use_then_end():
    """test agent with tool use handled internally by SDK — we only see ResultMessage."""
    async def mock_query(*, prompt, options=None, transport=None):
        # SDK handles tool loop internally; we only see final result with total usage
        yield _make_result_message(1000, 450)

    mock_engine = AsyncMock()

    with patch("agents.agent_base.query", mock_query), \
         patch("agents.agent_base.create_sdk_mcp_server"):
        usage = await run_checkin_agent(
            agent_type=AgentType.agent_5min,
            prompt_name="checkin_5min",
            engine=mock_engine,
            cycle_id=2,
        )

    assert usage.input_tokens == 1000
    assert usage.output_tokens == 450


@pytest.mark.asyncio
async def test_run_pm_agent_no_proposal():
    """test PM agent that decides to hold steady (no config proposal)."""
    async def mock_query(*, prompt, options=None, transport=None):
        yield _make_result_message(1500, 700)

    mock_engine = AsyncMock()

    with patch("agents.agent_base.query", mock_query), \
         patch("agents.agent_base.create_sdk_mcp_server"):
        usage, version_id = await run_pm_agent(
            engine=mock_engine,
            cycle_id=1,
        )

    assert version_id is None
    assert usage.input_tokens == 1500
    assert usage.output_tokens == 700
    assert usage.model == "claude-sonnet-4-5-20250514"


@pytest.mark.asyncio
async def test_run_pm_agent_with_proposal():
    """test PM agent that proposes a config mutation via capture dict."""
    async def mock_query(*, prompt, options=None, transport=None):
        yield _make_result_message(1500, 1000)

    mock_engine = AsyncMock()

    # simulate the capture dict being populated by the tool handler
    original_build = _build_mcp_tools

    def patched_build(engine, cycle_id, agent_type, tool_defs, capture=None):
        if capture is not None:
            capture["proposed_version_id"] = 6
        return original_build(engine, cycle_id, agent_type, tool_defs, capture)

    with patch("agents.agent_base.query", mock_query), \
         patch("agents.agent_base.create_sdk_mcp_server"), \
         patch("agents.agent_base._build_mcp_tools", patched_build):
        usage, version_id = await run_pm_agent(
            engine=mock_engine,
            cycle_id=1,
        )

    assert version_id == 6
    assert usage.input_tokens == 1500
    assert usage.output_tokens == 1000


@pytest.mark.asyncio
async def test_run_recommendation_agent():
    """test recommendation agent uses SONNET_MODEL."""
    captured_options = {}

    async def mock_query(*, prompt, options=None, transport=None):
        captured_options["model"] = options.model if options else None
        yield _make_result_message(800, 400)

    mock_engine = AsyncMock()

    with patch("agents.agent_base.query", mock_query), \
         patch("agents.agent_base.create_sdk_mcp_server"):
        usage = await run_recommendation_agent(
            agent_type=AgentType.agent_1min,
            prompt_name="recommend_1min",
            engine=mock_engine,
            cycle_id=1,
        )

    assert usage.input_tokens == 800
    assert usage.output_tokens == 400
    assert usage.model == "claude-sonnet-4-5-20250514"
    assert captured_options["model"] == "claude-sonnet-4-5-20250514"


# --- live SDK smoke test (requires ANTHROPIC_API_KEY) ---


@pytest.mark.asyncio
@pytest.mark.skipif(
    not os.environ.get("ANTHROPIC_API_KEY"),
    reason="ANTHROPIC_API_KEY not set",
)
async def test_sdk_connection_smoke():
    """smoke test: verify SDK can connect to anthropic and get a response.

    skipped unless ANTHROPIC_API_KEY is set. uses haiku for minimal cost.
    """
    from claude_agent_sdk import ClaudeAgentOptions, ResultMessage, query

    options = ClaudeAgentOptions(
        model="claude-haiku-4-5-20251001",
        permission_mode="bypassPermissions",
        max_turns=1,
    )

    got_result = False
    async for msg in query(prompt="Respond with exactly: ok", options=options):
        if isinstance(msg, ResultMessage):
            got_result = True
            assert not msg.is_error, f"SDK returned error: {msg.result}"
            assert msg.usage is not None, "no usage data in response"
            assert msg.usage.get("input_tokens", 0) > 0, "zero input tokens"
            assert msg.usage.get("output_tokens", 0) > 0, "zero output tokens"

    assert got_result, "query() yielded no ResultMessage"
