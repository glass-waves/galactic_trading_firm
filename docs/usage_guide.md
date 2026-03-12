# usage guide

## deploy to a new machine

### prerequisites

- docker desktop ([mac install](https://docs.docker.com/desktop/install/mac-install/))
- git (to clone the repo)
- alpaca api credentials (paper trading account)
- anthropic api key

### setup

```bash
# 1. clone the repo
git clone <your-repo-url> ~/galactic_trading_firm
cd ~/galactic_trading_firm

# 2. create .env file
cat > .env << 'EOF'
APCA_API_KEY_ID=your-alpaca-key
APCA_API_SECRET_KEY=your-alpaca-secret
ANTHROPIC_API_KEY=your-anthropic-key
BROKER_MODE=simulated
INITIAL_CAPITAL=100000
RUST_LOG=info
EOF

# 3. start everything
docker compose up -d
```

that's it. docker compose handles the rest:

1. starts postgres
2. runs all database migrations (creates tables, seeds config)
3. starts paper_trader (rust execution engine)
4. starts agents-ts (orchestrator in scheduled mode)

first build takes a few minutes (rust compilation). subsequent starts are instant.

all services have `restart: unless-stopped` — they survive crashes and reboots as long as Docker Desktop is running.

### verify

```bash
docker compose ps                # all services should be running (migrate will show "exited 0")
docker compose logs paper_trader  # check execution engine
docker compose logs agents-ts    # check agent orchestrator
```

### attach to the TUI

```bash
docker exec -it galactic_trading_firm-paper_trader-1 tmux attach -t trader
# detach with ctrl-b d
```

### updating

```bash
cd ~/galactic_trading_firm
git pull
docker compose up -d --build
```

the migrate service automatically applies any new migrations on restart.

### shutdown

```bash
docker compose down       # stop everything (data persists in docker volumes)
docker compose down -v    # stop and DELETE all data (fresh start)
```

---

## agent schedule

all times US/Eastern, weekdays only:

| time | cycle | model | purpose |
|------|-------|-------|---------|
| 12:30 | mid-day analysis | sonnet 4.6 | observation memo with structured suggestions |
| 15:30 | full PM cycle | opus 4.6 | analysis + PM decision → config mutation or hold steady |

budget enforcement: $5/day cap. exhausted budget skips remaining cycles.

### manual agent runs

```bash
# single analysis cycle
docker compose exec agents-ts node dist/orchestrator.js --mode once

# single PM cycle
docker compose exec agents-ts node dist/orchestrator.js --mode pm
```

---

## local development

for working on the code without docker:

```bash
# start just postgres
docker compose up -d postgres
# wait for healthy, then run migrations
DATABASE_URL=postgres://postgres:postgres@localhost:5433/adaptive_trading sqlx migrate run

# rust — build and test
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# run paper_trader locally
cargo run -p data_feed --release
# with TUI
cargo run -p data_feed --release --features tui

# typescript agents — build and test
cd agents-ts && npm install
npx tsc
npx vitest run

# run orchestrator locally
node dist/orchestrator.js --mode once
```

## backtest

```bash
cargo run -p backtest --release -- \
    --date 2025-01-15 --lookback-days 20

# with verbose output and equity curve
cargo run -p backtest --release -- \
    --date 2025-01-15 --lookback-days 20 --verbose --output-equity
```

or use the convenience scripts:

```bash
scripts/backtest_20days.sh    # 20-day window
scripts/backtest_100days.sh   # 100-day window
scripts/backtest_2022bear.sh  # 2022 bear market stress test
```

## config management

configs follow immutable append-only versioning: proposed → backtesting → validated → promoted.

the execution engine polls postgres every 60 seconds for new promoted configs and hot-reloads automatically. rollback = promote an older version.

## key operational notes

- **intraday only** — no overnight holds. `session_close` action forces exit before market close.
- **config hot-reload** — engine picks up newly promoted configs within 60 seconds.
- **broker modes** — `simulated` (default) or `alpaca` (live paper trading through alpaca).
- **demo mode** — pass `--demo` to paper_trader for synthetic data (no alpaca credentials needed).
