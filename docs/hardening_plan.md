# hardening plan

implementation plan for securing the trading system before deploying with real capital. organized into three tiers by priority.

---

## current state

the system has some safety mechanisms:

| mechanism | location | what it does |
|-----------|----------|-------------|
| per-position stop loss | `actions/src/exit/fixed_pct_stop.rs` | exits when loss >= configured % (default 1.5%) |
| ATR trailing stop | `actions/src/exit/atr_trailing_stop.rs` | volatility-adjusted trailing stop |
| max hold timeout | `actions/src/exit/max_hold_timeout.rs` | force-close after configured duration (default 45min) |
| session close | `actions/src/exit/session_close.rs` | force-close all positions by 15:55 ET |
| hard gates | `engine/src/scoring.rs` | blocks entry when gated timescale score <= 0 |
| max concurrent positions | `data_feed/src/main.rs:336` | skips entry when at capacity (default 2) |
| agent budget cap | `agents-ts/src/orchestrator.ts` | stops agent runs when daily cost >= $5 |

**critical gaps:**

- `max_capital_deployed_pct` (0.10) is defined in `SessionConfig` but **never enforced** anywhere
- short selling is fully enabled — `TradeDirection::Short` can produce unlimited losses
- no account-level drawdown circuit breaker
- no daily loss limit
- `AlpacaBroker` returns `"not yet implemented"` — can't actually trade real money
- agent configs auto-promote when `BACKTEST_DATA_DIR` is not set
- no pre-entry equity/buying power verification

---

## tier 1: must-have before real money

these items directly prevent catastrophic loss scenarios.

### 1.1 disable short selling by default

**problem**: `ScoreThresholdEntry` (`actions/src/entry/score_threshold.rs:48-53`) enters short positions when composite <= `short_threshold` (-0.45 in seed config). short positions have theoretically unlimited loss.

**change**: add a `short_enabled` boolean parameter to `score_threshold_entry`. default to `false`. when disabled, the `composite <= short_threshold` branch returns `Hold` instead of `Enter { direction: Short }`.

**files**:
- `crates/actions/src/entry/score_threshold.rs` — add field, gate the short branch
- `migrations/20260228000010_seed_initial_config.sql` — add `"short_enabled": false` to entry params

**validation**: existing tests pass. add test confirming short signals are suppressed when `short_enabled = false`.

### 1.2 enforce `max_capital_deployed_pct`

**problem**: `SessionConfig.max_capital_deployed_pct` (0.10) is deserialized but never checked before opening positions. the tick loop at `engine/src/tick_loop.rs:148-154` opens positions without verifying total exposure.

**change**: track cumulative deployed capital in `TradingEngine`. before opening a position, check that `(current_deployed + proposed_size) / capital <= max_capital_deployed_pct`. reject entry if it would exceed the limit.

**approach**: add `session_config: SessionConfig` and `deployed_capital: f64` fields to `TradingEngine`. update `deployed_capital` on open/close. check the limit in the entry branch of `on_tick()` before calling `open_position()`.

**files**:
- `crates/engine/src/tick_loop.rs` — add fields, add pre-entry check, update on close
- `crates/backtest/src/replay.rs` — pass session config to engine
- `crates/data_feed/src/config_watcher.rs` — pass session config to engine
- all `TradingEngine::new()` call sites (tests, live_session, etc.)

**validation**: add test that entry is rejected when exposure would exceed limit. verify existing tests still pass with default 0.10 limit.

### 1.3 account-level drawdown circuit breaker

**problem**: no mechanism to stop trading when cumulative losses reach a dangerous level. the system will keep opening positions even after losing 50% of capital.

**change**: add a `daily_max_drawdown_pct` parameter (default 0.03 = 3% of capital). track `daily_pnl` inside `TradingEngine` (or pass it in as context). when `daily_pnl / capital <= -daily_max_drawdown_pct`, suppress all new entries for the rest of the session.

**approach — main loop level**: this is best enforced in `main.rs` since `daily_pnl` is already tracked there (line 264). add a `circuit_breaker_tripped` bool. when `daily_pnl / capital <= -threshold`, set it to true and log a warning. skip `session.on_tick()` entry evaluation (still process exits for open positions). add `DAILY_MAX_DRAWDOWN_PCT` env var.

