# galactic trading firm

adaptive intraday trading system that evolves its own strategy. a rust execution engine processes market ticks in real-time while claude agents analyze performance and mutate the strategy config daily.

trades SPY, QQQ, and liquid mega-caps. intraday only — all positions close before market close.

## how it works

```
market data (alpaca) → rust engine (sub-ms ticks) → postgres ← typescript agents (claude)
                              ↓                          ↑            ↓
                        indicator scores              trades    config mutations
                              ↓                                      ↓
                        entry/exit actions              backtest validation
                              ↓                                      ↓
                        simulated broker                  promote or reject
```

the engine runs a configurable set of indicators and actions loaded from a strategy config stored in postgres. agents analyze trade history and propose config mutations (adjust indicator weights, tune action parameters, enable/disable tools). each mutation is backtested before promotion. the engine hot-reloads promoted configs with zero downtime.

## prerequisites

- rust 1.82+
- node.js 22+
- docker (for postgres 16)
- [alpaca](https://alpaca.markets) paper trading account (free)
- [anthropic](https://console.anthropic.com) API key

## quick start

```bash
# 1. clone and configure
cp .env.example .env
# edit .env with your API keys

# 2. start postgres and run migrations
docker compose up -d postgres
cargo install sqlx-cli
sqlx migrate run

# 3. build and test
cargo build --workspace
cargo test --workspace
cd agents-ts && npm install && npx vitest run && cd ..

# 4. run a backtest
cargo run -p backtest -- --date 2026-02-28 --lookback-days 5

# 5. start paper trading
cargo run -p data_feed

# 6. run an agent cycle (in another terminal)
cd agents-ts && node dist/orchestrator.js --mode once
```

## docker deployment

runs the full stack: postgres, paper trader, and agent orchestrator.

```bash
docker compose up -d
docker compose logs -f paper_trader    # watch trades
docker compose logs -f agents-ts       # watch agent cycles
docker compose down                    # graceful shutdown
```

## environment variables

| variable | required | default | purpose |
|----------|----------|---------|---------|
| `DATABASE_URL` | yes | — | postgres connection string |
| `ANTHROPIC_API_KEY` | for agents | — | claude API access |
| `APCA_API_KEY_ID` | for live data | — | alpaca API key |
| `APCA_API_SECRET_KEY` | for live data | — | alpaca API secret |
| `BROKER_MODE` | no | `simulated` | `simulated` or `alpaca` |
| `INITIAL_CAPITAL` | no | `100000` | starting capital for simulated broker |
| `BACKTEST_DATA_DIR` | no | — | if set, agents must pass backtest before promoting configs |
| `RUST_LOG` | no | `info` | log level filter |

## running tests

```bash
# rust — 326 tests
cargo test --workspace
cargo clippy --workspace -- -D warnings

# typescript — 72 tests
cd agents-ts
npx vitest run
npx tsc --noEmit
```

## tech stack

| component | technology |
|-----------|------------|
| execution engine | rust, ta-rs v0.5, tokio, sqlx |
| market data | alpaca websocket (1-min bars) |
| agents | claude agent sdk, opus 4.6 / sonnet 4.6 |
| database | postgres 16 |
| deployment | docker compose |
| TUI | ratatui (feature-gated: `--features tui`) |

## documentation

- [`CLAUDE.md`](CLAUDE.md) — architecture, constraints, and build commands (for AI agents)
- [`docs/index.md`](docs/index.md) — comprehensive project index with per-file breakdowns
- [`docs/cli_usage_guide.md`](docs/cli_usage_guide.md) — full CLI reference
- [`docs/usage_guide.md`](docs/usage_guide.md) — operations and deployment guide
- [`docs/hardening_plan.md`](docs/hardening_plan.md) — safety guardrails for real capital
