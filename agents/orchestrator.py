"""main scheduler, budget tracking, cycle dispatch.

orchestrates the two-tier evolution cycle:
- full PM cycles (sonnet 4.5): 1-3x daily, all agents, config authority
- check-in cycles (haiku 4.5): more frequent, observation-only memos

enforces a $5/day budget cap under anthropic tier 1.
"""

from __future__ import annotations

import asyncio
import logging
import os
import signal
from datetime import date

from sqlalchemy import text
from sqlalchemy.ext.asyncio import AsyncEngine

from agents.agent_base import (
    run_checkin_agent,
    run_recommendation_agent,
    run_pm_agent,
    HAIKU_MODEL,
    SONNET_MODEL,
)
from agents.db import get_engine, close_engine
from agents.models import AgentType, CycleType, DailyBudget, TokenUsage
from agents.tools.backtest_runner import (
    run_backtest_validation,
    validate_backtest_result,
    DEFAULT_DATA_DIR,
)
from agents.tools.config_diff import compute_config_diff
from agents.tools.config_ops import (
    get_current_config,
    get_config_version,
    update_config_status,
    write_changelog_entries,
)

logger = logging.getLogger(__name__)

# check-in agent schedule: which agents run and their prompt names
CHECKIN_AGENTS = [
    (AgentType.agent_1min, "checkin_1min"),
    (AgentType.agent_5min, "checkin_5min"),
    (AgentType.agent_hourly, "checkin_hourly"),
]

# PM cycle recommendation agents
PM_RECOMMENDATION_AGENTS = [
    (AgentType.agent_1min, "recommend_1min"),
    (AgentType.agent_5min, "recommend_5min"),
    (AgentType.agent_hourly, "recommend_hourly"),
]

# default budget cap per day (usd)
DEFAULT_BUDGET_LIMIT = 5.00


async def get_or_create_daily_budget(
    engine: AsyncEngine,
    trading_date: date | None = None,
) -> DailyBudget:
    """get today's budget record, creating it if it doesn't exist."""
    if trading_date is None:
        trading_date = date.today()

    async with engine.begin() as conn:
        result = await conn.execute(
            text("SELECT * FROM daily_budget WHERE trading_date = :d"),
            {"d": trading_date},
        )
        row = result.mappings().first()

        if row is not None:
            return DailyBudget(**dict(row))

        # create new budget row for today
        await conn.execute(
            text("""
                INSERT INTO daily_budget (trading_date, budget_limit_usd)
                VALUES (:d, :limit)
            """),
            {"d": trading_date, "limit": DEFAULT_BUDGET_LIMIT},
        )
        return DailyBudget(trading_date=trading_date, budget_limit_usd=DEFAULT_BUDGET_LIMIT)


async def check_budget(engine: AsyncEngine) -> tuple[bool, DailyBudget]:
    """check if we have budget remaining for today.

    returns (can_proceed, budget).
    """
    budget = await get_or_create_daily_budget(engine)
    can_proceed = not budget.budget_exhausted and budget.total_cost_usd < budget.budget_limit_usd
    return can_proceed, budget


async def update_budget(
    engine: AsyncEngine,
    usage: TokenUsage,
    cycle_type: CycleType = CycleType.checkin,
) -> DailyBudget:
    """update today's budget with token usage from a cycle."""
    trading_date = date.today()

    cycle_col = "checkin_cycles" if cycle_type == CycleType.checkin else "full_pm_cycles"

    async with engine.begin() as conn:
        await conn.execute(
            text(f"""
                UPDATE daily_budget SET
                    total_input_tokens = total_input_tokens + :input_tokens,
                    total_output_tokens = total_output_tokens + :output_tokens,
                    total_cost_usd = total_cost_usd + :cost,
                    {cycle_col} = {cycle_col} + 1,
                    budget_exhausted = (total_cost_usd + :cost) >= budget_limit_usd
                WHERE trading_date = :d
            """),
            {
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cost": usage.estimated_cost_usd,
                "d": trading_date,
            },
        )

    return await get_or_create_daily_budget(engine, trading_date)


async def create_evolution_cycle(
    engine: AsyncEngine,
    cycle_type: CycleType,
    agents: list[AgentType],
    model: str = HAIKU_MODEL,
) -> int:
    """create a new evolution_cycles row. returns the cycle id."""
    agent_values = [a.value for a in agents]
    trading_date = date.today()

    async with engine.begin() as conn:
        result = await conn.execute(
            text("""
                INSERT INTO evolution_cycles (
                    trading_date, cycle_type, model_used, agents_triggered
                ) VALUES (
                    :trading_date, :cycle_type::cycle_type, :model_used,
                    :agents_triggered::agent_type[]
                )
                RETURNING id
            """),
            {
                "trading_date": trading_date,
                "cycle_type": cycle_type.value,
                "model_used": model,
                "agents_triggered": agent_values,
            },
        )
        row = result.fetchone()
        return row[0]


