# CLAUDE.md

this file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## project overview

adaptive multi-timescale intraday trading system with two-layer architecture:

- **fast layer (rust)**: execution engine processing ticks in real-time using `ta` crate (ta-rs v0.5) for indicator computation. targets sub-ms latency per tick.
- **slow layer (typescript)**: evolution agents via claude agent sdk. 2-agent cycles: analysis agent + PM agent. opus 4.6 for end-of-day full PM cycle (1x daily), sonnet 4.6 for mid-day analysis (1x daily). orchestrator enforces a $5/day budget cap under anthropic tier 1 ($100/month).

instruments: SPY, QQQ, 3-5 liquid mega-caps. intraday only, no overnight holds.

**agent schedule:**
- 12:30 ET — mid-day analysis (sonnet 4.6, observation only)
- 15:30 ET — full PM cycle (opus 4.6, analysis + PM with config authority)

## repository structure

```
galactic_trading_firm/
├── Cargo.toml                    # rust workspace root
├── docker-compose.yml            # postgres 16 + paper_trader + agents-ts
├── crates/
│   ├── types/                    # shared types — Candle, Indicator/Action traits, StrategyConfig
│   ├── indicators/               # indicator implementations (phase 2)
│   │   └── src/{native/, composable/, custom/}
│   ├── actions/                  # action implementations (phase 3)
│   │   └── src/{entry/, exit/, monitor/, sizing/}
│   ├── engine/                   # execution engine binary
│   │   └── src/{main.rs, config.rs, scoring.rs, position.rs}
│   ├── backtest/                 # historical replay + reports (phase 4)
│   └── data_feed/                # paper trading binary (phase 7)
├── agents-ts/                    # typescript agent layer
│   ├── src/
│   │   ├── orchestrator.ts       # scheduler, budget tracking
│   │   ├── agent-base.ts         # shared agent sdk invocation
│   │   ├── models.ts             # type definitions
│   │   └── tools/                # agent tools (sql queries, config ops, memo writer)
│   ├── prompts/                  # system prompts per agent/timescale
│   └── tests/
├── migrations/                   # sqlx migrations (from data_model.sql)
└── docs/                         # design artifacts and reference docs
```

## build and test commands

```bash
# rust
cargo build --workspace
cargo test --workspace
cargo test -p types                  # single crate
cargo test -p indicators -- ema      # single test by name
cargo clippy --workspace -- -D warnings

# database (requires docker)
docker-compose up -d
sqlx migrate run

# typescript agents
cd agents-ts && npm install
npx vitest run                       # run tests
npx tsc                              # type check
```

## implementation phases

1. **foundation** (done) — workspace setup, types crate, db schema, config loading
2. **indicator engine** (done) — 24 indicators (13 native ta-rs + 11 composable + 4 custom), registry, aggregation
3. **action engine & scoring** (done) — scoring pipeline (weighted sum + hard gates + agreement + dynamic fusion), 8 actions (entry/exit/monitor/sizing), tick loop
4. **backtest engine** (done) — historical replay, metrics (P&L, sharpe, drawdown, win rate), JSON/CSV export, config comparison
5. **agent layer** (done) — typescript orchestrator, analysis agent (sonnet 4.6 mid-day), agent tools, budget enforcement
6. **full evolution loop** (done) — PM agent (opus 4.6) with config mutation authority, backtest validation gates, config diff/changelog, beliefs/CVRF, hot-reload
7. **paper trading** (done) — alpaca websocket, candle aggregation, simulated broker, trade writer, config hot-reload, ratatui TUI (feature-gated)
8. **proposal system** — agents propose new tool types via human-gated github PRs

## key architecture concepts

### tool belt model

two registries of pluggable modules loaded from config at runtime:
- **indicators** (sensing): market state → normalized score (-1.0 to +1.0). stateless, pure functions. trait defined in `crates/types/src/indicator.rs`.
- **actions** (doing): manage position lifecycle (entry, exit, monitor, sizing). can be stateful within a position's lifetime. trait defined in `crates/types/src/action.rs`.

### scoring pipeline

1. each indicator computes a score per timescale (with `catch_unwind` for panic safety)
2. scores are aggregated per timescale via weighted sum (weights normalized to 1.0)
3. timescale scores combine into a composite score using PM-tunable weights
4. hard gates: if any hard-gate timescale score < 0, composite floors to 0
5. cross-timescale agreement: confidence multiplier or hard gate mode
6. dynamic fusion (optional): volatility/trend-aware adaptive timescale weighting
7. composite >= entry_threshold → eligible for entry; composite <= exit_threshold → exit signal

