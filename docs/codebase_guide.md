# codebase architecture guide

this document maps out the key files, areas, and entry points of the galactic trading firm codebase.

## high-level architecture

the system uses a two-layer model:

- **fast layer (rust)**: execution engine processing ticks in real-time using `ta` crate (v0.5) for indicator computation. targets sub-ms latency per tick. handles position management, scoring, and trade execution.
- **slow layer (python)**: evolution agents via claude agent SDK. sonnet 4.5 for full PM cycles (1-3x daily), haiku 4.5 for lightweight check-ins (more frequent). orchestrator enforces a $5/day budget cap.

```
                         ┌─────────────────────────┐
                         │    postgres 16           │
                         │  (config, trades, memos) │
                         └────────┬────────────────┘
                                  │
                 ┌────────────────┼────────────────┐
                 │                │                 │
    ┌────────────▼──────┐  ┌─────▼──────┐  ┌──────▼──────────┐
    │   paper_trader    │  │  backtest  │  │  python agents  │
    │  (data_feed bin)  │  │   CLI      │  │  (orchestrator) │
    │                   │  │            │  │                  │
    │ alpaca websocket  │  │ CSV replay │  │ check-in agents │
    │ → candle agg      │  │ → engine   │  │ PM agent        │
    │ → engine on_tick  │  │ → metrics  │  │ → propose config│
    │ → trade writer    │  │ → report   │  │ → backtest gate │
    │ → config watcher  │  └────────────┘  │ → promote/reject│
    └───────────────────┘                  └─────────────────┘
```

data flows through:
1. market data (alpaca websocket or demo feed) → `MarketStateBuilder` aggregates candles
2. `TradingEngine::on_tick()` computes indicator scores → composite score → action evaluation
3. trades written to postgres via `TradeWriter`
4. python agents read trade data + memos → propose config mutations → backtest validates → promote or reject
5. `ConfigWatcher` polls postgres for promoted configs → hot-reloads engines

---

## rust crates

the workspace is defined in `Cargo.toml` with 6 crates under `crates/`.

### `types` — shared types and traits

**purpose**: defines all core types, traits, and interfaces shared across the rust layer.

**key files**:
- `src/lib.rs` — re-exports all public modules
- `src/market.rs` — `Candle`, `MarketState`, `Timescale` enum (OneMinute, FiveMinute, OneHour, OneDay, OneMonth)
- `src/indicator.rs` — `Indicator` trait (`fn compute(&self, market: &MarketState) -> Option<IndicatorOutput>`), `IndicatorConfig`, `IndicatorOutput`
- `src/action.rs` — `Action` trait (`fn evaluate(...) -> ActionSignal`), `ActionConfig`, `ActionPhase` (Entry/Monitor/Exit/Sizing), `ActionSignal` enum, `Position`, `TradeDirection`, `ExitReason`
- `src/scoring.rs` — `ScoringConfig`, `TimescaleScores`, `AggregationMethod` enum (WeightedSum, WeightedSumWithGates, MinScore)
- `src/config.rs` — `StrategyConfig`, `SessionConfig`
- `src/registry.rs` — `IndicatorRegistry`, `ActionRegistry`, `ToolBelt`
- `src/tick_result.rs` — `TickResult { scores, event }`, `TickEvent` enum (Nothing, PositionOpened, PositionClosed)
- `src/adapter.rs` — adapters between types
- `src/proposal.rs` — proposal system types
- `src/test_fixtures.rs` — shared test helpers

**public API**: all types are re-exported from `lib.rs` for ergonomic imports (`use types::{Candle, MarketState, ...}`).

### `indicators` — indicator implementations and registry

**purpose**: wraps ta-rs indicators and builds composable indicators. provides a factory-based registry.

**key files**:
- `src/lib.rs` — `default_indicator_registry()` (registers all 24 indicators), `build_indicators()` (config → instance map)
- `src/native/` — 13 native ta-rs wrappers: `rsi`, `ema`, `sma`, `macd`, `bollinger`, `atr`, `keltner`, `stochastic_fast`, `stochastic_slow`, `cci`, `mfi`, `roc`, `obv`
- `src/composable/` — 11 composable indicators: `bollinger_pct_b`, `bollinger_bandwidth`, `adx`, `supertrend`, `vwap_distance`, `stochastic_rsi`, `williams_r`, `donchian`, `dema`, `ttm_squeeze`, `awesome_oscillator`
- `src/custom/` — placeholder for future from-scratch indicators
- `src/aggregation.rs` — `compute_timescale_scores()` aggregates per-indicator scores into per-timescale scores
- `src/helpers.rs` — shared math helpers

