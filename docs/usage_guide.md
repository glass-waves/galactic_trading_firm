# usage guide

how to run, deploy, and operate the adaptive multi-timescale trading system.

## prerequisites

- rust 1.82+ (for building the execution engine)
- python 3.10+ (for the agent layer)
- docker & docker compose (for containerized deployment)
- postgres 16 (runs via docker compose)
- alpaca api credentials (for live/paper market data; optional for demo mode)
- anthropic api key (for agent evolution cycles)

## quick start (local development)

### 1. environment setup

```bash
cp .env.example .env
# edit .env with your credentials
```

| variable | purpose | example |
|----------|---------|---------|
| `DATABASE_URL` | postgres connection (rust uses `postgres://`, python uses `postgresql+asyncpg://`) | `postgres://postgres:postgres@localhost:5433/adaptive_trading` |
| `ANTHROPIC_API_KEY` | anthropic api for agent cycles | `sk-ant-...` |
| `APCA_API_KEY_ID` | alpaca api key (optional) | |
| `APCA_API_SECRET_KEY` | alpaca api secret (optional) | |
| `BROKER_MODE` | `simulated` or `alpaca_paper` | `simulated` |
| `BACKTEST_DATA_DIR` | directory with CSV data for backtest validation | |
| `RUST_LOG` | rust log level | `info` |

### 2. start postgres

```bash
docker compose up -d postgres
```

### 3. run database migrations

```bash
sqlx migrate run
```

### 4. build and run the execution engine

```bash
# headless mode
cargo run -p data_feed --release

# with TUI dashboard
cargo run -p data_feed --release --features tui

# demo mode (synthetic data, no alpaca credentials needed)
cargo run -p data_feed --release --features tui -- --demo
```

### 5. run agents

```bash
cd agents
pip install -e ".[dev]"

# single check-in cycle
python -m agents.orchestrator --mode once

# single full PM cycle
python -m agents.orchestrator --mode pm

# sleep-loop check-ins (every 15 min)
python -m agents.orchestrator --mode checkin --interval 15

# production cron schedule (market-hours aware)
python -m agents.orchestrator --mode scheduled
```

## docker deployment

the full stack runs as three containers: postgres, paper_trader, and agents.

### build and start

```bash
docker compose up -d
```

this starts:
- **postgres** — database with healthcheck
- **paper_trader** — rust execution engine inside a tmux session (waits for healthy postgres)
- **agents** — python orchestrator in `scheduled` mode (waits for healthy postgres)

### attach to the TUI

the paper_trader container runs inside tmux so you can view the live TUI dashboard over SSH:

```bash
docker exec -it galactic_trading_firm-paper_trader-1 tmux attach -t trader
```

detach with `ctrl-b d` (standard tmux detach). the trading engine continues running.

### view logs

```bash
# all services
docker compose logs -f

# specific service
docker compose logs -f agents
docker compose logs -f paper_trader
```

### trigger a manual cycle

```bash
# manual check-in
docker exec galactic_trading_firm-agents-1 python -m agents.orchestrator --mode once

# manual PM cycle
docker exec galactic_trading_firm-agents-1 python -m agents.orchestrator --mode pm
```

### graceful shutdown

```bash
docker compose down
```

both services handle SIGTERM gracefully:
- **paper_trader** — logs open positions, prints session stats, exits cleanly (30s grace period)
- **agents** — stops the apscheduler, waits for any in-flight cycle to finish, closes db (15s grace period)

### rebuild after code changes

```bash
docker compose build
docker compose up -d
```

## agent schedule

when running in `--mode scheduled`, agents follow a market-hours-aware cron schedule (all times US/Eastern, weekdays only):

| time | cycle | model | purpose |
|------|-------|-------|---------|
| 10:00 | check-in | haiku 4.5 | morning observation memos |
| 12:00 | check-in | haiku 4.5 | midday observation memos |
| 14:00 | check-in | haiku 4.5 | afternoon observation memos |
| 15:30 | full PM | sonnet 4.5 | EOD config evolution: recommendations → PM decision → backtest → promote/reject |

budget enforcement ($5/day cap) applies across all cycles. if the budget is exhausted, scheduled cycles log a warning and skip.

## running tests

### rust

```bash
# full workspace
cargo test --workspace

# single crate
cargo test -p data_feed
cargo test -p engine
cargo test -p indicators

# single test by name
cargo test -p indicators -- ema

# with TUI tests
cargo test -p data_feed --features tui

# lint
cargo clippy --workspace -- -D warnings
```

### python

```bash
cd agents
source .venv/bin/activate

# all tests
pytest tests/ -v

# single file
pytest tests/test_orchestrator.py -v

# lint
ruff check .
```

## backtest engine

replay historical data against a config to evaluate performance:

```bash
cargo run -p backtest --release -- \
    --config path/to/config.json \
    --data path/to/SPY.csv \
    --ticker SPY
```

CSV data format: `timestamp,open,high,low,close,volume` (1-minute bars).

output includes: total P&L, sharpe ratio, max drawdown, win rate, profit factor, trade log, and equity curve.

## config management

configs follow an immutable append-only versioning model:

1. **proposed** — PM agent creates a new config version
2. **backtesting** — backtest validation runs against historical data
3. **validated/rejected** — backtest passes or fails quality gates
4. **promoted** — config goes live; execution engine hot-reloads within 60 seconds

to roll back: promote an older config version. the engine picks up the latest promoted version automatically.

## key operational notes

- **no overnight holds** — the system is intraday only. `session_close` action forces exit before market close.
- **max concurrent positions** — configured in `strategy_config.session.max_concurrent_positions`. the engine enforces this limit.
- **config hot-reload** — the engine polls postgres every 60 seconds for new promoted configs. no restart needed.
- **demo mode** — pass `--demo` to paper_trader for a synthetic random-walk data feed (no alpaca credentials required).
- **broker modes** — `simulated` (default, 5 bps slippage model) or `alpaca_paper` (live paper trading through alpaca).