### config versioning

immutable append-only model. every config change creates a new `config_versions` row. flow: proposed → backtested → validated → promoted. the execution engine reads the latest `promoted` config and hot-reloads. rollback = promote an older version.

### two-agent evolution cycles

consolidated from 7 agents to 2 per cycle (based on research synthesis — task granularity > agent count):

- **analysis agent**: exploratory, detail-oriented. fine-grained atomic sub-tasks across all timescales. output = structured JSON memo with evidence-backed suggestions. zero config authority.
- **PM agent**: conservative, evaluative. receives analysis + accumulated beliefs + validation constraints. output = approved config mutations or "hold steady" with reasoning. full config authority. maintains beliefs via CVRF (conceptual verbal reinforcement framework).

### proposal system (phase 8)

agents can request new tool *types* (not instances) through a human-gated PR process. agents can freely add/remove/reconfigure *instances* of existing types through config alone, but new types require code changes and human review.

---

## rust crates — detailed file map

### `crates/types/` — shared domain model

the foundation crate. defines all traits, types, and configs used by every other crate.

| file | key types | purpose |
|------|-----------|---------|
| `src/lib.rs` | module re-exports | barrel file — re-exports all public types |
| `src/market.rs` | `Candle`, `Timescale`, `MarketState`, `PositionContext` | market data primitives. `MarketState` is the primary input to all indicators (candle windows by timescale, bid/ask, session VWAP, position context) |
| `src/indicator.rs` | `Indicator` trait, `IndicatorOutput`, `IndicatorConfig` | indicator interface. `compute(market: &MarketState) → Option<IndicatorOutput>`. output: score in [-1.0, +1.0], raw_value, metadata map |
| `src/action.rs` | `Action` trait, `ActionSignal`, `ActionPhase`, `Position`, `ExitReason`, `ActionConfig` | action interface. `evaluate(position, market, scores) → ActionSignal`. phases: Entry, Exit, Monitor, Sizing. signals: Hold, Enter, Exit, ModifyStop, ScalePosition |
| `src/scoring.rs` | `TimescaleScores`, `ScoringConfig`, `AggregationMethod`, `AgreementConfig`, `DynamicFusionConfig` | scoring pipeline types. methods: WeightedSum, WeightedSumWithGates, MinScore, DynamicFusion. hard gates, agreement modes, dynamic fusion config |
| `src/config.rs` | `StrategyConfig`, `SessionConfig`, `TickerOverrides` | top-level config: tickers, indicator/action configs, scoring config, session rules, per-ticker overrides (entry_threshold, exit_threshold, indicator_weights, stop_loss_pct, atr_multiplier, sizing_fraction, max_hold_ms) |
| `src/registry.rs` | `IndicatorRegistry`, `ActionRegistry`, `ToolBelt` | factory pattern: config → boxed trait object. pluggable module loading at runtime |
| `src/adapter.rs` | `DataItem` conversion | ta-rs integration: `Candle` → ta-rs `DataItem` |
| `src/tick_result.rs` | `TickResult`, `TickEvent` | engine output per tick: scores + event (Nothing, PositionOpened, PositionClosed) |
| `src/proposal.rs` | proposal types | type defs for phase 8 proposal system (agents requesting new tool types) |
| `src/test_fixtures.rs` | fixture builders | shared test data: sample configs, candles, market states. used across all crates |

**tests:** 17 (config roundtrip, fixtures, adapter, tick_result)

---

### `crates/indicators/` — market signal computation

24 indicators across three categories, each normalizing output to [-1.0, +1.0].

| file | purpose |
|------|---------|
| `src/lib.rs` | `default_indicator_registry()` — registers all 24 indicators. `build_indicators()` factory |
| `src/aggregation.rs` | `compute_timescale_scores()` — per-timescale weighted sum aggregation |
| `src/helpers.rs` | normalization utilities, ta-rs conversions |

#### native indicators (13) — direct ta-rs wrappings