**pattern**: each indicator is stateless — `compute()` creates a fresh ta-rs indicator and replays the full candle window from `MarketState`. all scores normalize to -1.0..+1.0.

### `engine` — trading engine core

**purpose**: orchestrates the tick loop — scoring pipeline, position management, and action evaluation.

**key files**:
- `src/lib.rs` — re-exports `TradingEngine`, `PositionManager`, `TradeRecord`, `compute_composite`
- `src/tick_loop.rs` — `TradingEngine` struct and `on_tick(&mut self, market: &MarketState) -> TickResult`. wraps each indicator `compute()` in `catch_unwind` (no-panic constraint). evaluates actions in order: monitor → exit (if position) or entry → sizing (if no position).
- `src/scoring.rs` — `compute_composite()` implements three aggregation methods (weighted sum, weighted sum with hard gates, min score). hard gates: if any gated timescale score ≤ 0, composite floors to 0.
- `src/position.rs` — `PositionManager` (open/update/close lifecycle), `TradeRecord` (completed trade data)
- `src/config.rs` — config loading from postgres (promoted config_versions row)
- `src/main.rs` — minimal binary that loads config and prints info. not the live trading entry point.

**binary**: `cargo run -p engine` — config info display only. the live engine runs via `data_feed`.

### `data_feed` — paper trading engine

**purpose**: connects to market data (alpaca or demo), runs engines per ticker, manages live sessions, and writes trades to postgres. this is the primary runtime binary.

**key files**:
- `src/main.rs` — **`paper_trader` binary**. main `tokio::select!` loop handling: bar events from data feed, config reload timer (60s), ctrl-c, SIGTERM. builds one `LiveSession` + `MarketStateBuilder` per ticker.
- `src/alpaca_feed.rs` — `AlpacaFeed` struct. websocket bar streaming via `apca` crate. `fetch_historical_bars()` for lookback seeding, `stream_bars()` for live data.
- `src/candle_aggregator.rs` — `CandleAggregator` clock-aligns 5-min/hourly candle boundaries from 1-min bars.
- `src/market_state.rs` — `MarketStateBuilder` wraps `CandleAggregator` with VWAP computation. `on_bar()` → `MarketState`.
- `src/live_session.rs` — `LiveSession` wraps `TradingEngine`. stashes entry scores on `PositionOpened`, pairs with exit scores on `PositionClosed` to produce `TradeWithScores`.
- `src/broker.rs` — `SimulatedBroker` (configurable slippage in bps). alpaca paper broker mode also supported.
- `src/trade_writer.rs` — `TradeWriter` persists completed trades + indicator snapshots to postgres.
- `src/config_watcher.rs` — `ConfigWatcher` polls postgres for new promoted configs. `try_build_engine()` rebuilds with fallback to previous config on failure.
- `src/config_loader.rs` — `load_config()` reads the latest promoted `StrategyConfig` from postgres.
- `src/tui.rs` — feature-gated (`--features tui`) ratatui terminal dashboard. `DashboardState`, `TickerState`, `PositionDisplay`, `TradeLogEntry`. runs on a separate thread.

**binary**: `cargo run -p data_feed` (headless) or `cargo run -p data_feed --features tui` (TUI). supports `--demo` flag for synthetic random-walk data without alpaca credentials.

**environment variables**:
- `DATABASE_URL` — postgres connection string (required)
- `APCA_API_KEY_ID` / `APCA_API_SECRET_KEY` — alpaca API credentials (optional for demo mode)
- `BROKER_MODE` — `simulated` (default) or `alpaca_paper`
- `RUST_LOG` — tracing filter (default: `info`)

### `backtest` — historical replay and reporting

**purpose**: replays historical candle data through the trading engine and produces performance metrics.

**key files**:
- `src/lib.rs` — re-exports all public types and functions
- `src/replay.rs` — `BacktestConfig`, `BacktestData`, `run_backtest()` (drives `TradingEngine` over candle series), `load_candles_from_csv()` (CSV parser)
- `src/report.rs` — `BacktestMetrics` (P&L, sharpe, max drawdown, win rate, profit factor), `BacktestResult`, `compute_metrics()`, `compare_configs()`, `ConfigComparison`, `to_json()`, `trades_to_csv()`, `summary()`, `EquityPoint`, `ExitReasonBreakdown`
- `src/main.rs` — CLI binary: `backtest --config <path> --data <path> --ticker <name>`. reads JSON config + CSV candle data, runs backtest, outputs JSON report to stdout.

**binary**: `cargo run -p backtest -- --config config.json --data SPY.csv --ticker SPY`

---

## python agent layer

located in `agents/`. installed as an editable package (`pip install -e ".[dev]"`). venv at `agents/.venv`.

### entry points