**files**:
- `crates/data_feed/src/main.rs` — add circuit breaker logic in main loop
- `docker-compose.yml` — add `DAILY_MAX_DRAWDOWN_PCT` env var

**validation**: manual test with `--demo` mode and low capital to verify circuit breaker fires. unit test the threshold math.

### 1.4 daily loss limit

**problem**: related to but distinct from drawdown. the drawdown circuit breaker stops new entries. a daily loss limit adds an absolute dollar floor.

**change**: add `DAILY_LOSS_LIMIT` env var (default: no limit). when cumulative `daily_pnl <= -DAILY_LOSS_LIMIT`, force-close all open positions and halt trading.

**implementation**: in the main `select!` loop, after updating `daily_pnl` (line 414), check against the limit. if exceeded, iterate all sessions, close positions, log the event, and break.

**files**:
- `crates/data_feed/src/main.rs` — add loss limit check after trade completion

**validation**: manual test with low limit in demo mode.

### 1.5 absolute position size cap

**problem**: `size_fraction` from sizing actions is unbounded. if the sizing action returns 1.0 (or a bug produces > 1.0), the engine deploys `1.0 * capital` on a single position.

**change**: clamp `size_fraction` in `on_tick()` to `[0.0, max_position_fraction]` where `max_position_fraction` is a new field (default 0.05 = 5% of capital per position). this is a hard safety clamp independent of the sizing action's output.

**location**: `engine/src/tick_loop.rs:152`, right before `open_position()`:
```rust
let clamped = size_fraction.clamp(0.0, self.max_position_fraction);
let size = clamped * self.capital;
```

**files**:
- `crates/engine/src/tick_loop.rs` — add field, clamp before open
- `crates/types/src/config.rs` — add `max_position_size_pct` to `SessionConfig`
- `migrations/` — add to seed config

**validation**: test that oversized `size_fraction` values are clamped.

### 1.6 pre-entry buying power check

**problem**: the engine computes position size from its internal `capital` value, which is set once at startup and never updated. after losses, the actual account balance is lower than `capital`, so positions are oversized relative to real funds.

**change**: when `broker_mode == "alpaca_paper"`, periodically refresh account equity from alpaca (reuse `fetch_alpaca_equity` from `account.rs`). use the minimum of `(startup_capital, current_equity)` for position sizing.

**approach**: add a `capital_refresh_interval` (default 5 minutes). in the main loop's config reload timer branch, also refresh capital. store `effective_capital` and pass it to engines on rebuild or via a setter.

**files**:
- `crates/data_feed/src/main.rs` — add capital refresh logic
- `crates/data_feed/src/account.rs` — already has `fetch_alpaca_equity()`
- `crates/engine/src/tick_loop.rs` — add `set_capital()` method

**validation**: test with simulated mode (no API call). verify capital never increases beyond startup value.

### 1.7 implement `AlpacaBroker`

**problem**: `AlpacaBroker` (`data_feed/src/broker.rs:164-182`) returns `"not yet implemented"` for both `submit_order` and `close_position`. cannot actually trade.

**change**: implement using `apca::api::v2::order` API. support market orders only (no limit/stop). map `TradeDirection::Long` to buy, `Short` to sell-short. map `close_position` to close-all-for-symbol.

**dependencies**: the `apca` crate (v0.30) is already in `Cargo.toml`. `apca::api::v2::order` provides `OrderReq`, `Side`, `Type`, etc.

**files**:
- `crates/data_feed/src/broker.rs` — implement `AlpacaBroker` methods
- `crates/data_feed/src/main.rs` — wire up `AlpacaBroker` when `broker_mode == "alpaca_paper"`

**validation**: test against alpaca paper trading sandbox. add unit test that constructs broker (existing test already passes).

### 1.8 require backtest validation for config promotion

**problem**: in `orchestrator.ts:484-498`, when `BACKTEST_DATA_DIR` is not set, the PM agent's proposed config is auto-promoted without validation.

**change**: remove the auto-promote fallback. if `BACKTEST_DATA_DIR` is not set, reject the proposal and log a warning. this forces operators to configure backtest data before agents can mutate the live config.