| file | indicator | key params |
|------|-----------|------------|
| `src/native/rsi.rs` | RSI | period (14), overbought/oversold (70/30) |
| `src/native/ema.rs` | EMA | period |
| `src/native/sma.rs` | SMA | period |
| `src/native/macd.rs` | MACD | fast (12), slow (26), signal (9) |
| `src/native/bollinger.rs` | bollinger bands | period, std_dev multiplier |
| `src/native/atr.rs` | ATR | period |
| `src/native/keltner.rs` | keltner channels | ema period, atr multiplier |
| `src/native/stochastic.rs` | stochastic (fast/slow) | k_period (5), d_period (3) |
| `src/native/cci.rs` | CCI | period (20) |
| `src/native/mfi.rs` | MFI | period (14) |
| `src/native/roc.rs` | ROC | period |
| `src/native/obv.rs` | OBV | (cumulative) |

#### composable indicators (11) — multi-step combinations

| file | indicator | built from |
|------|-----------|------------|
| `src/composable/bollinger_pct_b.rs` | bollinger %B | bollinger upper/lower |
| `src/composable/bollinger_bandwidth.rs` | bollinger bandwidth | bollinger upper/lower/SMA |
| `src/composable/stochastic_rsi.rs` | stochastic RSI | RSI → stochastic |
| `src/composable/williams_r.rs` | williams %R | highest high / lowest low |
| `src/composable/donchian.rs` | donchian channels | max/min over period |
| `src/composable/dema.rs` | DEMA | 2×EMA − EMA(EMA) |
| `src/composable/adx.rs` | ADX | DI+/DI- agreement |
| `src/composable/supertrend.rs` | supertrend | HL2 ± ATR×multiplier |
| `src/composable/ttm_squeeze.rs` | TTM squeeze | bollinger inside keltner detection |
| `src/composable/vwap_distance.rs` | VWAP distance | (close − VWAP) / VWAP |
| `src/composable/awesome_oscillator.rs` | awesome oscillator | SMA(HL2, 5) − SMA(HL2, 34) |

#### custom indicators (4) — domain-specific

| file | indicator | purpose |
|------|-----------|---------|
| `src/custom/ofi.rs` | OFI (order flow imbalance) | CLV-weighted volume proxy for order flow. from Cont et al. (2014) |
| `src/custom/vpin.rs` | VPIN | volume-synchronized probability of informed trading. from Easley et al. (2012) |
| `src/custom/position_context.rs` | position context (4 meta-indicators) | `position_direction`, `unrealized_pnl`, `hold_duration`, `session_remaining` |

**tests:** 142 (55 native, 42 composable, 18 custom, 9 VPIN, 7 aggregation, 6 registry, 3 integration, 2 smoke)

---

### `crates/actions/` — position lifecycle management

8 actions across 4 phases, loaded from config via registry.

#### entry

| file | action | behavior |
|------|--------|----------|
| `src/entry/score_threshold.rs` | score threshold entry | composite ≥ entry_threshold → long; ≤ short_threshold → short |

#### exit

| file | action | behavior |
|------|--------|----------|
| `src/exit/atr_trailing_stop.rs` | ATR trailing stop | entry: stop = price − ATR×multiplier. tightens as price moves favorably |
| `src/exit/fixed_pct_stop.rs` | fixed % stop | hard stop at entry_price × (1 ± loss_pct) |
| `src/exit/max_hold_timeout.rs` | max hold timeout | force exit after max_hold_ms |
| `src/exit/session_close.rs` | session close | exit at configurable market close time |

#### monitor

| file | action | behavior |
|------|--------|----------|
| `src/monitor/breakeven_stop.rs` | breakeven stop | once profit ≥ breakeven_pct, tighten stop to entry price |

#### sizing

| file | action | behavior |
|------|--------|----------|
| `src/sizing/fixed_fractional.rs` | fixed fractional | size = capital × fixed_fraction |
| `src/sizing/volatility_scaled.rs` | vol-scaled sizing | adjusted_fraction = base / (current_atr / baseline_atr). reduces size in high vol |

**tests:** 33 (27 action behavior, 6 registry)

---

### `crates/engine/` — tick loop and scoring pipeline

the core execution engine. both a library (for tests and backtest) and a binary.

| file | purpose |
|------|---------|
| `src/lib.rs` | module exports: scoring, position, tick_loop |
| `src/scoring.rs` | `compute_composite()` — full scoring pipeline: hard gates → weighted sum → agreement/dynamic fusion → composite score |
| `src/position.rs` | `PositionManager` — single-position lifecycle: open, update on tick, close. tracks PnL, high/low water marks, hold duration |
| `src/tick_loop.rs` | `TradingEngine` — main `on_tick(market) → TickResult` loop. computes indicators (with `catch_unwind`), aggregates scores, evaluates actions in phase order |
| `src/config.rs` | config loading and hot-reload utilities |
| `src/main.rs` | binary stub (unused — execution via data_feed/paper_trader) |