**`agents/orchestrator.py`** — CLI with 4 modes:
- `--mode once` (default): runs a single check-in cycle
- `--mode checkin`: runs check-ins on a sleep loop (`--interval` minutes)
- `--mode pm`: runs a single full PM cycle
- `--mode scheduled`: market-hours-aware cron via apscheduler — check-ins at 10:00/12:00/14:00 ET (weekdays), full PM at 15:30 ET

run with: `python -m agents.orchestrator --mode <mode>`

### agent framework

**`agents/agent_base.py`** — claude agent SDK integration:
- `_run_agent_sdk()` — shared loop: loads system prompt, builds MCP tools, runs `query()` async generator
- `run_checkin_agent()` — haiku 4.5 with `CHECKIN_TOOLS` (read-only trade data + `write_observation_memo`)
- `run_recommendation_agent()` — sonnet 4.5 with `RECOMMENDATION_TOOLS` (read-only + `write_recommendation_memo`)
- `run_pm_agent()` — sonnet 4.5 with `PM_TOOLS` (read-only + `get_checkin_memos` + `get_recommendation_memos` + `propose_config_mutation` + `write_pm_memo`)
- tool execution dispatched through `execute_tool()` which routes to the appropriate `agents.tools.*` function

**`agents/models.py`** — pydantic models: `AgentType`, `CycleType`, `MemoType`, `TokenUsage` (with model-aware pricing), `AgentMemo`, `DailyBudget`, `TradeRecord`, `DailyPerformance`, `ChangelogEntry`, `ValidationThresholds`

**`agents/db.py`** — async sqlalchemy engine management (`get_engine()` / `close_engine()`)

### tool implementations

| file | purpose | access |
|------|---------|--------|
| `tools/sql_queries.py` | read trade data: `get_recent_trades()`, `get_daily_performance()`, `get_performance_by_exit_reason()`, `get_config_changelog()`, `get_checkin_memos_since_last_pm()`, `get_recommendation_memos()` | read-only |
| `tools/memo_writer.py` | `write_memo()` — persist agent memos to `agent_memos` table | write-only |
| `tools/config_ops.py` | `get_current_config()`, `propose_config()`, `update_config_status()`, `get_config_version()`, `write_changelog_entries()` | read + write |
| `tools/config_diff.py` | `compute_config_diff()` — compares two config blobs, produces atomic change dicts for changelog | pure function |
| `tools/backtest_runner.py` | `run_backtest_validation()` — shells out to rust backtest CLI. `validate_backtest_result()` — checks metrics against thresholds | read-only + subprocess |

### prompt templates

7 system prompts in `agents/prompts/`:

| file | agent | role |
|------|-------|------|
| `checkin_1min.md` | 1-minute check-in | observation-only, high-frequency signal analysis |
| `checkin_5min.md` | 5-minute check-in | observation-only, swing signal analysis |
| `checkin_hourly.md` | hourly check-in | observation-only, regime analysis |
| `recommend_1min.md` | 1-minute recommendation | parameter change suggestions for fast signals |
| `recommend_5min.md` | 5-minute recommendation | parameter change suggestions for swing signals |
| `recommend_hourly.md` | hourly recommendation | parameter change suggestions for regime detection |
| `agent_pm.md` | portfolio manager | reads all memos, proposes or holds config changes |

---

## database and migrations

postgres 16, managed by sqlx migrations in `migrations/`. full schema reference at `docs/data_model.sql`.

### migrations (in order)

1. `_create_enums` — custom postgres enums: `timescale`, `trade_direction`, `exit_reason`, `agent_type`, `memo_type`, `cycle_type`, `mutation_status`, `trade_event_type`, `change_category`
2. `_create_config_versions` — immutable append-only config store with status lifecycle
3. `_create_trades` — completed trade records with entry/exit scores per timescale
4. `_create_indicator_snapshots` — per-tick indicator values for analysis
5. `_create_agent_memos` — structured agent observations and recommendations
6. `_create_evolution_cycles` — tracks each agent cycle (check-in or PM) with cost
7. `_create_config_changelog` — atomic config change records
8. `_create_daily_budget` — per-day token usage and cost tracking
9. `_create_analysis_views` — 6 views for agent consumption (see below)
10. `_seed_initial_config` — initial promoted `StrategyConfig`

### key tables

| table | purpose |
|-------|---------|
| `config_versions` | immutable config blobs with status: proposed → backtesting → validated → promoted (or rejected) |
| `trades` | completed trades with P&L, hold duration, exit reason, and per-timescale entry/exit scores |
| `indicator_snapshots` | raw indicator values per tick (for analysis) |
| `agent_memos` | structured memos from agents (observations + recommendations) |
| `evolution_cycles` | tracks each check-in or PM cycle with agents triggered/completed and cost |
| `config_changelog` | atomic config change records (what changed, old/new values, changed_by) |
| `daily_budget` | per-day budget tracking (cost, cycle counts, exhaustion flag) |

