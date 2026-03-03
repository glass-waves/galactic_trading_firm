# CLAUDE.md

this file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## project overview

adaptive multi-timescale intraday trading system with two-layer architecture:

- **fast layer (rust)**: execution engine processing ticks in real-time using `ta` crate (ta-rs v0.5) for indicator computation. targets sub-ms latency per tick.
- **slow layer (typescript)**: evolution agents via claude agent sdk. opus 4.6 for full PM cycles and recommendations (1x daily), sonnet 4.6 for check-ins (3x daily). orchestrator enforces a $5/day budget cap under anthropic tier 1 ($100/month).

instruments: SPY, QQQ, 3-5 liquid mega-caps. intraday only, no overnight holds.

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
2. **indicator engine** — wrap 22 native ta-rs indicators, build ~30 composable indicators, implement registry
3. **action engine & scoring** — scoring pipeline (weighted sum + hard gates), core actions (entry/exit/sizing), tick loop
4. **backtest engine** — historical replay, report generation, config comparison
5. **agent layer** — typescript orchestrator, check-in agents (sonnet 4.6), agent tools
6. **full evolution loop** — PM agent (opus 4.6) with config mutation authority, backtest validation gates, hot-reload
7. **paper trading** — live market data, simulated broker, observability
8. **proposal system** — agents propose new tool types via human-gated github PRs

## key architecture concepts

### tool belt model

two registries of pluggable modules loaded from config at runtime:
- **indicators** (sensing): market state → normalized score (-1.0 to +1.0). stateless, pure functions. trait defined in `crates/types/src/indicator.rs`.
- **actions** (doing): manage position lifecycle (entry, exit, monitor, sizing). can be stateful within a position's lifetime. trait defined in `crates/types/src/action.rs`.

### scoring pipeline

1. each indicator computes a score per timescale
2. scores are aggregated per timescale via weighted sum (weights normalized to 1.0)
3. timescale scores combine into a composite score using PM-tunable weights
4. hard gates: if any hard-gate timescale score < 0, composite floors to 0
5. composite >= entry_threshold → eligible for entry; composite <= exit_threshold → exit signal

### config versioning

immutable append-only model. every config change creates a new `config_versions` row. flow: proposed → backtested → validated → promoted. the execution engine reads the latest `promoted` config and hot-reloads. rollback = promote an older version.

### two-tier evolution cycles

- **full PM cycle** (opus 4.6): 3 recommendation agents produce parameter change suggestions → PM agent reads memos + trade data + changelog → proposes config mutations → backtest validates → promote or reject
- **check-in cycle** (sonnet 4.6): 1min/5min/hourly agents produce observation-only memos with zero config authority. builds evidence base for next full PM cycle.

### proposal system (phase 8)

agents can request new tool *types* (not instances) through a human-gated PR process. agents can freely add/remove/reconfigure *instances* of existing types through config alone, but new types require code changes and human review.

## companion artifact reference

| file | purpose |
|------|---------|
| `docs/development_spec.md` | implementation plan with phases and definitions of done |
| `docs/tool_belt_catalog.md` | 163 indicators + 124 actions with knobs and timescale assignments |
| `docs/ta_rs_implementation_map.md` | indicator → ta-rs mapping (22 native, 74 composable, ~67 custom) |
| `docs/technical_reference.md` | consolidated rust types + sql in one file |
| `docs/system_architecture.mermaid` | full architecture diagram (mermaid) |
| `docs/data_model.sql` | complete postgres schema reference |

## key constraints

- rust engine must never panic in the tick loop — log and continue on indicator errors
- config loading failures must fall back to the previous valid config
- all indicator scores normalize to -1.0..+1.0
- agent tools are read-only for trade data; write-only for memos and config proposals
- agents use only parameterized sql queries, never arbitrary sql
- api keys and db credentials must come from environment variables
- `ta` crate version is pinned to v0.5
- core rust dependencies: `ta`, `serde`, `serde_json`, `chrono`, `tokio`