**tests:** 40 (22 scoring, 9 position, 9 tick loop)

---

### `crates/backtest/` — historical replay and metrics

validates strategy configs against historical data. both a library and a CLI binary.

| file | purpose |
|------|---------|
| `src/lib.rs` | module exports: alpaca_loader, config_loader, replay, report |
| `src/replay.rs` | `run_backtest()` — replays candles through `TradingEngine`, records trades, computes metrics |
| `src/report.rs` | `BacktestResult`, `BacktestMetrics` (P&L, sharpe, win_rate, max_drawdown, profit_factor), `compute_metrics()`, JSON/CSV export, `compare_configs()` |
| `src/alpaca_loader.rs` | `fetch_bars_range()` — async historical bar fetch from alpaca API |
| `src/config_loader.rs` | `load_promoted_config_with_id()`, `write_backtest_trades()` — postgres integration |
| `src/main.rs` | CLI: `--date YYYY-MM-DD [--lookback-days N] [--write-db]` or legacy `--config X --data Y --ticker Z`. supports walk-forward mode, per-ticker overrides (`--ticker-override "NVDA:entry_threshold=0.35"`), and `ConfigOverrides` for A/B testing |

**tests:** 40 (15 metrics, 13 replay, 8 report/serialization, 4 integration)

---

### `crates/data_feed/` — live paper trading

live market data ingestion, candle aggregation, simulated broker, trade recording. binary name: `paper_trader`.

| file | purpose |
|------|---------|
| `src/main.rs` | async main: SIGTERM/ctrl-c handling, per-ticker sessions, `select!` loop for alpaca events → broker → trade_writer. config hot-reload via `ConfigWatcher` |
| `src/alpaca_feed.rs` | `AlpacaFeed` — websocket client for alpaca market data stream. subscribes to 1-min bars |
| `src/candle_aggregator.rs` | `CandleAggregator` — ingests 1-min candles, emits clock-aligned 5-min and hourly candles. rolling window of up to 200 candles per timescale |
| `src/market_state.rs` | `MarketStateBuilder` — wraps candle aggregator, adds bid/ask spread, computes session VWAP, emits `MarketState` snapshots |
| `src/broker.rs` | `SimulatedBroker` (paper mode) — execute orders, track fills, simulate slippage. respects position and capital limits |
| `src/account.rs` | account state: cash, margin, equity. reads `INITIAL_CAPITAL` env var or queries alpaca |
| `src/live_session.rs` | `LiveSession` (per ticker) — single intraday position lifecycle. stashes entry scores, pairs with exit scores on close |
| `src/trade_writer.rs` | `TradeWriter` — async writes completed trades to postgres |
| `src/config_watcher.rs` | `ConfigWatcher` — polls postgres for promoted config changes. `try_build_engine()` applies per-ticker overrides from `config.ticker_overrides`, then rebuilds with fallback to previous config |
| `src/config_loader.rs` | config loading from database or JSON file |
| `src/tui.rs` | (feature-gated: `--features tui`) ratatui terminal UI: live positions, P&L, recent trades, market state |
| `src/lib.rs` | module tree and re-exports |

**tests:** 54 (15 candle_aggregator, 10 broker, 8 market_state, 6 live_session, 5 trade_writer, 5 config_watcher, 5 alpaca_feed)

---

## typescript agent layer (`agents-ts/`)

### core orchestration

| file | purpose |
|------|---------|
| `src/orchestrator.ts` | main scheduler and budget enforcement. cron: 12:30 ET analysis (sonnet), 15:30 ET full PM (opus). `runFullPmCycle()`: analysis agent → PM agent → backtest validate → promote/reject. CLI: `--mode scheduled\|once\|pm\|checkin`. SIGTERM handling via `AbortController` |
| `src/agent-base.ts` | shared agent invocation framework. `runAgentSdk()`: creates in-process MCP server, builds tool array, calls `sdk.query()`, parses responses, tracks token usage. `runAnalysisAgent(model, pool, cycleId)` and `runPmAgent(model, pool, cycleId)` are the two entry points |
| `src/models.ts` | type definitions mirroring postgres schema: `AgentMemo`, `EvolutionCycle`, `Suggestion`, `Belief`, `TokenUsage`. factory functions: `createAgentMemo()`, `createEvolutionCycle()` |
| `src/db.ts` | `getPool()` — pg.Pool with `DATABASE_URL` env var |
| `src/logger.ts` | structured logging |
| `src/walk-forward.ts` | walk-forward backtesting utility (multi-day rolling window validation) |
| `src/promote-config.ts` | one-liner utility to promote a config version |
| `src/index.ts` | barrel re-exports |