**alternative**: require the PM agent to call a backtest tool before proposing (your earlier idea about 5-day validation). this is more complex but more useful long-term. for now, the simpler "reject without backtest" gate is sufficient.

**files**:
- `agents-ts/src/orchestrator.ts` — change auto-promote to reject

**validation**: existing orchestrator tests. add test confirming proposals are rejected without `BACKTEST_DATA_DIR`.

---

## tier 2: operational safety for deployment

these items ensure the system runs reliably in a container environment.

### 2.1 separate read-only API keys

**problem**: the same alpaca credentials are used for data streaming and order execution. if compromised, an attacker has full trading access.

**change**: support two credential pairs:
- `APCA_DATA_KEY_ID` / `APCA_DATA_SECRET_KEY` — for `AlpacaFeed` (market data only)
- `APCA_API_KEY_ID` / `APCA_API_SECRET_KEY` — for `AlpacaBroker` (order execution)

fall back to the trading credentials for data if the data-only keys aren't set (backwards compatible).

**files**:
- `crates/data_feed/src/main.rs` — read separate env vars, pass to `AlpacaFeed` vs `AlpacaBroker`
- `docker-compose.yml` — add data key env vars

### 2.2 container hardening

**problem**: `Dockerfile.paper_trader` runs as root, has a shell (`tmux`), and uses the default debian image with no resource constraints.

**changes**:

**non-root user**:
```dockerfile
RUN useradd -r -s /usr/sbin/nologin trader
USER trader
```

**read-only filesystem**: add `read_only: true` to docker-compose with explicit tmpfs for `/tmp` and writable volume for `/data/logs`.

**resource limits**: add to docker-compose:
```yaml
deploy:
  resources:
    limits:
      cpus: '2.0'
      memory: 1G
```

**no shell (non-TUI mode)**: for headless production deployment, use a distroless or scratch base image instead of debian. keep the tmux variant as `Dockerfile.paper_trader.tui` for interactive debugging.

**files**:
- `Dockerfile.paper_trader` — add non-root user, consider multi-target for headless vs TUI
- `docker-compose.yml` — add `read_only`, `tmpfs`, resource limits

### 2.3 secrets management

**problem**: credentials are passed as plaintext environment variables in `docker-compose.yml`. anyone with access to the compose file or `docker inspect` can read them.

**change**: use docker secrets for sensitive values. in docker-compose:
```yaml
secrets:
  apca_api_key:
    file: ./secrets/apca_api_key.txt
  apca_api_secret:
    file: ./secrets/apca_api_secret.txt
```

update the rust binary to read from `/run/secrets/<name>` when the env var is empty. add a helper function:
```rust
fn read_secret(env_var: &str, secret_name: &str) -> String {
    std::env::var(env_var)
        .or_else(|_| std::fs::read_to_string(format!("/run/secrets/{}", secret_name)))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}
```

**files**:
- `crates/data_feed/src/main.rs` — add `read_secret()` helper, use for all credentials
- `docker-compose.yml` — add secrets section
- `.gitignore` — add `secrets/`

### 2.4 health check endpoint

**problem**: no way for external monitoring to know if the paper trader is alive and functioning. docker's `healthcheck` only checks postgres.

**change**: add a minimal HTTP health check server on a configurable port (default 8080). responds to `GET /health` with:
```json
{"status": "ok", "tick_count": 12345, "daily_pnl": -23.45, "circuit_breaker": false, "uptime_s": 3600}
```

use `axum` or `hyper` (lightweight). spawn as a separate tokio task. expose in docker-compose:
```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
  interval: 30s
```

**files**:
- `crates/data_feed/src/main.rs` — spawn health server task, share state via `Arc<RwLock<HealthState>>`
- `crates/data_feed/Cargo.toml` — add `axum` dependency (or minimal hyper)
- `docker-compose.yml` — add healthcheck and port mapping

### 2.5 structured alerting

**problem**: critical events (circuit breaker trips, position errors, agent failures) are only logged. no way to get notified.

**change**: add a webhook-based alerting module. on critical events, POST a JSON payload to a configurable `ALERT_WEBHOOK_URL` (works with slack incoming webhooks, discord, or any HTTP endpoint).

