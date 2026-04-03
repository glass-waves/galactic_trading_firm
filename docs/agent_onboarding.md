# agent onboarding guide

essential reading for a fresh claude code agent working on this project. read in order — each section builds on the previous.

**last updated:** 2026-04-02

---

## 1. start here: project architecture

| doc | what it gives you | read time |
|-----|-------------------|-----------|
| `CLAUDE.md` (project root) | **the single source of truth.** complete file map of every crate, every file, every test. architecture overview, build commands, key constraints, data flow diagram, contributor patterns. | 15 min |
| `docs/system_architecture.mermaid` | visual mermaid diagram of component relationships | 2 min |

`CLAUDE.md` is exhaustive and current. if you read nothing else, read this.

---

## 2. understand the trading logic

read these to understand what the system actually does:

| doc | what it gives you |
|-----|-------------------|
| `docs/research_synthesis.md` | **why** every architecture decision was made. maps 44 papers to specific code choices (agent consolidation rationale, indicator selection, scoring pipeline design). read sections 1-4 at minimum. |
| `docs/entry_windows_analysis.md` | how the entry window system works, research-backed analysis of W1-W5 windows. explains the "gated entry" concept and why certain windows were adopted/rejected. |
| `../galactic_trading_agents/agents-ts/prompts/agent_pm.md` | the archived PM agent's system prompt. contains the most detailed explanation of every action's tuning ranges, what's been tested, and what failed. sections 4 (action catalog) and 5 (sizing) are essential for understanding position management. |

### key concepts to internalize

- **scoring pipeline**: indicators → per-timescale weighted sum → composite score → hard gates → entry/exit decisions. see `crates/engine/src/scoring.rs`.
- **entry windows**: named entry conditions (W1: 5m thrust, W4: strong core, W5: candle reversal) that gate when the engine can open positions. configured via CLI flags `--w1-*`, `--w4-*`, `--w5-*`.
- **tool belt model**: indicators (sensing) and actions (doing) are pluggable modules loaded from config at runtime. adding new ones follows a registration pattern — see "key patterns for contributors" in `CLAUDE.md`.

---

## 3. understand the backtest system

the backtest is the primary tool for validating changes. you'll use it constantly.

| resource | what it gives you |
|----------|-------------------|
| `docs/cli_usage_guide.md` | complete CLI reference for backtest, paper_trader, orchestrator. all flags documented. |
| `docs/testing_strategy.md` | standardized process: unit tests → single-day backtest → full-year sweep → 4-year cross-validation. |
| `scripts/backtest_year.sh` | the workhorse script. runs every trading day in a year, outputs CSV. supports `--compound` for day-to-day capital accumulation and `--capital N` for custom starting capital. |
| `scripts/sweep_params.sh` | parameter sweep automation. runs `backtest_year.sh` with varied CLI overrides to find optimal values. |

### critical backtest details

- **next-bar execution**: orders fill at the next bar's open, not the signal bar's close. realistic slippage.
- **cost model**: `--slippage-bps 2.0 --half-spread 0.005` is standard. applied via `BacktestCostConfig`.
- **compounding**: `--compound` flag in `backtest_year.sh` carries ending capital forward day-to-day. without it, each day resets to `--capital` (default $10k). year-to-year compounding requires chaining year scripts manually.
- **CLI overrides**: `ConfigOverrides` struct in `crates/backtest/src/main.rs` — dozens of flags to A/B test parameter changes without touching the DB config. key ones: `--sizing-fraction`, `--entry-threshold`, `--no-vol-sizing`, `--max-capital-deployed-pct`, `--ticker-override "NVDA:entry_threshold=0.35"`.
- **max 2 concurrent backtests** to avoid alpaca API rate limiting.

---

## 4. current config state

### promoted DB config (v89, promoted 2026-03-05)

the live/promoted config in postgres. defined in `migrations/20260305000002_v89_phase3_winner.sql`.

key parameters:
- **sizing**: `volatility_scaled` at `base_fraction: 0.05` (5%), `baseline_atr: 1.0`
- **entry**: `entry_threshold: 0.58`, `avoid_first_minutes: 60`, `no_new_entries_after: 15:30`
- **exit**: ATR trailing (7x), fixed stop (2.5%), max hold (90 min adaptive), session close (15:55)
- **session**: `max_capital_deployed_pct: 0.10`, `max_concurrent_positions: 1`, `entry_cooldown_ms: 30000`
- **tickers**: SPY, QQQ, AAPL, NVDA, MSFT