### agent tools

| file | tools | access level |
|------|-------|--------------|
| `src/tools/sql-queries.ts` | `getRecentTrades`, `getDailyPerformance`, `getConfigChangelog`, `getPerformanceByExitReason`, `getCheckinMemosSinceLastPm`, `getAnalysisMemos`, `getActiveBeliefs` | read-only (both agents) |
| `src/tools/config-ops.ts` | `getCurrentConfig`, `getConfigVersion`, `proposeConfig`, `updateConfigStatus`, `writeChangelogEntries` | write (PM agent only) |
| `src/tools/memo-writer.ts` | `writeMemo` | write (both agents, different memo types) |
| `src/tools/backtest-runner.ts` | `runBacktestValidation`, `validateBacktestResult` | spawns backtest binary, polls for results |
| `src/tools/config-diff.ts` | `computeConfigDiff` | pure function — produces atomic change dicts for changelog |
| `src/tools/log-reader.ts` | `readLogs` | read-only — tails paper_trader/engine logs |

**tool set assignment:**
- analysis agent (7 tools): `get_recent_trades`, `get_daily_performance`, `get_current_config`, `get_config_changelog`, `get_performance_by_exit_reason`, `get_beliefs`, `write_analysis_memo`
- PM agent (12 tools): all analysis tools + `get_prior_memos`, `get_analysis_memos`, `propose_config_mutation`, `write_pm_memo`, `write_belief`, `read_logs`

### prompts

| file | agent | model | purpose |
|------|-------|-------|---------|
| `prompts/analysis.md` | analysis agent | sonnet 4.6 (mid-day) / opus 4.6 (end-of-day) | analyze trade history, compute metrics, assess signal quality, detect market regime, produce structured JSON suggestions with evidence. zero config authority |
| `prompts/agent_pm.md` | PM agent | opus 4.6 | evaluate analysis + suggestions + beliefs, propose config mutations or hold steady, write beliefs (CVRF framework). full config authority |

### typescript tests

| file | tests | coverage |
|------|-------|----------|
| `tests/agent-base.test.ts` | 11 | prompt loading, tool sets, token cost, tool invocation parsing |
| `tests/models.test.ts` | 21 | model factories, type conversions, enum validation |
| `tests/orchestrator.test.ts` | 12 | budget tracking, cycle state machine, agent triggering |
| `tests/tools/backtest-runner.test.ts` | 7 | spawning backtest binary, result parsing |
| `tests/tools/config-diff.test.ts` | 8 | diff computation, change categorization |
| `tests/tools/config-ops.test.ts` | 4 | config versioning, status transitions |
| `tests/tools/log-reader.test.ts` | 9 | log tailing |

**total: 72 tests passing, tsc clean**

---

## database schema

managed via sqlx migrations in `migrations/`. reference schema in `docs/data_model.sql`.

### enums

`timescale`, `trade_direction`, `exit_reason`, `agent_type`, `memo_type`, `cycle_type`, `mutation_status`, `trade_event_type`, `change_category`, `volatility_regime`, `directional_bias`, `signal_quality`, `belief_status`

### core tables

| table | purpose | key fields |
|-------|---------|------------|
| `config_versions` | immutable append-only config store | id, status (proposed/backtesting/validated/promoted/rejected), config_blob (JSONB), parent_version_id, backtest results |
| `trades` | completed trade records | ticker, direction, entry/exit prices, PnL, exit_reason, entry/exit scores per timescale, config_version_id |
| `indicator_snapshots` | historical indicator values per trade | timestamp, indicator_id, timescale, raw_value, normalized_score |
| `agent_memos` | structured agent outputs | agent, memo_type, reasoning, suggestions (JSONB), confidence, regime/bias/signal assessments, period metrics |
| `evolution_cycles` | per-cycle metadata and cost tracking | trading_date, cycle_type, model_used, token usage, cost, configs proposed/promoted/rejected |
| `config_changelog` | atomic change records | config_version_id, change_category, tool_name, param_name, old/new values, reason |
| `daily_budget` | per-day token and cost tracking | trading_date, total cost, cycle counts, budget_limit (default $5), budget_exhausted flag |
| `beliefs` | accumulated investment beliefs (CVRF) | belief_id, stated_belief, status, confidence, evidence, tool_context |