async def complete_evolution_cycle(
    engine: AsyncEngine,
    cycle_id: int,
    agents_completed: list[AgentType],
    usage: TokenUsage,
    configs_proposed: int = 0,
    configs_promoted: int = 0,
    configs_rejected: int = 0,
) -> None:
    """mark an evolution cycle as completed."""
    agent_values = [a.value for a in agents_completed]

    async with engine.begin() as conn:
        await conn.execute(
            text("""
                UPDATE evolution_cycles SET
                    completed_at = now(),
                    agents_completed = :agents_completed::agent_type[],
                    input_tokens_used = :input_tokens,
                    output_tokens_used = :output_tokens,
                    estimated_cost_usd = :cost,
                    configs_proposed = :configs_proposed,
                    configs_promoted = :configs_promoted,
                    configs_rejected = :configs_rejected
                WHERE id = :cycle_id
            """),
            {
                "cycle_id": cycle_id,
                "agents_completed": agent_values,
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cost": usage.estimated_cost_usd,
                "configs_proposed": configs_proposed,
                "configs_promoted": configs_promoted,
                "configs_rejected": configs_rejected,
            },
        )


async def run_checkin_cycle(engine: AsyncEngine | None = None) -> TokenUsage | None:
    """run a complete check-in cycle: all check-in agents sequentially.

    returns total token usage, or None if budget exhausted.
    """
    own_engine = engine is None
    if engine is None:
        engine = await get_engine()

    try:
        # 1. check budget
        can_proceed, budget = await check_budget(engine)
        if not can_proceed:
            logger.warning(
                f"budget exhausted for {budget.trading_date}: "
                f"${budget.total_cost_usd:.2f} / ${budget.budget_limit_usd:.2f}"
            )
            return None

        # 2. create evolution cycle record
        agents = [a for a, _ in CHECKIN_AGENTS]
        cycle_id = await create_evolution_cycle(engine, CycleType.checkin, agents)
        logger.info(f"starting check-in cycle {cycle_id}")

        # 3. run each check-in agent
        total_usage = TokenUsage()
        agents_completed: list[AgentType] = []

        for agent_type, prompt_name in CHECKIN_AGENTS:
            # re-check budget before each agent
            can_proceed, _ = await check_budget(engine)
            if not can_proceed:
                logger.warning(f"budget exhausted mid-cycle, stopping after {len(agents_completed)} agents")
                break

            try:
                logger.info(f"running {agent_type.value}...")
                usage = await run_checkin_agent(
                    agent_type=agent_type,
                    prompt_name=prompt_name,
                    engine=engine,
                    cycle_id=cycle_id,
                )
                total_usage.input_tokens += usage.input_tokens
                total_usage.output_tokens += usage.output_tokens
                agents_completed.append(agent_type)
            except Exception:
                logger.exception(f"agent {agent_type.value} failed")

        # 4. complete the evolution cycle
        await complete_evolution_cycle(engine, cycle_id, agents_completed, total_usage)

        # 5. update daily budget
        budget = await update_budget(engine, total_usage, CycleType.checkin)
        logger.info(
            f"check-in cycle {cycle_id} complete: "
            f"{len(agents_completed)}/{len(CHECKIN_AGENTS)} agents, "
            f"${total_usage.estimated_cost_usd:.4f}, "
            f"budget: ${budget.total_cost_usd:.2f}/${budget.budget_limit_usd:.2f}"
        )

        return total_usage

    finally:
        if own_engine:
            await close_engine()


