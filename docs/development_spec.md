# Development Specification

**Adaptive Multi-Timescale Trading System | v0.3 | February 2026**

This document serves as the implementation anchor. It defines what to build, in what order, with what technologies, and what "done" looks like for each phase. All companion artifacts (Rust types, SQL schema, architecture diagram, tool catalogs) are referenced by name rather than duplicated here.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Repository Structure](#2-repository-structure)
3. [Phase 1: Foundation](#3-phase-1-foundation)
4. [Phase 2: Indicator Engine](#4-phase-2-indicator-engine)
5. [Phase 3: Action Engine & Scoring Pipeline](#5-phase-3-action-engine--scoring-pipeline)
6. [Phase 4: Backtest Engine](#6-phase-4-backtest-engine)
7. [Phase 5: Agent Layer — Orchestrator & Check-ins](#7-phase-5-agent-layer)
8. [Phase 6: Full Evolution Loop](#8-phase-6-full-evolution-loop)
9. [Phase 7: Paper Trading](#9-phase-7-paper-trading)
10. [Phase 8: Proposal System](#10-phase-8-proposal-system)
11. [Cross-Cutting Concerns](#11-cross-cutting-concerns)
12. [Companion Artifacts](#12-companion-artifacts)

---

## 1. System Overview

Two-layer architecture: a fast Rust execution engine handles real-time tick processing, while a slow Python/LLM evolution layer periodically analyzes trade outcomes and tunes strategy parameters via the Claude Agent SDK.

### Key Architecture Decisions (Locked)

- **Execution engine**: Rust, using `ta` crate (ta-rs v0.5) for indicator computation
- **Agent runtime**: Claude Agent SDK (Python), headless mode with JSON output
- **LLM models**: Sonnet 4.5 for full PM cycles, Haiku 4.5 for check-ins
- **API auth**: Anthropic API key, Tier 1 ($100/month hard cap)
- **Database**: PostgreSQL (consider SQLite for early dev/backtest)
- **Config model**: Immutable append-only versioning
- **Instrument scope**: SPY, QQQ, 3-5 liquid mega-caps (equities/ETFs only)
- **Trading style**: Intraday, no overnight holds

### Reference: `system_architecture.mermaid` (v0.3)

---

## 2. Repository Structure

```
adaptive-trading/
├── Cargo.toml                    # Rust workspace
├── crates/
│   ├── engine/                   # Execution engine binary
│   │   ├── src/
│   │   │   ├── main.rs           # Entry point, tick loop
│   │   │   ├── config.rs         # Config loading, hot-reload
│   │   │   ├── scoring.rs        # Scoring pipeline
│   │   │   └── position.rs       # Position state management
│   │   └── Cargo.toml
│   ├── indicators/               # Indicator library
│   │   ├── src/
│   │   │   ├── lib.rs            # Indicator trait, registry
│   │   │   ├── native/           # Direct ta-rs wrappers (Phase 2a)
│   │   │   ├── composable/       # ta-rs primitive compositions (Phase 2b)
│   │   │   └── custom/           # From-scratch implementations (Phase 2c+)
│   │   └── Cargo.toml
│   ├── actions/                  # Action library
│   │   ├── src/
│   │   │   ├── lib.rs            # Action trait, registry
│   │   │   ├── entry/            # Entry strategy implementations
│   │   │   ├── exit/             # Stop, trailing, take profit, time, signal
│   │   │   ├── monitor/          # Position monitoring actions
│   │   │   └── sizing/           # Position sizing methods
│   │   └── Cargo.toml
│   ├── backtest/                 # Backtest engine
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── replay.rs         # Historical data replay
│   │   │   └── report.rs         # Backtest result generation
│   │   └── Cargo.toml
│   └── types/                    # Shared types (from tool_belt_types.rs)
│       ├── src/lib.rs
│       └── Cargo.toml
├── agents/                       # Python agent layer
│   ├── orchestrator.py           # Main scheduler, budget tracking
│   ├── agent_base.py             # Shared agent invocation logic
│   ├── prompts/                  # System prompts per agent
│   │   ├── agent_1min.md
│   │   ├── agent_5min.md
│   │   ├── agent_hourly.md
│   │   ├── agent_daily.md
│   │   ├── agent_monthly.md
│   │   ├── agent_pm.md
│   │   ├── checkin_1min.md       # Lighter check-in prompts
│   │   ├── checkin_5min.md
│   │   └── checkin_hourly.md
│   ├── tools/                    # Agent tools (SQL queries, config ops)
│   │   ├── sql_queries.py
│   │   ├── config_ops.py
│   │   └── memo_writer.py
│   └── requirements.txt
├── migrations/                   # SQL migrations (from data_model.sql)
│   ├── 001_initial_schema.sql
│   └── ...
├── docs/                         # Companion artifacts
│   ├── tool_belt_catalog.md
│   ├── ta_rs_implementation_map.md
│   └── ...
└── CLAUDE.md                     # Claude Code context for the repo
```

---

## 3. Phase 1: Foundation

**Goal**: Compilable Rust workspace with shared types, database schema deployed, basic config loading.

### 3.1 Rust Workspace Setup

- Initialize Cargo workspace with crates: `types`, `indicators`, `actions`, `engine`, `backtest`
- Copy type definitions from `tool_belt_types.rs` into `crates/types/src/lib.rs`
- Add dependencies: `ta = "0.5"`, `serde`, `serde_json`, `chrono`, `tokio`
- Verify: `cargo build` succeeds, `cargo test` passes

### 3.2 Database

- Deploy `data_model.sql` to PostgreSQL (or SQLite for initial dev)
- Write a small Rust module (`crates/engine/src/config.rs`) that:
  - Loads the latest `promoted` config from `config_versions`
  - Deserializes `config_blob` JSONB into `StrategyConfig`
  - Validates the config (all referenced indicator/action types exist)
- Seed an initial config version with a minimal set of indicators (RSI, EMA, ATR, Bollinger) and actions (score threshold entry, ATR trailing stop, fixed risk sizing, session close exit)

### 3.3 Market Data Types

- Define `Candle`, `MarketState` in types crate (already done in `tool_belt_types.rs`)
- Create a `DataItem` adapter that bridges ta-rs's trait requirements with our `Candle` type
- Write unit tests verifying ta-rs indicators accept our data types

### Definition of Done

- `cargo build --workspace` succeeds
- Database schema deployed with initial seed config
- Config loads from DB and deserializes into `StrategyConfig`
- ta-rs `ExponentialMovingAverage` runs against our `Candle` type in a test

---

## 4. Phase 2: Indicator Engine

**Goal**: Working indicator registry that loads from config and computes scores per tick.

### 4.2a Native ta-rs Wrappers (Priority)

Wrap all 22 native ta-rs indicators behind our `Indicator` trait. Each wrapper:
- Accepts `IndicatorConfig` in its factory function
- Extracts knob values from `params` HashMap
- Maintains the internal ta-rs struct as state
- Implements `compute(&self, market: &MarketState) -> Option<IndicatorOutput>`
- Normalizes raw output to -1.0 .. +1.0 score

Start with the most useful subset: RSI, EMA, SMA, MACD, Bollinger Bands, ATR, Keltner, Stochastic (fast/slow), CCI, MFI, OBV, ROC.

### 4.2b Composable Indicators (Second Priority)

Build the ~30 most impactful composable indicators from ta-rs primitives. Priority list:
- ADX / +DI / -DI (trend strength — critical for filtering)
- Donchian Channels (breakout detection)
- SuperTrend (trend following overlay)
- VWAP + deviation bands (intraday anchor — custom session logic)
- Bollinger %B and BandWidth (from native BB output)
- TTM Squeeze (from native BB + KC)
- Stochastic RSI (from native RSI + stochastic logic)
- Williams %R (from native Max/Min)
- DEMA, TEMA, HMA (from native EMA/SMA)
- Awesome Oscillator (from native SMA)
- Aroon Up/Down (from native Max/Min with index tracking)
- Ichimoku Cloud (from native Max/Min with displacement)
- Fisher Transform (from native Max/Min + log math)

### 4.2c Indicator Registry

- Implement `IndicatorRegistry` with factory function registration
- On config load: iterate `config.indicators`, call factory for each, populate `ToolBelt.indicators`
- On tick: iterate all active indicators, compute scores, accumulate per-timescale

### Reference: `ta_rs_implementation_map.md` for full mapping

### Definition of Done

- 12+ native ta-rs wrappers passing unit tests
- 10+ composable indicators passing unit tests
- Registry loads indicators from a config, computes all scores on synthetic market data
- Per-timescale score aggregation produces correct weighted sums

---

## 5. Phase 3: Action Engine & Scoring Pipeline

**Goal**: Complete tick loop — indicators → scores → entry/exit decisions.

### 5.1 Scoring Pipeline

- Implement `compute_composite()`: weighted sum of timescale scores
- Implement hard gates: if any hard-gate timescale score < 0, composite is floored to 0
- Entry threshold check: composite >= entry_threshold
- Exit threshold check: composite <= exit_threshold while in position

### 5.2 Core Actions

Implement minimum viable set of actions:

**Entry**: `ScoreThresholdEntry` — enter when composite score exceeds threshold and all gates pass.

**Exit**: `ATRTrailingStop` — trail by N × ATR from high water mark. `FixedPercentageStop` — hard stop at X% loss. `SessionCloseExit` — force exit before market close. `MaxHoldTimeout` — exit if position open > N minutes.

**Sizing**: `FixedFractionalSizing` — risk X% of capital per trade.

**Monitor**: `BreakevenStop` — move stop to entry after profit reaches threshold.

### 5.3 Execution Loop

Wire it all together in `crates/engine/src/main.rs`:
1. Load config → populate registries
2. On each tick: compute indicators → aggregate scores → evaluate actions
3. Track position state (entry price, high water mark, hold duration)
4. Log trade events to the `trades` table with full decision context

### Definition of Done

- Complete tick loop runs on synthetic data
- Scoring pipeline produces correct composites with hard gates
- Entry and exit actions fire at correct conditions
- Trade events written to DB with all required fields populated

---

## 6. Phase 4: Backtest Engine

**Goal**: Replay historical data through the same engine logic, generate performance reports.

### 6.1 Historical Data Replay

- Accept historical candle data (CSV or DB query) across all timescales
- Feed candles through the execution engine tick-by-tick
- Track all trades, indicator snapshots, and scores

### 6.2 Report Generation

- Compute: total P&L, win rate, Sharpe ratio, max drawdown, average hold time, trades per day
- Breakdown by: exit reason, timescale score buckets, time of day
- Output as JSON (for agent consumption) and human-readable summary

### 6.3 Config Comparison

- Run two configs against the same historical period
- Diff the results to show which changes improved/degraded performance
- This is what the PM agent will use to validate mutations before promoting

### 6.4 Visualization — Backtest Reports

the rust backtest engine outputs structured JSON and CSV files containing trade records, indicator snapshots, composite scores, and equity curves. a python report generator consumes this output and produces standalone HTML reports using plotly.

**report contents:**
- price chart with entry/exit markers (green triangles for entries, red triangles for exits)
- indicator score subplots per timescale (stacked below price chart)
- composite score line with entry/exit threshold bands
- equity curve and drawdown chart
- trade summary table (entry/exit prices, P&L, hold duration, exit reason)
- config comparison overlay when diffing two configs

**implementation approach:**
- `crates/backtest/src/report.rs` — serializes `BacktestResult` to JSON/CSV
- `agents/tools/report_generator.py` — reads JSON, generates HTML via plotly
- reports are self-contained single HTML files (no external dependencies) for easy sharing
- plotly chosen over matplotlib for interactivity (zoom, hover tooltips, pan) without a server

### Definition of Done

- Backtest engine replays 1 month of SPY 1-minute data
- Results match manual spot-check calculations
- Config A vs Config B comparison produces meaningful diff
- Results serialize to JSON matching the schema agents expect
- HTML report renders with interactive price chart, indicator subplots, and trade table

---

## 7. Phase 5: Agent Layer — Orchestrator & Check-ins

**Goal**: Python orchestrator running lightweight check-in cycles via Claude Agent SDK.

### 7.1 Python Orchestrator

- `orchestrator.py`: main entry point
  - Reads schedule config (check-in frequency, full cycle times)
  - Checks daily_budget table before starting any cycle
  - Dispatches cycles via Agent SDK
  - Tracks token usage from API responses (input_tokens, output_tokens)
  - Updates daily_budget after each cycle
  - Logs evolution_cycles row with cycle_type, model_used, cost
- `agent_base.py`: shared invocation logic
  - Constructs Agent SDK call with system prompt, tools, output schema
  - Handles --session-id, --max-turns, --output-format json
  - Parses structured response, writes agent_memos row
  - Returns token usage for budget tracking

### 7.2 Check-in Agent Prompts

Write system prompts for 1-min, 5-min, and hourly check-in agents. Each prompt:
- Explains the agent's timescale focus
- Provides current config context (read-only)
- Includes recent trade data (last N trades for that timescale)
- Instructs the agent to write an observation memo (NOT recommendation)
- Defines the JSON output schema matching agent_memos fields
- Explicitly states: "You have no config modification authority in this check-in."

### 7.3 Agent Tools

- `sql_queries.py`: parameterized queries for recent trades, performance stats, current config, changelog since last cycle
- `memo_writer.py`: writes structured memo to agent_memos table with memo_type = 'observation'

### Definition of Done

- Orchestrator runs a check-in cycle for 1-min, 5-min, hourly agents
- Haiku 4.5 produces structured observation memos
- Token usage tracked, daily_budget updated
- Budget enforcement prevents new cycles when daily cap hit

---

## 8. Phase 6: Full Evolution Loop

**Goal**: Complete PM-orchestrated evolution cycle with config modification authority.

### 8.1 Full Cycle Agent Prompts

Write system prompts for all 5 timescale agents (recommendation mode) and the PM agent. The PM prompt:
- Includes all timescale recommendation memos from this cycle
- Includes all check-in observation memos since last full cycle
- Includes recent trade performance data
- Includes config changelog (recent changes + their outcomes)
- Has tools for: reading trades, querying indicator snapshots, proposing config mutations, writing changelog entries
- Defines JSON output schema for config proposals

### 8.2 Config Mutation Flow

1. PM agent proposes a config change (JSON patch to config_blob)
2. Orchestrator writes new config_version row with status = 'proposed'
3. Orchestrator runs backtest against recent historical period
4. If backtest passes validation thresholds → promote (status = 'promoted')
5. Write config_changelog entries for each atomic change
6. Execution engine hot-reloads the new config

### 8.3 Backtest Validation Gates

- Sharpe ratio must not degrade by more than X% vs current config
- Win rate must remain above minimum threshold
- Maximum drawdown must not exceed limit
- These thresholds are themselves PM-tunable knobs

### Definition of Done

- Full PM cycle runs end-to-end: all agents → PM → config proposal → backtest → promote/reject
- Config changelog accurately reflects all changes
- Promoted config hot-reloads in execution engine
- PM reads check-in memos and references them in reasoning

---

## 9. Phase 7: Paper Trading

**Goal**: System runs live against real market data, executing paper trades.

### 9.1 Market Data Integration

- Connect to a market data provider (broker API or data service)
- Stream 1-minute candles for target instruments
- Build higher-timeframe candles (5-min, hourly, daily) from 1-min feed

### 9.2 Paper Trading Broker

- Implement a simulated broker that:
  - Accepts order signals from the engine
  - Fills at next-tick price with configurable slippage model
  - Tracks simulated positions and P&L
  - Writes trade events to DB with `is_paper = true`

### 9.3 Observability

- Log every tick loop iteration with timing metrics
- Alert on: budget exhaustion, engine errors, agent invocation failures

**terminal UI (ratatui):**
- real-time dashboard in the terminal using `ratatui` crate
- panels: current price + sparkline, composite score gauge, per-timescale score bars, position state (direction, P&L, hold duration), recent trade log
- lightweight — runs in the same process as the engine with minimal overhead
- serves as the primary monitoring interface during paper trading

**operational dashboards (optional, later):**
- prometheus metrics exporter for the rust engine (tick latency, indicator compute times, position state)
- grafana dashboards for historical visualization and alerting
- only worth adding once the system runs unattended for extended periods

### Definition of Done

- System runs for a full trading day on live SPY data
- Paper trades are logged with full decision context
- Evolution cycles run on schedule, producing meaningful memos
- No crashes, memory leaks, or missed ticks over 1 week of operation

---

## 10. Phase 8: Proposal System

**Goal**: Agents can identify capability gaps and propose new tool types through human-gated PRs.

This phase is deliberately last — the system needs to be running and generating trade data before agents can meaningfully identify gaps.

### 10.1 Tool Request Pipeline

- Agent writes ToolRequest with evidence (trade IDs, estimated impact)
- PM evaluates requests during full cycle, accepts or declines with reasoning
- Accepted requests get a ToolProposal with Rust implementation spec

### 10.2 GitHub PR Integration

- PM agent generates: Rust source file, unit tests, agent documentation
- Orchestrator creates a Git branch and opens a PR
- Human reviews, comments, iterates
- On merge + deploy, new tool type appears in registry

### Reference: `proposal_system.rs` for type definitions

### Definition of Done

- PM agent can generate a ToolProposal with valid Rust code
- PR is opened on GitHub with correct file structure
- After human merge + rebuild, new indicator/action type is usable in config

---

## 11. Cross-Cutting Concerns

### 11.1 Error Handling

- Rust engine: never panic in the tick loop. Log and continue on indicator errors.
- Agent SDK: retry with exponential backoff on rate limits (429). Fail gracefully on budget exhaustion.
- Config loading: if new config is invalid, keep running with previous config.

### 11.2 Testing Strategy

- **Unit tests**: Every indicator wrapper, action evaluator, and scoring edge case
- **Integration tests**: Full tick loop on synthetic data with known expected outcomes
- **Backtest regression**: Known config on known data must produce known results (golden file test)
- **Agent tests**: Mock Agent SDK responses, verify orchestrator handles all output shapes

### 11.3 Visualization Strategy

three tiers of visualization, introduced incrementally:

1. **backtest HTML reports** (phase 4): standalone interactive HTML files generated by python from rust JSON output. uses plotly for charts. no server required.
2. **terminal UI** (phase 7): `ratatui`-based real-time dashboard for monitoring during paper trading. runs in-process.
3. **operational dashboards** (optional): prometheus + grafana for long-running unattended operation. only if needed.

### 11.4 Logging & Observability

- Structured logging (JSON) for the Rust engine
- Token/cost logging for every Agent SDK call
- Daily summary: trades, P&L, evolution cycles, budget usage

### 11.5 Security

- API key in environment variable, never in code or config files
- Database credentials via secrets management
- Agent tools are read-only for trade data; write-only for memos and config proposals
- Agents cannot execute arbitrary SQL — only parameterized queries

### 11.6 CLAUDE.md

Create a `CLAUDE.md` in the repo root that provides Claude Code context for development work:
- Project overview and architecture summary
- Crate structure and dependency map
- Key design decisions and constraints
- Testing and build commands
- Link to this development spec and companion artifacts

---

## 12. Companion Artifacts

| Artifact | Purpose |
|----------|---------|
| `adaptive_trading_system_project_doc.docx` | Project overview document (v0.3) |
| `system_architecture.mermaid` | Architecture diagram with two-tier cycles (v0.3) |
| `data_model.sql` | Complete SQL schema (v0.3) |
| `tool_belt_types.rs` | Rust type definitions for execution engine |
| `proposal_system.rs` | Rust types for tool request/proposal pipeline |
| `technical_reference.md` | Consolidated Rust + SQL in single markdown |
| `tool_belt_catalog.md` | 163 indicators + 124 actions with knobs |
| `ta_rs_implementation_map.md` | Indicator → ta-rs implementation mapping |
| `development_spec.md` | This document |