alert-worthy events:
- circuit breaker tripped
- daily loss limit hit
- trade execution error (broker rejection)
- config hot-reload failure
- agent budget exhausted
- position force-closed on shutdown

**files**:
- `crates/data_feed/src/alerting.rs` — new module with `send_alert(event, details)` async fn
- `crates/data_feed/src/lib.rs` — register module
- `crates/data_feed/src/main.rs` — call `send_alert()` at each critical point

### 2.6 stale data detection

**problem**: if the alpaca websocket stream drops and reconnects, the engine might trade on stale market data during the gap. the reconnection logic in `alpaca_feed.rs` doesn't notify the engine that data is potentially stale.

**change**: track `last_bar_received_at` per ticker in the main loop. if no bar arrives for a ticker within a configurable timeout (default 120 seconds during market hours), mark that ticker as stale. skip entry evaluation for stale tickers (still process exits). log a warning.

**files**:
- `crates/data_feed/src/main.rs` — add staleness tracking per ticker

### 2.7 log redaction

**problem**: if a log statement accidentally includes an API key or secret (e.g., in an error message from the apca crate), it would be written to the log file in plaintext.

**change**: add a tracing layer that redacts known secret patterns. at minimum, redact any string that matches the loaded API key or secret values. implement as a custom `tracing_subscriber::Layer` that filters field values.

**alternative (simpler)**: audit all `tracing::` calls and `eprintln!` calls to confirm no secrets can leak. add a grep-based CI check that flags log statements containing `api_key`, `api_secret`, `password`, etc.

**files**:
- `crates/data_feed/src/main.rs` — add redaction layer or audit + CI check

---

## tier 3: defense in depth

these items add redundant safety layers and operational maturity.

### 3.1 alpaca account-level restrictions

**problem**: even with code-level guardrails, a bug could bypass them. belt-and-suspenders means also setting limits at the broker level.

**actions** (manual, not code changes):
- disable margin on the alpaca account (cash account only) — eliminates leverage risk
- disable short selling at the account level — redundant with 1.1 but independent
- set position size limits via alpaca's API if available
- use alpaca's built-in order validation

**documentation**: add a `docs/alpaca_setup.md` checklist for account configuration.

### 3.2 separate API key scopes

**problem**: a single API key pair has full access to the account. if the data feed is compromised, the attacker can place orders.

**change**: use two alpaca sub-accounts or API key pairs:
- data-only key (market data subscription, no trading permissions)
- trading key (order execution, minimal data access)

this extends 2.1 by actually creating separate keys in alpaca, not just supporting two env vars.

**documentation**: add to `docs/alpaca_setup.md`.

### 3.3 network isolation

**problem**: the container can make arbitrary outbound connections. a supply chain attack or code injection could exfiltrate data.

**change**: add network constraints in docker-compose:
```yaml
networks:
  trading:
    driver: bridge
  internal:
    internal: true

services:
  postgres:
    networks: [internal]
  paper_trader:
    networks: [trading, internal]
  agents-ts:
    networks: [trading, internal]
```

for tighter control, use iptables or a reverse proxy to allowlist only:
- `*.alpaca.markets` (trading API + data)
- `api.anthropic.com` (agent layer)
- the postgres container (internal)

**files**:
- `docker-compose.yml` — add network definitions

### 3.4 order rate limiting

**problem**: a bug in the scoring pipeline could cause rapid entry/exit cycling, generating many orders per minute (churning).

**change**: add a `min_time_between_entries_ms` parameter (default 60000 = 1 minute). after opening a position for a ticker, suppress new entries for that ticker until the cooldown expires. track `last_entry_time` per ticker.

**files**:
- `crates/engine/src/tick_loop.rs` — add cooldown tracking and enforcement
- `crates/types/src/config.rs` — add `min_time_between_entries_ms` to `SessionConfig`

### 3.5 position reconciliation

**problem**: the engine's internal position state could drift from alpaca's actual positions (e.g., alpaca fills a partial order, or an order is rejected after the engine assumes it succeeded).

**change**: add a periodic reconciliation task that:
1. fetches open positions from alpaca (`apca::api::v2::positions`)
2. compares with the engine's internal `PositionManager` state
3. if they disagree, logs an alert and optionally force-syncs