async def run_full_pm_cycle(engine: AsyncEngine | None = None) -> TokenUsage | None:
    """run a complete full PM cycle: recommendation agents → PM → backtest → promote/reject.

    returns total token usage, or None if budget exhausted.
    """
    own_engine = engine is None
    if engine is None:
        engine = await get_engine()

    try:
        # 1. check budget
        can_proceed, budget = await check_budget(engine)
        if not can_proceed:
            logger.warning(
                f"budget exhausted for {budget.trading_date}: "
                f"${budget.total_cost_usd:.2f} / ${budget.budget_limit_usd:.2f}"
            )
            return None

        # 2. create evolution cycle record
        agents = [a for a, _ in PM_RECOMMENDATION_AGENTS] + [AgentType.agent_pm]
        cycle_id = await create_evolution_cycle(
            engine, CycleType.full_pm, agents, model=SONNET_MODEL,
        )
        logger.info(f"starting full PM cycle {cycle_id}")

        # 3. run recommendation agents
        total_usage = TokenUsage(model=SONNET_MODEL)
        agents_completed: list[AgentType] = []

        for agent_type, prompt_name in PM_RECOMMENDATION_AGENTS:
            can_proceed, _ = await check_budget(engine)
            if not can_proceed:
                logger.warning(f"budget exhausted mid-cycle, stopping after {len(agents_completed)} agents")
                break

            try:
                logger.info(f"running recommendation {agent_type.value}...")
                usage = await run_recommendation_agent(
                    agent_type=agent_type,
                    prompt_name=prompt_name,
                    engine=engine,
                    cycle_id=cycle_id,
                )
                total_usage.input_tokens += usage.input_tokens
                total_usage.output_tokens += usage.output_tokens
                agents_completed.append(agent_type)
            except Exception:
                logger.exception(f"recommendation agent {agent_type.value} failed")

        # 4. run PM agent
        configs_proposed = 0
        configs_promoted = 0
        configs_rejected = 0

        can_proceed, _ = await check_budget(engine)
        if can_proceed:
            try:
                logger.info("running PM agent...")
                pm_usage, proposed_version_id = await run_pm_agent(
                    engine=engine,
                    cycle_id=cycle_id,
                )
                total_usage.input_tokens += pm_usage.input_tokens
                total_usage.output_tokens += pm_usage.output_tokens
                agents_completed.append(AgentType.agent_pm)

                # 5. handle config proposal
                if proposed_version_id is not None:
                    configs_proposed = 1
                    logger.info(f"PM proposed config version {proposed_version_id}")

                    # update status to backtesting
                    await update_config_status(engine, proposed_version_id, "backtesting")

                    # get backtest data directory
                    data_dir = os.environ.get("BACKTEST_DATA_DIR", DEFAULT_DATA_DIR)

                    if data_dir and os.path.isdir(data_dir):
                        # run backtest validation
                        proposed = await get_config_version(engine, proposed_version_id)
                        if proposed and proposed.get("config_blob"):
                            config_blob = proposed["config_blob"]
                            # get current config metrics for comparison
                            current_config = await get_current_config(engine)
                            current_metrics = None
                            if current_config:
                                current_metrics = {
                                    "sharpe_ratio": current_config.get("backtest_sharpe", 0),
                                    "win_rate": current_config.get("backtest_win_rate", 0),
                                }

                            # find a data file for any configured ticker
                            tickers = config_blob.get("tickers", ["SPY"])
                            ticker = tickers[0] if tickers else "SPY"
                            data_file = os.path.join(data_dir, f"{ticker}.csv")

                            if os.path.exists(data_file):
                                try:
                                    bt_result = run_backtest_validation(
                                        config_blob, data_file, ticker,
                                    )
                                    if bt_result and "metrics" in bt_result:
                                        passed, reasons = validate_backtest_result(
                                            current_metrics,
                                            bt_result["metrics"],
                                        )
                                        if passed:
                                            logger.info("backtest passed, promoting config")
                                            await _promote_config(
                                                engine, proposed_version_id,
                                                config_blob, current_config,
                                                bt_result["metrics"],
                                            )
                                            configs_promoted = 1
                                        else:
                                            logger.warning(f"backtest failed: {reasons}")
                                            await update_config_status(
                                                engine, proposed_version_id, "rejected",
                                            )
                                            configs_rejected = 1
                                    else:
                                        logger.error("backtest returned no metrics, rejecting")
                                        await update_config_status(
                                            engine, proposed_version_id, "rejected",
                                        )
                                        configs_rejected = 1
                                except Exception:
                                    logger.exception("backtest error, rejecting config")
                                    await update_config_status(
                                        engine, proposed_version_id, "rejected",
                                    )
                                    configs_rejected = 1
                            else:
                                logger.warning(
                                    f"no data file at {data_file}, auto-promoting"
                                )
                                await _promote_config(
                                    engine, proposed_version_id,
                                    config_blob, current_config,
                                )
                                configs_promoted = 1
                        else:
                            logger.error("could not read proposed config, rejecting")
                            await update_config_status(
                                engine, proposed_version_id, "rejected",
                            )
                            configs_rejected = 1
                    else:
                        # no data directory configured — auto-promote
                        logger.warning("no BACKTEST_DATA_DIR configured, auto-promoting")
                        proposed = await get_config_version(engine, proposed_version_id)
                        current_config = await get_current_config(engine)
                        config_blob = proposed["config_blob"] if proposed else {}
                        await _promote_config(
                            engine, proposed_version_id,
                            config_blob, current_config,
                        )
                        configs_promoted = 1
                else:
                    logger.info("PM decided to hold steady (no config proposal)")

            except Exception:
                logger.exception("PM agent failed")

        # 6. complete the evolution cycle
        await complete_evolution_cycle(
            engine, cycle_id, agents_completed, total_usage,
            configs_proposed=configs_proposed,
            configs_promoted=configs_promoted,
            configs_rejected=configs_rejected,
        )

        # 7. update daily budget
        budget = await update_budget(engine, total_usage, CycleType.full_pm)
        logger.info(
            f"full PM cycle {cycle_id} complete: "
            f"{len(agents_completed)}/{len(agents)} agents, "
            f"proposed={configs_proposed} promoted={configs_promoted} rejected={configs_rejected}, "
            f"${total_usage.estimated_cost_usd:.4f}, "
            f"budget: ${budget.total_cost_usd:.2f}/${budget.budget_limit_usd:.2f}"
        )

        return total_usage

    finally:
        if own_engine:
            await close_engine()