### analysis views

`daily_performance`, `performance_by_exit_reason`, `score_interaction_analysis`, `recent_agent_signals`, `checkin_memos_since_last_pm`, `daily_cost_summary`

### migrations (chronological)

| migration | what it creates |
|-----------|----------------|
| `20260228000001` | all enum types |
| `20260228000002` | `config_versions` table + `active_config` view |
| `20260228000003` | `trades` table with full score columns |
| `20260228000004` | `indicator_snapshots` table |
| `20260228000005` | `agent_memos` table |
| `20260228000006` | `evolution_cycles` table |
| `20260228000007` | `config_changelog` table |
| `20260228000008` | `daily_budget` table |
| `20260228000009` | analysis views |
| `20260228000010` | seed initial config (24 indicators, 8 actions) |
| `20260302000001` | updated seed config (OFI/VPIN, vol-scaled sizing) |
| `20260302000002` | add `agent_analysis` to agent_type enum |
| `20260302000003` | add `suggestions` JSONB column to agent_memos |
| `20260302000004` | `beliefs` table |
| `20260302000005` | cleanup old agent_type enum values |

---

## infrastructure

### docker compose (`docker-compose.yml`)

3 services:

| service | image | purpose |
|---------|-------|---------|
| `postgres` | postgres:16 | database on port 5433. runs migrations on startup |
| `paper_trader` | rust binary | live market data + simulated execution. reads `DATABASE_URL`, `APCA_*`, `BROKER_MODE` |
| `agents-ts` | node | orchestrator in `--mode scheduled`. reads `DATABASE_URL`, `ANTHROPIC_API_KEY` |

### environment variables

| variable | used by | purpose |
|----------|---------|---------|
| `DATABASE_URL` | both layers | postgres connection string |
| `ANTHROPIC_API_KEY` | agents-ts | claude API access |
| `APCA_API_KEY_ID`, `APCA_API_SECRET_KEY` | paper_trader | alpaca market data + broker |
| `BROKER_MODE` | paper_trader | `simulated` or `alpaca` |
| `INITIAL_CAPITAL` | paper_trader | starting capital for simulated broker |
| `BACKTEST_DATA_DIR` | agents-ts | if set, backtest validates before promote; if unset, auto-promotes |
| `LOG_DIR` | both | log file directory for `read_logs` tool |
| `RUST_LOG` | paper_trader | log level filter |

---

## data flow summary

```
alpaca websocket
    │
    ▼
AlpacaFeed (1-min bars)
    │
    ▼
CandleAggregator (1m → 5m, 1h)
    │
    ▼
MarketStateBuilder (+ VWAP, bid/ask)
    │
    ▼
TradingEngine.on_tick(MarketState)
    ├─► indicators compute scores (catch_unwind per indicator)
    ├─► aggregate per-timescale (weighted sum)
    ├─► compute composite (hard gates + agreement/fusion)
    ├─► evaluate actions (entry → monitor → exit → sizing)
    └─► TickResult { scores, event }
            │
            ▼
    LiveSession (stash scores, manage position)
            │
            ▼
    TradeWriter → postgres (trades table)
            │
            ▼
    agents-ts reads trade data (sql-queries.ts)
            │
            ▼
    analysis agent → structured memo + suggestions
            │
            ▼
    PM agent → propose config mutation → backtest validate → promote
            │
            ▼
    ConfigWatcher detects promoted config → hot-reload TradingEngine
```

---

## key patterns for contributors

### adding a new indicator

1. create `crates/indicators/src/{native|composable|custom}/my_indicator.rs`
2. implement the `Indicator` trait from `types::indicator`
3. register in `crates/indicators/src/lib.rs` → `default_indicator_registry()`
4. add tests in `crates/indicators/tests/`
5. add to a config's `indicators` array with instance_id, timescale, weight, params

### adding a new action

