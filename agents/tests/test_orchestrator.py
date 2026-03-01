"""tests for orchestrator with mocked database and api."""

from datetime import date
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from agents.models import DailyBudget, TokenUsage
from agents.orchestrator import (
    CHECKIN_AGENTS,
    PM_RECOMMENDATION_AGENTS,
    _scheduled_checkin,
    _scheduled_pm_cycle,
)


def test_checkin_agents_list():
    assert len(CHECKIN_AGENTS) == 3
    prompt_names = [p for _, p in CHECKIN_AGENTS]
    assert "checkin_1min" in prompt_names
    assert "checkin_5min" in prompt_names
    assert "checkin_hourly" in prompt_names


def test_pm_recommendation_agents_list():
    assert len(PM_RECOMMENDATION_AGENTS) == 3
    prompt_names = [p for _, p in PM_RECOMMENDATION_AGENTS]
    assert "recommend_1min" in prompt_names
    assert "recommend_5min" in prompt_names
    assert "recommend_hourly" in prompt_names


def test_budget_not_exhausted():
    budget = DailyBudget(
        trading_date=date(2024, 1, 15),
        total_cost_usd=2.50,
        budget_limit_usd=5.00,
        budget_exhausted=False,
    )
    assert budget.total_cost_usd < budget.budget_limit_usd
    assert not budget.budget_exhausted


def test_budget_exhausted():
    budget = DailyBudget(
        trading_date=date(2024, 1, 15),
        total_cost_usd=5.10,
        budget_limit_usd=5.00,
        budget_exhausted=True,
    )
    assert budget.budget_exhausted


def test_token_usage_accumulation():
    total = TokenUsage()
    usage1 = TokenUsage(input_tokens=1000, output_tokens=500)
    usage2 = TokenUsage(input_tokens=2000, output_tokens=800)

    total.input_tokens += usage1.input_tokens + usage2.input_tokens
    total.output_tokens += usage1.output_tokens + usage2.output_tokens

    assert total.input_tokens == 3000
    assert total.output_tokens == 1300
    assert total.estimated_cost_usd > 0


@pytest.mark.asyncio
async def test_run_checkin_cycle_budget_exhausted():
    """test that cycle stops when budget is exhausted."""
    with patch("agents.orchestrator.get_engine", new_callable=AsyncMock), \
         patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.close_engine", new_callable=AsyncMock):

        mock_check.return_value = (
            False,
            DailyBudget(
                trading_date=date.today(),
                total_cost_usd=5.10,
                budget_limit_usd=5.00,
                budget_exhausted=True,
            ),
        )

        from agents.orchestrator import run_checkin_cycle
        result = await run_checkin_cycle()
        assert result is None


@pytest.mark.asyncio
async def test_run_checkin_cycle_success():
    """test successful cycle with mocked agents."""
    mock_engine = AsyncMock()

    with patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.create_evolution_cycle", new_callable=AsyncMock) as mock_create, \
         patch("agents.orchestrator.run_checkin_agent", new_callable=AsyncMock) as mock_agent, \
         patch("agents.orchestrator.complete_evolution_cycle", new_callable=AsyncMock), \
         patch("agents.orchestrator.update_budget", new_callable=AsyncMock) as mock_update:

        mock_check.return_value = (
            True,
            DailyBudget(trading_date=date.today(), total_cost_usd=1.00),
        )
        mock_create.return_value = 42  # cycle id
        mock_agent.return_value = TokenUsage(input_tokens=1000, output_tokens=500)
        mock_update.return_value = DailyBudget(
            trading_date=date.today(), total_cost_usd=1.50,
        )

        from agents.orchestrator import run_checkin_cycle
        result = await run_checkin_cycle(engine=mock_engine)

        assert result is not None
        assert result.input_tokens == 3000  # 1000 * 3 agents
        assert result.output_tokens == 1500  # 500 * 3 agents
        assert mock_agent.call_count == 3
        assert mock_create.call_count == 1