### recent backtest results (not yet in DB config)

as of 2026-04-01, backtests were run with full kelly sizing (~36% base_fraction) and day-to-day compounding. these changes are **not yet persisted in any migration or config file** — they were tested via CLI overrides (`--sizing-fraction` + `--compound`).

| variant | 2022 | 2023 | 2024 | 2025 | 4yr total |
|---------|------|------|------|------|-----------|
| baseline (5%, no compound) | +$215 | +$158 | +$114 | +$94 | +$581 |
| full kelly (~36%, no compound) | +$4,316 | +$3,197 | +$2,415 | +$1,964 | +$11,892 |
| full kelly + compound | +$5,340 | +$5,715 | +$5,718 | +$5,767 | +$22,540 |

these results use the v10 config (W5 candle reversal window, tuned W1/W4 params) on SPY/QQQ/AAPL/MSFT (no NVDA).

### what hasn't been promoted yet

1. **full kelly sizing** — `base_fraction` 0.05 → ~0.36 in DB config
2. **max_capital_deployed_pct** — currently 0.10 (10%), would need to increase to accommodate 36% positions
3. **compounding support in the engine** — currently only in shell scripts, not in the rust engine or DB config
4. **v10 entry windows** (W5 candle reversal, W1 lead_by=0.10, W5 composite_min=0.30) — these are only applied via CLI overrides, not in the promoted config

> **note on config creation:** configs are now created manually via new SQL migrations inserted into `config_versions`, or via the backtest CLI. the typescript agent layer that previously proposed configs has been archived to `../galactic_trading_agents/`.

---

## 5. tuning history and context

| doc | what it gives you |
|-----|-------------------|
| `docs/backtest_tuning_log.md` | complete tuning history: phase 1 baseline (4-year validation), phase 2 candle pattern (A/B test results with feature isolation). all P&L numbers are post-sizing-bug-fix. |
| `docs/pre-bug-fix.md` | archived tuning from before the position sizing bug fix. **absolute P&L numbers are invalid**, but timing insights and indicator findings remain directionally correct. |
| `docs/next_steps.md` | the plan that led to v10: candle pattern windows, existing window re-tune, per-ticker overrides. dated 2026-03-29. |
| `docs/proposed_additions.md` | 13 features + 5 bug fixes that were implemented in March 2026. reference for what was added and why. |

### sizing bug context

a critical bug was fixed 2026-03-21: `position_size = fraction * capital` was treated as share count instead of dollar amount, inflating P&L by 100-400x. all results before this date in the tuning log are post-fix re-runs. the bug and fix are in `crates/engine/src/tick_loop.rs` (dollars → shares conversion).

---

## 6. agent layer (archived)

the typescript agent layer (`agents-ts/`) was archived to `../galactic_trading_agents/` on 2026-04-02, replaced by anthropic claude code scheduled jobs. agent-specific database tables remain in migrations for compatibility. see `docs/archive/scheduled_jobs_design.md` for the replacement design.

---

## 7. code entry points by task

### "i need to run a backtest"

```bash
# single day
cargo run -p backtest --release -- --date 2025-10-06 --lookback-days 3

# full year with CSV output
./scripts/backtest_year.sh 2025 > data/my_test_2025.csv

# full year with compounding
./scripts/backtest_year.sh 2025 --compound --capital 10000 > data/my_test_2025.csv

# with overrides
./scripts/backtest_year.sh 2025 --sizing-fraction 0.36 --compound > data/full_kelly_2025.csv
```

### "i need to add/modify an indicator"

1. `crates/indicators/src/{native,composable,custom}/` — implementation
2. `crates/indicators/src/lib.rs` — registry registration
3. `crates/indicators/tests/` — unit tests
4. `crates/types/src/indicator.rs` — `Indicator` trait definition

### "i need to add/modify an action"

1. `crates/actions/src/{entry,exit,monitor,sizing}/` — implementation
2. `crates/actions/src/lib.rs` — registry registration
3. `crates/actions/tests/` — unit tests
4. `crates/types/src/action.rs` — `Action` trait definition

### "i need to modify the scoring pipeline"