1. create `crates/actions/src/{entry|exit|monitor|sizing}/my_action.rs`
2. implement the `Action` trait from `types::action`
3. register in `crates/actions/src/lib.rs` → `default_action_registry()`
4. add tests in `crates/actions/tests/`
5. add to a config's `actions` array with instance_id, phase, priority, params

### adding a new agent tool

1. add SQL query or function in `agents-ts/src/tools/` (use parameterized queries only)
2. add to tool set in `agents-ts/src/agent-base.ts` (`ANALYSIS_TOOL_NAMES` or `PM_TOOL_NAMES`)
3. tool factory uses zod/v4 for schema validation
4. add tests in `agents-ts/tests/tools/`

### config lifecycle

```
proposed → backtesting → validated → promoted
                      ↘ rejected

rollback = promote an older version
```

agents can add/remove/reconfigure *instances* of existing tool types through config alone. new tool *types* require code changes and human review (phase 8 proposal system).

---

## companion docs reference

| file | purpose | when to read |
|------|---------|--------------|
| [system_architecture.mermaid](docs/system_architecture.mermaid) | mermaid diagram of full system | understanding data flows between components |
| [development_spec.md](docs/development_spec.md) | implementation roadmap (phases 1-8) with definitions of done | understanding what's built vs planned |
| [technical_reference.md](docs/technical_reference.md) | consolidated rust types + sql schema | quick lookup of struct definitions and table schemas |
| [tool_belt_catalog.md](docs/tool_belt_catalog.md) | 163 indicators + 124 actions with knobs and timescale assignments | adding/modifying indicators or actions |
| [ta_rs_implementation_map.md](docs/ta_rs_implementation_map.md) | indicator → ta-rs mapping (22 native, 74 composable, ~67 custom) | implementing new indicators |
| [data_model.sql](docs/data_model.sql) | complete postgres schema (enums, tables, views, config blob structure) | writing SQL queries or modifying schema |
| [cli_usage_guide.md](docs/cli_usage_guide.md) | command-line reference for backtest, paper_trader, orchestrator | running any binary |
| [usage_guide.md](docs/usage_guide.md) | operations manual: prerequisites, quick start, docker deployment | first-time setup or deployment |
| [hardening_plan.md](docs/hardening_plan.md) | 3-tier safety guardrails for real capital | preparing for live trading |
| [research_synthesis.md](docs/research_synthesis.md) | applied learnings from 44 papers → architecture decisions | understanding why decisions were made |
| [action_items.md](docs/action_items.md) | prioritized implementation items from research synthesis | checking remaining work items |
| [agentic_system_recommendations.md](docs/agentic_system_recommendations.md) | post-mortem of oct 1-14 walk-forward run | debugging agent behavior |
| [intraday-trading-research.md](docs/intraday-trading-research.md) | bibliography of 32 papers on ML trading and microstructure | deep research on trading strategies |
| [multi-agent-research.md](docs/multi-agent-research.md) | 10 papers on multi-agent LLM trading systems | understanding agent consolidation rationale |

---

## test summary

| crate/package | tests | notes |
|---------------|-------|-------|
| types | 17 | fixtures, adapter, config roundtrip, tick_result |
| indicators | 142 | 55 native, 42 composable, 18 custom, 9 VPIN, 7 aggregation, 6 registry, 3 integration, 2 smoke |
| actions | 33 | 27 action behavior, 6 registry |
| engine | 40 | 22 scoring, 9 position, 9 tick loop |
| backtest | 40 | 15 metrics, 13 replay, 8 report, 4 integration |
| data_feed | 54 | 15 candle, 10 broker, 8 market_state, 6 live_session, 5 trade_writer, 5 config_watcher, 5 alpaca |
| **rust total** | **326** | **0 failures, 2 ignored** |
| agents-ts | 72 | 11 agent-base, 21 models, 12 orchestrator, 8 config-diff, 7 backtest, 4 config-ops, 9 log-reader |
| **ts total** | **72** | **tsc clean** |

## key constraints

- rust engine must never panic in the tick loop — log and continue on indicator errors
- config loading failures must fall back to the previous valid config
- all indicator scores normalize to -1.0..+1.0
- agent tools are read-only for trade data; write-only for memos and config proposals
- agents use only parameterized sql queries, never arbitrary sql
- api keys and db credentials must come from environment variables
- `ta` crate version is pinned to v0.5
- core rust dependencies: `ta`, `serde`, `serde_json`, `chrono`, `tokio`
- intraday only — no overnight holds