@pytest.mark.asyncio
async def test_run_full_pm_cycle_budget_exhausted():
    """test that PM cycle returns None when budget exhausted."""
    with patch("agents.orchestrator.get_engine", new_callable=AsyncMock), \
         patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.close_engine", new_callable=AsyncMock):

        mock_check.return_value = (
            False,
            DailyBudget(
                trading_date=date.today(),
                total_cost_usd=5.10,
                budget_limit_usd=5.00,
                budget_exhausted=True,
            ),
        )

        from agents.orchestrator import run_full_pm_cycle
        result = await run_full_pm_cycle()
        assert result is None


@pytest.mark.asyncio
async def test_run_full_pm_cycle_no_proposal():
    """test PM cycle when PM agent holds steady."""
    mock_engine = AsyncMock()

    with patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.create_evolution_cycle", new_callable=AsyncMock) as mock_create, \
         patch("agents.orchestrator.run_recommendation_agent", new_callable=AsyncMock) as mock_rec, \
         patch("agents.orchestrator.run_pm_agent", new_callable=AsyncMock) as mock_pm, \
         patch("agents.orchestrator.complete_evolution_cycle", new_callable=AsyncMock) as mock_complete, \
         patch("agents.orchestrator.update_budget", new_callable=AsyncMock) as mock_update:

        mock_check.return_value = (
            True,
            DailyBudget(trading_date=date.today(), total_cost_usd=1.00),
        )
        mock_create.return_value = 100
        mock_rec.return_value = TokenUsage(
            input_tokens=500, output_tokens=200, model="claude-sonnet-4-5-20250514",
        )
        mock_pm.return_value = (
            TokenUsage(input_tokens=1000, output_tokens=500, model="claude-sonnet-4-5-20250514"),
            None,  # no proposal
        )
        mock_update.return_value = DailyBudget(trading_date=date.today(), total_cost_usd=2.00)

        from agents.orchestrator import run_full_pm_cycle
        result = await run_full_pm_cycle(engine=mock_engine)

        assert result is not None
        # 3 rec agents * 500 + 1000 PM = 2500
        assert result.input_tokens == 2500
        assert mock_rec.call_count == 3
        assert mock_pm.call_count == 1
        # verify complete was called with 0 configs
        mock_complete.assert_called_once()
        call_kwargs = mock_complete.call_args
        assert call_kwargs.kwargs.get("configs_proposed", call_kwargs[1].get("configs_proposed", 0)) == 0


@pytest.mark.asyncio
async def test_run_full_pm_cycle_proposal_promoted():
    """happy path: PM proposes config, no backtest data dir, auto-promoted."""
    mock_engine = AsyncMock()

    with patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.create_evolution_cycle", new_callable=AsyncMock) as mock_create, \
         patch("agents.orchestrator.run_recommendation_agent", new_callable=AsyncMock) as mock_rec, \
         patch("agents.orchestrator.run_pm_agent", new_callable=AsyncMock) as mock_pm, \
         patch("agents.orchestrator.update_config_status", new_callable=AsyncMock) as mock_status, \
         patch("agents.orchestrator.get_config_version", new_callable=AsyncMock) as mock_get_ver, \
         patch("agents.orchestrator.get_current_config", new_callable=AsyncMock) as mock_current, \
         patch("agents.orchestrator.compute_config_diff") as mock_diff, \
         patch("agents.orchestrator.write_changelog_entries", new_callable=AsyncMock), \
         patch("agents.orchestrator.complete_evolution_cycle", new_callable=AsyncMock), \
         patch("agents.orchestrator.update_budget", new_callable=AsyncMock) as mock_update, \
         patch.dict("os.environ", {"BACKTEST_DATA_DIR": ""}, clear=False):

        mock_check.return_value = (
            True,
            DailyBudget(trading_date=date.today(), total_cost_usd=1.00),
        )
        mock_create.return_value = 200
        mock_rec.return_value = TokenUsage(
            input_tokens=500, output_tokens=200, model="claude-sonnet-4-5-20250514",
        )
        mock_pm.return_value = (
            TokenUsage(input_tokens=1000, output_tokens=500, model="claude-sonnet-4-5-20250514"),
            42,  # proposed version id
        )
        mock_get_ver.return_value = {"config_blob": {"tickers": ["SPY"]}}
        mock_current.return_value = {"config_version_id": 41, "config": {"tickers": ["SPY"]}}
        mock_diff.return_value = []
        mock_update.return_value = DailyBudget(trading_date=date.today(), total_cost_usd=3.00)

        from agents.orchestrator import run_full_pm_cycle
        result = await run_full_pm_cycle(engine=mock_engine)

        assert result is not None
        # verify config was promoted (status calls: backtesting + promoted)
        assert mock_status.call_count == 2
        status_calls = [c.args[2] for c in mock_status.call_args_list]
        assert "backtesting" in status_calls
        assert "promoted" in status_calls


