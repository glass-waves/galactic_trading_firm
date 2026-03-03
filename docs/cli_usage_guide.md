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

## orchestrator (agent layer)

runs the AI agent evolution cycles — check-ins (observation-only) and full PM cycles (config authority).

```bash
# from agents-ts/
npx tsx src/orchestrator.ts --mode <MODE>

# or after building:
node dist/orchestrator.js --mode <MODE>
```

| flag | default | description |
|------|---------|-------------|
| `--mode` | `once` | execution mode (see below) |
| `--interval` | `15` | minutes between cycles in `checkin` mode |

### modes

| mode | what it does |
|------|-------------|
| `once` | run one check-in cycle (3 agents), then exit |
| `pm` | run one full PM cycle (3 recommendation agents + PM agent), then exit |
| `scheduled` | start the cron scheduler — check-ins at 10am/12pm/2pm ET, PM at 3:30pm ET, weekdays only |
| `checkin` | run check-in cycles in a loop every `--interval` minutes until budget is exhausted |

examples:

```bash
# quick smoke test — run one check-in
npx tsx src/orchestrator.ts --mode once

# run one PM cycle (will use claude-opus-4-6 by default)
npx tsx src/orchestrator.ts --mode pm

# production mode — scheduled cron
npx tsx src/orchestrator.ts --mode scheduled

# run check-ins every 30 minutes
npx tsx src/orchestrator.ts --mode checkin --interval 30
```

the orchestrator enforces a $5/day budget cap by default. budget tracking is per trading date in the `daily_budget` table.

---

## walk-forward simulation

simulates multi-day trading where agents iterate on config day-by-day. each day: backtest → PM cycle → next day uses the (potentially updated) config.

```bash
# from agents-ts/
npm run walk-forward -- --start 2025-10-01 --end 2025-10-14
```

| flag | default | description |
|------|---------|-------------|
| `--start` | required | first trading date (`YYYY-MM-DD`) |
| `--end` | required | last trading date (`YYYY-MM-DD`) |
| `--lookback-days` | `5` | passed to the backtest for indicator warmup |
| `--model` | `sonnet` | model for PM agents: `sonnet` or `opus` |
| `--budget-limit` | `50` | daily budget cap in USD (per simulated day) |

weekends are automatically skipped.

examples:

```bash
# one trading week with defaults
npm run walk-forward -- --start 2025-10-06 --end 2025-10-10

# two weeks with longer warmup
npm run walk-forward -- --start 2025-10-01 --end 2025-10-14 --lookback-days 10

# use opus for higher-quality reasoning
npm run walk-forward -- --start 2025-10-06 --end 2025-10-10 --model opus

# tighter budget for cost control
npm run walk-forward -- --start 2025-10-06 --end 2025-10-10 --budget-limit 10
```

### what happens each day

1. `cargo run -p backtest -- --date $DATE --lookback-days N --write-db` — backtests and writes trades to postgres
2. `runFullPmCycle()` — agents analyze the day's trades, may propose and auto-promote a config mutation
3. queries the database for trade summary (count, P&L, win rate)
4. prints a day summary line

at the end, prints a table like:

```
=========================================================================
walk-forward simulation summary
=========================================================================
date          trades          P&L  win rate   config  promoted
---------------------------------------------------------------------------
2025-10-06         4      +$127.50    75.0%       v2       yes
2025-10-07         3       -$43.20    33.3%       v3        no
2025-10-08         5      +$210.00    80.0%       v3       yes
---------------------------------------------------------------------------
total             12      +$294.30
=========================================================================
```

### cost considerations

each simulated day runs a full PM cycle (3 recommendation agents + 1 PM agent). approximate costs per day:

| model | estimated cost/day |
|-------|-------------------|
| `sonnet` (claude-sonnet-4-6) | ~$0.50–2.00 |
| `opus` (claude-opus-4-6) | ~$2.00–8.00 |

a 10-day simulation with sonnet typically costs $5–20 total. use `--budget-limit` to cap spend per simulated day.

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
| `agents-ts` | built from Dockerfile.agents-ts | — | typescript agent orchestrator |

environment variables are passed through from the host `.env` file. the database URL inside docker uses `postgres:5432` (container networking), while the host uses `127.0.0.1:5433` (mapped port).

---

## npm scripts (agents-ts)

run from the `agents-ts/` directory:

| script | command | description |
|--------|---------|-------------|
| `npm run build` | `tsc` | compile typescript to `dist/` |
| `npm run typecheck` | `tsc --noEmit` | type-check without emitting files |
| `npm test` | `vitest run` | run test suite |
| `npm run test:watch` | `vitest` | run tests in watch mode |
| `npm run lint` | `biome check src/ tests/` | lint with biome |
| `npm run format` | `biome format --write src/ tests/` | auto-format |
| `npm start` | `node dist/orchestrator.js --mode scheduled` | start scheduled orchestrator |
| `npm run walk-forward` | `tsx src/walk-forward.ts` | walk-forward simulation (pass args after `--`) |

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
cd agents-ts && npm test              # run ts tests
```

### run a quick backtest

```bash
cargo run -p backtest -- --date 2025-10-06
```

### test the agent layer

```bash
cd agents-ts
npx tsx src/orchestrator.ts --mode once    # one check-in cycle
npx tsx src/orchestrator.ts --mode pm      # one PM cycle
```

### simulate a trading week

```bash
cd agents-ts
npm run walk-forward -- --start 2025-10-06 --end 2025-10-10 --model sonnet
```

### run the full live stack

```bash
docker-compose up
# or without docker:
cargo run -p data_feed &                                    # paper trader
cd agents-ts && npx tsx src/orchestrator.ts --mode scheduled  # agents
```