### analysis views

| view | purpose |
|------|---------|
| `daily_performance` | per-day per-ticker trade stats (count, win rate, P&L, avg hold) |
| `performance_by_exit_reason` | trade stats grouped by exit reason |
| `score_interaction_analysis` | win rate by entry score buckets (1min × 5min × hourly) |
| `recent_agent_signals` | agent memos from last 30 days |
| `checkin_memos_since_last_pm` | observation memos since the last completed PM cycle |
| `daily_cost_summary` | budget usage per day with percentage |

---

## deployment

### docker compose

3 services defined in `docker-compose.yml`:

| service | image | role |
|---------|-------|------|
| `postgres` | `postgres:16` | database, port 5433→5432, healthcheck, persistent volume |
| `paper_trader` | `Dockerfile.paper_trader` | rust execution engine, depends on postgres healthy |
| `agents` | `Dockerfile.agents` | python orchestrator in scheduled mode, depends on postgres healthy |

### dockerfiles

**`Dockerfile.paper_trader`** — multi-stage build:
- builder: `rust:1.82-bookworm`, builds release binary with TUI feature
- runtime: `debian:bookworm-slim` + tmux + ca-certificates. copies `paper_trader` binary and entrypoint script.

**`Dockerfile.agents`** — `python:3.12-slim-bookworm` with build-essential. installs agents package, runs `python -m agents.orchestrator --mode scheduled`.

### entrypoint

**`docker/paper_trader_entrypoint.sh`** — wraps `paper_trader` in a tmux session for TUI access over SSH:
```
docker exec -it <container> tmux attach -t trader
```
`exec tmux` propagates SIGTERM. `remain-on-exit on` keeps the session visible after exit.

### environment variables

| variable | used by | description |
|----------|---------|-------------|
| `DATABASE_URL` | both | postgres connection (`postgres://` for rust, `postgresql+asyncpg://` for python) |
| `APCA_API_KEY_ID` | paper_trader | alpaca API key |
| `APCA_API_SECRET_KEY` | paper_trader | alpaca API secret |
| `BROKER_MODE` | paper_trader | `simulated` (default) or `alpaca_paper` |
| `RUST_LOG` | paper_trader | tracing log level filter |
| `ANTHROPIC_API_KEY` | agents | claude API key |
| `BACKTEST_DATA_DIR` | agents | directory of historical CSV files for backtest validation |

---

## build and test

### rust

```bash
cargo build --workspace                      # build all crates
cargo build -p data_feed --features tui      # build paper_trader with TUI
cargo test --workspace                       # run all 268 tests
cargo test -p types                          # single crate
cargo test -p indicators -- ema              # single test by name
cargo clippy --workspace -- -D warnings      # lint
```

### database

```bash
docker-compose up -d                         # start postgres
sqlx migrate run                             # apply migrations
```

### python

```bash
cd agents && pip install -e ".[dev]"         # install with dev deps
python -m pytest tests/ -v                   # run all 70 tests
ruff check agents/ tests/                    # lint
python -m agents.orchestrator --mode once    # single check-in cycle
```

---

## config lifecycle

configs follow an immutable append-only model. every change creates a new `config_versions` row.

```
proposed → backtesting → validated → promoted
                                  ↘ rejected
```

### flow

1. **proposal**: PM agent calls `propose_config_mutation` tool → creates a `config_versions` row with `status = 'proposed'`
2. **backtest gate**: orchestrator updates status to `backtesting`, shells out to the rust `backtest` CLI binary with the proposed config and historical CSV data
3. **validation**: `validate_backtest_result()` checks metrics against thresholds (sharpe, win rate). if backtest data isn't available, the config is auto-promoted.
4. **promotion**: status updated to `promoted`. `compute_config_diff()` compares old vs new config and writes atomic changes to `config_changelog`.
5. **hot-reload**: `ConfigWatcher` in paper_trader polls postgres every 60s. on new promoted config, rebuilds engines per ticker with fallback to previous config on failure. force-closes open positions before swapping.
6. **rollback**: promote an older version (no special mechanism — just promote the desired config).

### what agents can change

- indicator knobs (period, thresholds, weights)
- enable/disable indicator and action instances
- scoring weights and aggregation method
- entry/exit thresholds
- session rules (max hold, max concurrent positions)
- agents **cannot** add new tool *types* — only reconfigure instances of existing ones. new types require human-gated PRs (phase 8).