@pytest.mark.asyncio
async def test_run_full_pm_cycle_proposal_rejected():
    """PM proposes config, backtest fails validation, config rejected."""
    mock_engine = AsyncMock()

    with patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.create_evolution_cycle", new_callable=AsyncMock) as mock_create, \
         patch("agents.orchestrator.run_recommendation_agent", new_callable=AsyncMock) as mock_rec, \
         patch("agents.orchestrator.run_pm_agent", new_callable=AsyncMock) as mock_pm, \
         patch("agents.orchestrator.update_config_status", new_callable=AsyncMock) as mock_status, \
         patch("agents.orchestrator.get_config_version", new_callable=AsyncMock) as mock_get_ver, \
         patch("agents.orchestrator.get_current_config", new_callable=AsyncMock) as mock_current, \
         patch("agents.orchestrator.run_backtest_validation") as mock_backtest, \
         patch("agents.orchestrator.validate_backtest_result") as mock_validate, \
         patch("agents.orchestrator.complete_evolution_cycle", new_callable=AsyncMock), \
         patch("agents.orchestrator.update_budget", new_callable=AsyncMock) as mock_update, \
         patch("os.path.isdir", return_value=True), \
         patch("os.path.exists", return_value=True), \
         patch.dict("os.environ", {"BACKTEST_DATA_DIR": "/tmp/data"}, clear=False):

        mock_check.return_value = (
            True,
            DailyBudget(trading_date=date.today(), total_cost_usd=1.00),
        )
        mock_create.return_value = 300
        mock_rec.return_value = TokenUsage(
            input_tokens=500, output_tokens=200, model="claude-sonnet-4-5-20250514",
        )
        mock_pm.return_value = (
            TokenUsage(input_tokens=1000, output_tokens=500, model="claude-sonnet-4-5-20250514"),
            50,  # proposed version id
        )
        mock_get_ver.return_value = {"config_blob": {"tickers": ["SPY"]}}
        mock_current.return_value = {"config_version_id": 49, "config": {"tickers": ["SPY"]}}
        mock_backtest.return_value = {"metrics": {"sharpe_ratio": 0.1, "win_rate": 0.2, "max_drawdown_pct": 0.3, "total_trades": 10}}
        mock_validate.return_value = (False, ["win rate too low", "drawdown too high"])
        mock_update.return_value = DailyBudget(trading_date=date.today(), total_cost_usd=3.00)

        from agents.orchestrator import run_full_pm_cycle
        result = await run_full_pm_cycle(engine=mock_engine)

        assert result is not None
        # verify config was rejected
        status_calls = [c.args[2] for c in mock_status.call_args_list]
        assert "backtesting" in status_calls
        assert "rejected" in status_calls


