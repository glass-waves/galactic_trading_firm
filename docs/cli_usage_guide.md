# cli usage guide

quick reference for every command-line entry point in the project.

## prerequisites

all commands expect a `.env` file at the project root (or equivalent environment variables):

```bash
DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5433/adaptive_trading
APCA_API_KEY_ID=<your alpaca key>
APCA_API_SECRET_KEY=<your alpaca secret>
ANTHROPIC_API_KEY=<your anthropic key>
```

postgres must be running. start it with:

```bash
docker-compose up -d postgres
sqlx migrate run
```

---

## backtest

runs the trading engine against historical data and reports results.

### date mode (recommended)

fetches bars from Alpaca, loads the promoted config from postgres, and backtests a single day.

```bash
cargo run -p backtest -- --date 2025-10-06
```

| flag | default | description |
|------|---------|-------------|
| `--date` | required | trading date to backtest (`YYYY-MM-DD`) |
| `--lookback-days` | `5` | calendar days of history before the target date, used for indicator warmup |
| `--write-db` | off | write completed trades into the `trades` table |

examples:

```bash
# basic single-day backtest
cargo run -p backtest -- --date 2025-10-06

# longer warmup window
cargo run -p backtest -- --date 2025-10-06 --lookback-days 10

# backtest and persist trades to the database
cargo run -p backtest -- --date 2025-10-06 --lookback-days 5 --write-db
```

output goes to stdout (summary table) and `logs/backtest_2025-10-06.log` (structured JSON). the `LOG_DIR` env var overrides the log directory.

### legacy mode

uses local files instead of Alpaca + postgres. useful for offline testing with CSV data.

```bash
cargo run -p backtest -- --config configs/default.json --data data/SPY.csv --ticker SPY
```

| flag | description |
|------|-------------|
| `--config` | path to a `StrategyConfig` JSON file |
| `--data` | path to a CSV file with candle data |
| `--ticker` | ticker symbol for the data |

outputs the full backtest result as JSON to stdout.

---

## paper trader

live (or simulated) intraday trading engine that streams market data, executes the scoring pipeline, and manages positions in real time.

```bash
cargo run -p data_feed
```

| flag | default | description |
|------|---------|-------------|
| `--demo` | off | run with a synthetic price feed instead of connecting to Alpaca |

with TUI dashboard (feature-gated):

```bash
cargo run -p data_feed --features tui
```

### environment variables

| variable | default | description |
|----------|---------|-------------|
| `DATABASE_URL` | required | postgres connection string |
| `APCA_API_KEY_ID` | — | Alpaca key; if missing, falls back to demo mode |
| `APCA_API_SECRET_KEY` | — | Alpaca secret; if missing, falls back to demo mode |
| `BROKER_MODE` | `simulated` | `simulated` or `alpaca_paper` |
| `RUST_LOG` | `info` | log level filter |
| `LOG_DIR` | `logs` | log output directory |

the engine hot-reloads the promoted config from postgres every 60 seconds. gracefully shuts down on SIGTERM or ctrl-c.

---

## docker compose

runs the full stack in containers.

```bash
# start everything
docker-compose up

# just postgres (for local dev)
docker-compose up -d postgres

# rebuild after code changes
docker-compose up --build
```

### services

| service | image | port | description |
|---------|-------|------|-------------|
| `postgres` | postgres:16 | 5433 → 5432 | database |
| `paper_trader` | built from Dockerfile | — | rust trading engine |
| `cockpit` | built from cockpit/Dockerfile | 3000 | monitoring dashboard |

environment variables are passed through from the host `.env` file. the database URL inside docker uses `postgres:5432` (container networking), while the host uses `127.0.0.1:5433` (mapped port).

---

## cargo commands

run from the project root:

```bash
# build everything
cargo build --workspace

# test everything
cargo test --workspace

# test a single crate
cargo test -p backtest

# test a single test by name
cargo test -p indicators -- ema

# lint
cargo clippy --workspace -- -D warnings
```

### workspace crates

| crate | type | description |
|-------|------|-------------|
| `types` | library | shared types, traits, config structs |
| `indicators` | library | indicator implementations (native + composable) |
| `actions` | library | action implementations (entry, exit, sizing) |
| `engine` | library | scoring pipeline, position manager, tick loop |
| `backtest` | binary + library | historical replay, metrics, reporting |
| `data_feed` | binary + library | live/paper trading, market data, TUI |

---

## typical workflows

### daily development

```bash
docker-compose up -d postgres        # start db
cargo test --workspace                # run rust tests
```

### run a quick backtest

```bash
cargo run -p backtest -- --date 2025-10-06
```

### run the full live stack

```bash
docker-compose up
# or without docker:
cargo run -p data_feed
```