run every 5 minutes during market hours.

**files**:
- `crates/data_feed/src/reconciliation.rs` — new module
- `crates/data_feed/src/lib.rs` — register module
- `crates/data_feed/src/main.rs` — spawn reconciliation task

### 3.6 kill switch

**problem**: no way to remotely halt trading without SSH access to the machine or docker host.

**change**: add a simple HTTP endpoint `POST /kill` (on the same health check server from 2.4) that:
1. trips the circuit breaker
2. force-closes all positions
3. logs the event
4. sends an alert via webhook

protect with a shared secret (`KILL_SWITCH_TOKEN` env var) — the request must include the token in the `Authorization` header.

**files**:
- `crates/data_feed/src/main.rs` — add `/kill` endpoint to health server

### 3.7 audit log

**problem**: trade decisions are logged but not in an easily queryable, immutable format. hard to do post-mortem analysis.

**change**: add an `audit_log` table:
```sql
CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    event_type TEXT NOT NULL,  -- 'entry_signal', 'exit_signal', 'order_submitted', 'order_filled', 'circuit_breaker', etc.
    ticker TEXT,
    details JSONB NOT NULL,    -- full context: scores, config version, position state, etc.
    config_version_id BIGINT REFERENCES config_versions(id)
);
```

write an audit entry for every trade decision (entry/exit), circuit breaker event, config change, and agent action.

**files**:
- `migrations/` — new migration for `audit_log` table
- `crates/data_feed/src/audit.rs` — new module with `write_audit_entry()` async fn
- `crates/data_feed/src/main.rs` — call at each decision point

---

## implementation order

recommended sequence, roughly by risk reduction per effort:

| order | item | effort | risk reduced |
|-------|------|--------|-------------|
| 1 | 1.1 disable short selling | small | eliminates unlimited loss |
| 2 | 1.5 position size cap | small | prevents single-trade blowup |
| 3 | 1.2 enforce max capital deployed | medium | prevents overexposure |
| 4 | 1.3 drawdown circuit breaker | medium | stops cascading losses |
| 5 | 1.4 daily loss limit | small | hard dollar floor |
| 6 | 1.8 require backtest for promotion | small | prevents untested configs |
| 7 | 2.2 container hardening | small | reduces attack surface |
| 8 | 2.3 secrets management | medium | protects credentials |
| 9 | 1.7 implement AlpacaBroker | large | enables real trading |
| 10 | 1.6 pre-entry buying power check | medium | prevents oversized positions |
| 11 | 2.4 health check endpoint | medium | enables monitoring |
| 12 | 2.5 structured alerting | medium | enables notification |
| 13 | 2.6 stale data detection | small | prevents stale-data trades |
| 14 | 2.1 separate API keys | small | limits blast radius |
| 15 | 2.7 log redaction | small | prevents credential leaks |
| 16 | 3.4 order rate limiting | small | prevents churning |
| 17 | 3.1 alpaca account restrictions | none (manual) | broker-level safety |
| 18 | 3.3 network isolation | small | limits exfiltration |
| 19 | 3.6 kill switch | medium | enables remote halt |
| 20 | 3.5 position reconciliation | large | prevents state drift |
| 21 | 3.7 audit log | medium | enables post-mortem |
| 22 | 3.2 separate API key scopes | small (manual) | further isolation |

---

## verification checklist

before deploying with real money, all of these must be true:

- [ ] short selling disabled (or explicitly opted in with understanding of risk)
- [ ] position size clamped to max % of capital
- [ ] `max_capital_deployed_pct` enforced in tick loop
- [ ] drawdown circuit breaker tested and configured
- [ ] daily loss limit configured
- [ ] `AlpacaBroker` implemented and tested against paper trading
- [ ] config promotion requires backtest validation
- [ ] container runs as non-root with resource limits
- [ ] secrets not in plaintext env vars or compose files
- [ ] all rust tests pass (`cargo test --workspace`)
- [ ] clippy clean (`cargo clippy --workspace -- -D warnings`)
- [ ] at least 5 full trading days on paper with all guardrails active
- [ ] manual review of all trades from paper period
- [ ] alpaca account configured as cash-only (no margin)
