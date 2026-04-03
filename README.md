# galactic trading firm

adaptive intraday trading system. a rust execution engine processes market ticks in real-time with configurable indicators and actions loaded from postgres.

trades SPY, QQQ, and liquid mega-caps. intraday only — all positions close before market close.

## how it works

```
market data (alpaca) → rust engine (sub-ms ticks) → postgres
                              ↓                          ↑
                        indicator scores            config versions
                              ↓
                        entry/exit actions
                              ↓
                        simulated broker → trades
```

the engine runs a configurable set of indicators and actions loaded from a strategy config stored in postgres. configs are versioned and immutable — the engine hot-reloads promoted configs with zero downtime. the backtest CLI validates changes against historical data before promotion.

## prerequisites

- rust 1.82+
- docker (for postgres 16)
- [alpaca](https://alpaca.markets) paper trading account (free)

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

# 4. run a backtest
cargo run -p backtest -- --date 2026-02-28 --lookback-days 5

# 5. start paper trading
cargo run -p data_feed
```

## docker deployment

runs the full stack: postgres, paper trader, and cockpit dashboard.

```bash
docker compose up -d
docker compose logs -f paper_trader    # watch trades
docker compose down                    # graceful shutdown
```

## environment variables

| variable | required | default | purpose |
|----------|----------|---------|---------|
| `DATABASE_URL` | yes | — | postgres connection string |
| `APCA_API_KEY_ID` | for live data | — | alpaca API key |
| `APCA_API_SECRET_KEY` | for live data | — | alpaca API secret |
| `BROKER_MODE` | no | `simulated` | `simulated` or `alpaca` |
| `INITIAL_CAPITAL` | no | `100000` | starting capital for simulated broker |
| `RUST_LOG` | no | `info` | log level filter |

## running tests

```bash
# rust — 326 tests
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## tech stack

| component | technology |
|-----------|------------|
| execution engine | rust, ta-rs v0.5, tokio, sqlx |
| market data | alpaca websocket (1-min bars) |
| database | postgres 16 |
| dashboard | next.js (cockpit) |
| deployment | docker compose |
| TUI | ratatui (feature-gated: `--features tui`) |

## documentation

- [`CLAUDE.md`](CLAUDE.md) — architecture, constraints, and build commands (for AI agents)
- [`docs/cli_usage_guide.md`](docs/cli_usage_guide.md) — full CLI reference
- [`docs/usage_guide.md`](docs/usage_guide.md) — operations and deployment guide
- [`docs/hardening_plan.md`](docs/hardening_plan.md) — safety guardrails for real capital