1. `crates/engine/src/scoring.rs` — composite score computation
2. `crates/engine/src/tick_loop.rs` — `TradingEngine::on_tick()` main loop
3. `crates/types/src/scoring.rs` — scoring config types

### "i need to modify the backtest"

1. `crates/backtest/src/main.rs` — CLI, `ConfigOverrides`, CSV output
2. `crates/backtest/src/replay.rs` — tick-by-tick replay loop, deferred fills, equity tracking
3. `crates/backtest/src/report.rs` — metrics computation, daily returns, sharpe, drawdown
4. `scripts/backtest_year.sh` — multi-day orchestration, compounding

### "i need to modify the paper trader"

1. `crates/data_feed/src/main.rs` — async main, per-ticker sessions
2. `crates/data_feed/src/candle_aggregator.rs` — 1m → 5m/1h aggregation
3. `crates/data_feed/src/market_state.rs` — VWAP, market state snapshots
4. `crates/data_feed/src/broker.rs` — simulated/alpaca broker
5. `crates/data_feed/src/config_watcher.rs` — hot-reload from DB

### "i need to create a new promoted config"

1. write a new migration in `migrations/` inserting into `config_versions` with status `'promoted'`
2. the config blob is JSONB — see existing migrations for the schema
3. `configs/default_v1.json` and `configs/v3_post_analysis.json` are reference configs (may be stale)
4. the latest promoted config is in `migrations/20260305000002_v89_phase3_winner.sql`

---

## 8. docs that are outdated or low-priority

| doc | status | notes |
|-----|--------|-------|
| `docs/codebase_guide.md` | **outdated** | references "python agents". `CLAUDE.md` supersedes this entirely. |
| `docs/development_spec.md` | **historical** | original implementation roadmap. phases 1-7 are all done. |
| `docs/action_items.md` | **completed** | all 11 items from research synthesis were implemented 2026-03-02. |
| `docs/hardening_plan.md` | **future** | 3-tier safety guardrails for real capital. not yet implemented. |
| `docs/tool_belt_catalog.md` | **reference only** | 163 indicators + 124 actions catalog. useful when considering new indicator/action types. |
| `docs/ta_rs_implementation_map.md` | **reference only** | maps indicators to ta-rs functions. useful when implementing new native indicators. |
| `docs/intraday-trading-research.md` | **reference only** | 32-paper bibliography. read if you need research backing for a specific approach. |
| `docs/archive/*` | **archived** | agent-specific docs (scheduled_jobs_design, agentic_system_recommendations, multi-agent-research) |

---

## 9. key gotchas

1. **dual entry_threshold trap**: the engine reads `action.params.entry_threshold` on the score_threshold_entry action, NOT `scoring.entry_threshold`. CLI `--entry-threshold` updates both. manual jq edits must update both or entries won't trigger.

2. **no-panic constraint**: the tick loop must never panic. all indicator `compute()` calls are wrapped in `catch_unwind`. if you add new indicators, ensure they don't panic on edge cases.

3. **config instance_ids**: use the exact IDs: `sizing_vol`, `sizing_fixed`, `hard_stop`, `trailing_stop_atr`, `max_hold`, `ofi_5min`, `mom_persist_5min`, `candle_5min`. mismatched IDs silently create new instances.

4. **ta crate v0.5**: pinned version. `Next<f64>` for close-price indicators, `Next<&DataItem>` for OHLCV. indicators replay the full candle window on each `compute()` (stateless by design).

5. **backtest parallelism**: max 2 concurrent backtests to avoid alpaca API rate limiting.

6. **serde_json in dev-dependencies**: test files using `json!` macro need `serde_json` in `[dev-dependencies]`, not just `[dependencies]`.

7. **f64::MAX not INFINITY**: use `f64::MAX` for JSON-serializable values (infinity breaks JSON serialization).

8. **ExitReason derives**: `Hash + Eq` derives are required (used as HashMap key in metrics breakdown).

---

## 10. recent changes

- **v10 (2026-04-02)**: candle patterns (hammer/shooting star, evening star, confirmed engulfing), W5 entry window, `--sizing-fraction`/`--compound`/`--capital` CLI flags, entry_reason tracking in CSV output
- **agent layer archived (2026-04-02)**: `agents-ts/` moved to `../galactic_trading_agents/`, replaced by claude code scheduled jobs