@pytest.mark.asyncio
async def test_run_full_pm_cycle_backtest_error():
    """PM proposes config, backtest throws error, config rejected."""
    mock_engine = AsyncMock()

    with patch("agents.orchestrator.check_budget", new_callable=AsyncMock) as mock_check, \
         patch("agents.orchestrator.create_evolution_cycle", new_callable=AsyncMock) as mock_create, \
         patch("agents.orchestrator.run_recommendation_agent", new_callable=AsyncMock) as mock_rec, \
         patch("agents.orchestrator.run_pm_agent", new_callable=AsyncMock) as mock_pm, \
         patch("agents.orchestrator.update_config_status", new_callable=AsyncMock) as mock_status, \
         patch("agents.orchestrator.get_config_version", new_callable=AsyncMock) as mock_get_ver, \
         patch("agents.orchestrator.get_current_config", new_callable=AsyncMock) as mock_current, \
         patch("agents.orchestrator.run_backtest_validation", side_effect=Exception("cargo not found")), \
         patch("agents.orchestrator.complete_evolution_cycle", new_callable=AsyncMock), \
         patch("agents.orchestrator.update_budget", new_callable=AsyncMock) as mock_update, \
         patch("os.path.isdir", return_value=True), \
         patch("os.path.exists", return_value=True), \
         patch.dict("os.environ", {"BACKTEST_DATA_DIR": "/tmp/data"}, clear=False):

        mock_check.return_value = (
            True,
            DailyBudget(trading_date=date.today(), total_cost_usd=1.00),
        )
        mock_create.return_value = 400
        mock_rec.return_value = TokenUsage(
            input_tokens=500, output_tokens=200, model="claude-sonnet-4-5-20250514",
        )
        mock_pm.return_value = (
            TokenUsage(input_tokens=1000, output_tokens=500, model="claude-sonnet-4-5-20250514"),
            60,  # proposed version id
        )
        mock_get_ver.return_value = {"config_blob": {"tickers": ["SPY"]}}
        mock_current.return_value = {"config_version_id": 59, "config": {"tickers": ["SPY"]}}
        mock_update.return_value = DailyBudget(trading_date=date.today(), total_cost_usd=3.00)

        from agents.orchestrator import run_full_pm_cycle
        result = await run_full_pm_cycle(engine=mock_engine)

        assert result is not None
        # verify config was rejected due to error
        status_calls = [c.args[2] for c in mock_status.call_args_list]
        assert "rejected" in status_calls


# --- scheduler wrapper and run_scheduled tests ---


@pytest.mark.asyncio
async def test_scheduled_checkin_wrapper_catches_exception():
    """_scheduled_checkin catches and logs exceptions without propagating."""
    with patch("agents.orchestrator.run_checkin_cycle", new_callable=AsyncMock) as mock_cycle:
        mock_cycle.side_effect = RuntimeError("db connection lost")
        # should not raise
        await _scheduled_checkin()
        mock_cycle.assert_called_once()


@pytest.mark.asyncio
async def test_scheduled_pm_wrapper_catches_exception():
    """_scheduled_pm_cycle catches and logs exceptions without propagating."""
    with patch("agents.orchestrator.run_full_pm_cycle", new_callable=AsyncMock) as mock_cycle:
        mock_cycle.side_effect = RuntimeError("api timeout")
        # should not raise
        await _scheduled_pm_cycle()
        mock_cycle.assert_called_once()


@pytest.mark.asyncio
async def test_run_scheduled_creates_jobs():
    """run_scheduled creates check-in and PM jobs with correct triggers."""
    mock_scheduler = MagicMock()

    # mock Event so wait() returns immediately (simulates instant signal)
    mock_event = MagicMock()
    mock_event.wait = AsyncMock()
    mock_event.set = MagicMock()

    with patch("apscheduler.schedulers.asyncio.AsyncIOScheduler", return_value=mock_scheduler) as mock_cls, \
         patch("asyncio.Event", return_value=mock_event), \
         patch("agents.orchestrator.close_engine", new_callable=AsyncMock):

        from agents.orchestrator import run_scheduled
        await run_scheduled()

        # verify scheduler was created with US/Eastern timezone
        mock_cls.assert_called_once_with(timezone="US/Eastern")

        # verify add_job was called twice (checkin + pm)
        assert mock_scheduler.add_job.call_count == 2

        job_ids = [call.kwargs.get("id") for call in mock_scheduler.add_job.call_args_list]
        assert "checkin" in job_ids
        assert "pm_cycle" in job_ids

        mock_scheduler.start.assert_called_once()
        mock_scheduler.shutdown.assert_called_once_with(wait=True)


@pytest.mark.asyncio
async def test_run_scheduled_signal_shutdown():
    """run_scheduled shuts down cleanly and calls close_engine."""
    mock_scheduler = MagicMock()

    mock_event = MagicMock()
    mock_event.wait = AsyncMock()
    mock_event.set = MagicMock()

    with patch("apscheduler.schedulers.asyncio.AsyncIOScheduler", return_value=mock_scheduler), \
         patch("asyncio.Event", return_value=mock_event), \
         patch("agents.orchestrator.close_engine", new_callable=AsyncMock) as mock_close:

        from agents.orchestrator import run_scheduled
        await run_scheduled()

        mock_scheduler.shutdown.assert_called_once_with(wait=True)
        mock_close.assert_called_once()