async def _promote_config(
    engine: AsyncEngine,
    version_id: int,
    config_blob: dict,
    current_config: dict | None,
    backtest_metrics: dict | None = None,
) -> None:
    """promote a config version: update status, compute diff, write changelog."""
    await update_config_status(
        engine, version_id, "promoted",
        backtest_results=backtest_metrics,
    )

    # compute diff and write changelog
    old_config = current_config.get("config", {}) if current_config else {}
    changes = compute_config_diff(old_config, config_blob)
    if changes:
        await write_changelog_entries(
            engine, version_id, changes,
            changed_by="agent_pm",
        )
        logger.info(f"wrote {len(changes)} changelog entries for config version {version_id}")


async def _scheduled_checkin() -> None:
    """wrapper for scheduled check-in cycles that catches and logs exceptions."""
    try:
        await run_checkin_cycle()
    except Exception:
        logger.exception("scheduled check-in cycle failed")


async def _scheduled_pm_cycle() -> None:
    """wrapper for scheduled PM cycles that catches and logs exceptions."""
    try:
        await run_full_pm_cycle()
    except Exception:
        logger.exception("scheduled PM cycle failed")


async def run_scheduled() -> None:
    """run agents on a market-hours-aware cron schedule.

    check-ins at 10:00, 12:00, 14:00 ET (weekdays).
    full PM cycle at 15:30 ET (weekdays).
    runs until SIGTERM or SIGINT.
    """
    from apscheduler.schedulers.asyncio import AsyncIOScheduler
    from apscheduler.triggers.cron import CronTrigger

    scheduler = AsyncIOScheduler(timezone="US/Eastern")

    scheduler.add_job(
        _scheduled_checkin,
        trigger=CronTrigger(hour="10,12,14", minute=0, day_of_week="mon-fri"),
        id="checkin",
        name="check-in cycle",
    )
    scheduler.add_job(
        _scheduled_pm_cycle,
        trigger=CronTrigger(hour=15, minute=30, day_of_week="mon-fri"),
        id="pm_cycle",
        name="full PM cycle",
    )

    shutdown_event = asyncio.Event()

    loop = asyncio.get_running_loop()
    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, shutdown_event.set)

    scheduler.start()
    logger.info("scheduler started — check-ins at 10/12/14 ET, PM at 15:30 ET")

    await shutdown_event.wait()

    logger.info("shutdown signal received, stopping scheduler")
    scheduler.shutdown(wait=True)
    await close_engine()
    logger.info("scheduler stopped")


async def run_scheduled_checkins(
    interval_minutes: int = 15,
    max_cycles: int | None = None,
) -> None:
    """run check-in cycles on a schedule.

    runs indefinitely (or up to max_cycles) with the given interval.
    stops when budget is exhausted.
    """
    engine = await get_engine()
    cycles_run = 0

    try:
        while max_cycles is None or cycles_run < max_cycles:
            usage = await run_checkin_cycle(engine)
            cycles_run += 1

            if usage is None:
                logger.info("budget exhausted, stopping scheduled check-ins")
                break

            if max_cycles is None or cycles_run < max_cycles:
                logger.info(f"sleeping {interval_minutes} minutes until next check-in...")
                await asyncio.sleep(interval_minutes * 60)
    finally:
        await close_engine()


def main() -> None:
    """entry point for running the orchestrator."""
    import argparse

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
    )

    parser = argparse.ArgumentParser(description="trading system orchestrator")
    parser.add_argument(
        "--mode", choices=["checkin", "once", "pm", "scheduled"], default="once",
        help=(
            "'once' runs a single check-in cycle, "
            "'checkin' runs check-ins on a sleep loop, "
            "'pm' runs a full PM cycle, "
            "'scheduled' runs on market-hours cron schedule"
        ),
    )
    parser.add_argument(
        "--interval", type=int, default=15,
        help="minutes between check-in cycles (default: 15, checkin mode only)",
    )
    args = parser.parse_args()

    if args.mode == "once":
        asyncio.run(run_checkin_cycle())
    elif args.mode == "pm":
        asyncio.run(run_full_pm_cycle())
    elif args.mode == "scheduled":
        asyncio.run(run_scheduled())
    else:
        asyncio.run(run_scheduled_checkins(interval_minutes=args.interval))


if __name__ == "__main__":
    main()
