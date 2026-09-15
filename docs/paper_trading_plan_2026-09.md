# paper trading revival — status, week-1 plan, intraday agent design

**written:** 2026-09-11 (friday). last substantive commit was 2026-04-30 (`eab9c2b`).
**goal:** run the v11 config on the alpaca paper account the week of 2026-09-14, with a claude agent
watching the live output during market hours and making config adjustments that hot-reload into the engine.

this doc is the output of a full read of every crate, the migrations, the scripts, the cockpit, and the
archived agent design. every "critical" bug below was verified by hand in source, not just reported by a subagent.

---

## status update — 2026-09-11 (evening)

**everything in section 2 and the P0 observability list is implemented, tested, and verified against the real alpaca paper account.** 471 tests pass, clippy is clean (lib and TUI). nothing is committed yet.

| item | state |
|---|---|
| #1 share count | fixed. engine now sizes in **whole shares** and the broker gets exactly that quantity |
| #2 session close TZ | fixed in `session_close.rs` (US/Eastern) **and** the engine has its own force-exit safety net from `session.force_exit_by` |
| #3 config row id | fixed. `load_config` returns the row id; trades and hot-reload keyed on it. v12 is row 6 |
| #4 reload / shutdown orphans | fixed. reload is deferred per ticker until flat; shutdown flattens at the broker and records the trade; a 30s clock-based net flattens anything still open 2 min past `force_exit_by` |
| #5 silent synthetic feed | fixed. no keys ⇒ startup error; `--demo` is explicit, forbidden with `alpaca_paper`, and writes `source='demo'` |
| #6 websocket clean close | fixed. reconnects with backoff (reset after a healthy minute); per-ticker `feed_stale` flag after 3 min without a bar during hours |
| #7 daily reset | fixed. engine resets breaker/cooldown/avoid-first on each new Eastern date, anchored to 09:30 ET; VWAP resets in the state builder; candle windows persist |
| #8 warmup | fixed. 8 calendar days of 1-min SIP bars (lagged 16 min for the free plan, IEX fallback), RTH-filtered, paged. verified: 1,950 bars → 36 hourly candles per ticker |
| #9 capital cap | partially. `max_position_pct` clamp added and set to 0.36 in v12; `max_concurrent_positions: 1` still the exposure bound (the dead `max_capital_deployed_pct` path is unchanged) |
| #10 fills | recorded. `trades.broker_entry_price/broker_exit_price` now hold the real fills; engine P&L still uses bar closes (compare the two after week 1) |
| #11 size clamp | fixed (see #9). non-finite / ≤0 fractions skip the entry |
| extended-hours bars | new: only 09:30–16:00 ET bars reach the engine |
| alpaca order fills | new finding + fix: alpaca's create/close responses come back `accepted` before the fill. the broker now polls the order until `filled` (15s timeout ⇒ cancel + engine rollback). verified live: create → accepted → cancel |
| alpaca account | new finding + fix: apca 0.30 cannot parse the current `/v2/account` payload. a lenient endpoint replaces it; capital = min(`INITIAL_CAPITAL`, equity) |
| observability | `engine_state`, `entry_block_events`, `trades.entry_reason/source`, enum values for claude agents. all landed via `migrations/20260912000001` |
| v12 config | `migrations/20260912000002`: entries until 11:30 ET, flat by 11:55 ET, `avoid_first_minutes` 0, `max_position_pct` 0.36. everything else is v11 |
| scripts | `update_config.sh` now uses `DATABASE_URL`, sets `promoted_at`, takes a `created_by` |

**decisions made on your behalf (revisit if you disagree):**
- v12 reproduces what v11 actually backtested (morning-only). holding to 15:55 ET is untested; run the baseline on it before choosing it.
- `avoid_first_minutes` is 0 because the old code made it a no-op on the scored day. the engine now honours it, so any non-zero value is a strategy change to be backtested.
- whole-share sizing changes backtest numbers slightly at small capital; it is what the broker does, so paper and backtest now agree.

**still to do before monday:** commit this work; run the 20-day baseline (section 3) with the fixed binary; write the `/intraday-review` skill's first version (done: `.claude/skills/intraday-review/SKILL.md`); dry-run `/loop 15m /intraday-review` against the running trader on monday morning.

---

## 1. status opinion

### what's good

- **it builds and tests clean today** on rust 1.92: `cargo build`, `cargo test` (456 passed, 0 failed, 2 ignored), `cargo clippy -D warnings` all green.
- **the engine core is well built.** trait-based tool belt, `catch_unwind` around every indicator, immutable config versions, per-ticker overrides, a genuinely rich backtest CLI (~70 override flags, next-bar fills, cost model, per-window attribution).
- **alpaca paper keys in `.env` still work** (checked 2026-09-11: account ACTIVE, $100k equity, trading + data APIs both 200). no refresh needed.
- **the TUI (`--features tui`) is complete** and is the best live view that exists.

### what's not — the honest version

**the live path has never actually been exercised end-to-end with the current sizing model.** the evidence is bug #1 below: in `alpaca_paper` mode every single order would be rejected for zero shares. nobody has seen a v11 trade fill on the paper account. all "making money now" claims are backtest-only.

**the v11 numbers are in-sample and describe a different strategy than the one configured.**
- six rounds of selection (v6→v11) on the same 2022–2025 window; the only hold-out script was archived.
- "+$22,540 / 4yr" is 36% fixed-fractional sizing on the same ~1,100 trades that made +$581 at 5%. it's leverage, not alpha, and no drawdown was reported for it.
- validated on SPY/QQQ/AAPL/MSFT; promoted config trades AMZN/QQQ/AAPL/NVDA/MSFT. NVDA was explicitly excluded from validation as an outlier, AMZN was never backtested in the full config.
- because of bug #2, every backtest and the live engine force-flat at **11:55 ET**, not 15:55. the "validated" strategy is a morning-only strategy plus a tail of 1-bar churn trades in the afternoon.
- nothing has been tested on may–september 2026 data.

**observability for an intraday agent is close to zero.** the DB is written exactly once per *completed* trade. no heartbeat, no per-tick scores, no open-position state, no record of which entry window fired, no record of why an entry was blocked. an agent polling postgres cannot tell "quiet market" from "dead websocket" from "misconfigured gate vetoing everything".

**4.5 of 22 hardening-plan items are implemented.** the ones an agent needs (health endpoint, alerting, kill switch, audit log) are all absent. there is no promotion gate: any `INSERT … status='promoted'` is live within 60s.

### verdict

good engine, unproven strategy, untested live plumbing. paper trading is exactly the right next step, but **week 1 is a plumbing test, not a strategy test**. treat P&L as noise until entries, fills, exits, and DB writes are shown to match the backtest of the same day.

---

## 2. bugs that must be fixed before monday (verified in source)

| # | bug | where | effect live |
|---|-----|-------|-------------|
| 1 | share count divided by price twice: `shares = pos.size / last_price` but `pos.size` is already shares | `crates/data_feed/src/main.rs:354` | `alpaca_paper`: 0 shares → broker rejects → `undo_last_open` → **zero trades all week**. `simulated`: engine fine, broker qty garbage |
| 2 | `session_close` compares the **UTC** hour to the ET-intended `"15:55"` | `crates/actions/src/exit/session_close.rs:51-53` | force-flat at 11:55 ET; entries after that close on the next bar. entry cutoff in `tick_loop.rs:337` correctly uses `US/Eastern`, so the two halves disagree |
| 3 | `TradeWriter` and `ConfigWatcher` are seeded with the blob's `config_id` (11), not the `config_versions.id` primary key (5 on a fresh migrate) | `crates/data_feed/src/main.rs:162,165` | **every trade insert fails its FK** and is dropped; hot-reload looks for `id > 11` and never fires |
| 4 | config hot-reload and shutdown log "force-closing position" but close nothing | `main.rs:16-20`, `main.rs:509-533` | orphaned broker positions, lost trade rows, new engine re-enters on top |
| 5 | empty alpaca keys silently start a synthetic random-walk feed and write its trades to the real `trades` table | `main.rs:175-257` | fake data indistinguishable from real |
| 6 | clean websocket close is treated as success; the feed task exits and the bar channel closes | `crates/data_feed/src/alpaca_feed.rs:83-86` | process stays "up", trades nothing, no alarm. no staleness watchdog exists |
| 7 | no daily reset: session VWAP/volume accumulate across days, `daily_loss_breaker_active` latches forever, `avoid_first_minutes` counts from **process start** not 09:30 | `market_state.rs:67` never called; `tick_loop.rs:115,328,374` | multi-day runs drift; a restart at noon blocks entries until 13:00 |
| 8 | warmup fetches only 200 **1-minute** bars (~3.3h) → ~4 hourly candles; hourly indicators need 21 | `main.rs:174-190`, `alpaca_feed.rs:183-190` | the hourly hard gate runs on `vwap_distance` alone for ~3 days; weights silently renormalize |
| 9 | `max_capital_deployed_pct` is dead: the cap needs `total_deployed_capital` which live hardcodes `None`; each ticker engine gets the **full** account capital | `tick_loop.rs:275`, `market_state.rs:59-60`, `main.rs:130` | only `max_concurrent_positions: 1` bounds exposure. raising it to 2 = 72% notional |
| 10 | broker fill price is never fed back; trades are recorded at raw `last_price` with zero cost | `main.rs:360-366` | live P&L is optimistic vs backtest (which charges 2–3 bps + spread + next-bar open) |
| 11 | no position-size clamp; `fraction` is unvalidated | `fixed_fractional.rs:51-57`, `tick_loop.rs:293` | a fat-finger `3.6` drives capital negative and inverts P&L sign |

also worth knowing: `BROKER_MODE` must be exactly `alpaca_paper` (not `alpaca` as CLAUDE.md says); anything else silently falls back to simulated. bid/ask are fabricated as `close ± $0.01`. `update_config.sh` never sets `promoted_at` and ignores `DATABASE_URL`. `scripts/baseline_config.json` is the dead pre-fix v3 config.

### the session_close decision

fixing #2 changes the strategy. options:

- **(a) recommended for week 1:** fix the TZ, then set `force_exit_by: "11:55"` and `no_new_entries_after: "11:30"` in the config. this reproduces the backtested behaviour minus the afternoon churn, and matches the pre-bug-fix finding that morning-only was best.
- **(b)** fix the TZ and keep `15:55`. this is an untested strategy; run the 2025 + 2026-ytd baseline on it first.

---

## 3. week-1 plan (2026-09-14 → 09-18)

market hours are 09:30–16:00 ET = **06:30–13:00 PT**.

### before monday (sat/sun)

**P0 code fixes** — bugs 1, 2(a), 3, 4, 5, 6, 7, 8, 11. roughly one focused day. each with a unit test.
- #4: simplest safe version is **deferred reload** — when a new config is detected, rebuild only tickers that are flat; the rest swap on their next close. shutdown must call `broker.close_position` and write the trade row with `exit_reason = config_change`/`manual_override`.
- #8: fetch 5 trading days of 1-min bars (limit 10,000) and feed them through the aggregator so 5m and 1h windows are full at start.
- #11: clamp `size_fraction` to `[0, max_position_pct]` in `on_tick`, with `max_position_pct` in `SessionConfig` (default 0.36 to match v11).
- #3: `load_config` returns `(id, config)`; seed both writers with the row id.

**P0 observability** — the minimum for the agent (section 4):
1. `engine_state` table, upserted every 30s per ticker: last bar ts, last price, composite + 1m/5m/1h scores, open position (direction, entry, size, unrealized $/%, hold ms), `daily_pnl`, `loss_breaker_active`, `config_version_id`, `feed_stale`.
2. `entry_reason TEXT` column on `trades`, plumbed from `TickResult.entry_reason` (already exists, currently dropped by `data_feed`).
3. `entry_block_events` table (or structured log line): ticker, ts, which gate blocked (`avoid_first`, `after_cutoff`, `cooldown`, `loss_breaker`, `capacity`, `reject_gate:<name>`), scores at the time. also log near-misses: any tick where a window's `composite_min` passed but another condition failed.
4. `source` column on `trades` (`paper` | `backtest`) so `--write-db` never pollutes the paper record.

**baseline expectation** — run the backtest on the 20 trading days ending 09-11 with the fixed code and the chosen session times:
```bash
docker compose up -d postgres && sqlx migrate run   # or: cargo install sqlx-cli first
cargo build -p backtest --release
for d in <20 weekdays>; do
  ./target/release/backtest --date "$d" --lookback-days 5 --capital 10000 \
    --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv
done > data/baseline_pre_paper.csv
python3 scripts/compare_exits.py data/baseline_pre_paper.csv baseline
```
you want trades/day, win rate, exit-reason mix, and per-ticker P&L. this is what the week gets compared against. max 2 concurrent runs (alpaca rate limit).

**infra**
- `docker compose` here is podman-emulated; `psql` and `sqlx-cli` are not installed. install `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features postgres`) and `postgresql` client tools.
- `.env`: `BROKER_MODE=alpaca_paper`, `INITIAL_CAPITAL=10000` (matches the backtest capital so P&L is comparable). consider `RUST_LOG=debug` for week 1.
- confirm `SELECT id, status FROM config_versions` shows v11 as the only `promoted` row and note its id.

### monday 09-14 — plumbing day

- **06:00 PT** start `cargo run -p data_feed --release --features tui` in tmux (TUI is the only live view). check logs for `historical bars loaded` with ~1,900 bars per ticker and no `starting synthetic demo feed`.
- watch the first entry: engine "position opened" → "broker order filled" with a sane quantity → alpaca dashboard shows the position → exit → `trades` row lands with `entry_reason` populated.
- **13:05 PT** confirm flat on alpaca and in `engine_state`. then run `backtest --date 2026-09-14 --lookback-days 5` and diff against `SELECT * FROM trades WHERE source='paper' AND entry_fill_at::date = '2026-09-14'`. entries should match on ticker/time/direction within a bar; prices will differ (IEX feed vs SIP, same-bar vs next-bar fill).
- if monday shows zero trades, that is *expected* on some days with this config (1 concurrent position, 30s cooldown). the `entry_block_events` table tells you whether it was a gate or a genuine no-signal.

### tue–fri 09-15 → 09-18 — run + watch

- same daily routine: pre-open check (06:15 PT), agent loop during hours (section 4), EOD backtest-vs-paper diff, one-paragraph note per day in `agent_memos`.
- **do not tune the strategy on week-1 results.** with 1–5 trades/day the sample is noise. week 1 config changes are allowed only for plumbing/safety reasons (a window vetoing everything, a ticker halted, an exit misfiring).

### success criteria for the week (none are P&L)

- process ran every session without manual restart; feed never went silently stale.
- every position opened at the broker with the intended share count and closed before 12:00 ET (option a) with a `trades` row.
- zero orphaned positions on alpaca at any close.
- paper entries match same-day backtest entries on ≥80% of trades.
- `engine_state` heartbeat never gapped >2 min during hours.
- at least one config hot-reload applied cleanly while flat.

if all six hold, week 2 becomes the strategy test: same config, no changes, gather 20+ trades, then compare to the 20-day backtest baseline.

---

## 4. intraday agent — design

### where it runs (this is the binding constraint)

- **cloud routines** (`/schedule`) have a **1-hour minimum interval** and run in anthropic's cloud with **no access to the local postgres or the running bot**. they cannot do the intraday job. they *can* do the EOD PM cycle if the repo + a reachable DB are set up later.
- **local session cron** (`CronCreate` / `/loop`) fires every N minutes inside a claude code session on this machine. jobs are session-only and expire after 7 days — fine for a trading week; the session must stay open in a terminal.
- **headless** (`claude -p "<prompt>"` from a systemd user timer) is the durable version for week 2+: stateless per run, all state in postgres.

**recommendation for week 1:** a second tmux pane running claude code with `/loop 15m /intraday-review`, where `/intraday-review` is a project skill (`.claude/skills/intraday-review/SKILL.md`) containing the prompt below. you're at the machine anyway, and the interactive session keeps memory of what it already changed. move to the systemd timer once the prompt has stabilised.

### cadence

| time (PT) | job | model |
|---|---|---|
| 06:15 | pre-open: process alive? bars flowing? `engine_state` fresh? hourly window full? config id as expected? push-notify if not | sonnet |
| 06:35 → 12:55 every 15 min | watchdog + review loop (below) | sonnet; escalate to opus only when proposing a change |
| 13:05 | EOD: confirm flat on alpaca, run same-day backtest diff, write memo, propose (not promote) any strategy change for human review | opus |

### the loop prompt — what each tick does

1. **health (always, fast, no LLM judgement needed):**
   - `engine_state.updated_at` older than 3 min for any ticker during hours → WARNING; older than 6 min → CRITICAL.
   - any open position with `hold_ms` > `max_hold_ms` + 10 min, or any position after `force_exit_by` → CRITICAL.
   - `daily_pnl / capital` < −3% → WARNING, < −5% → CRITICAL.
   - trade count today > 3× the 20-day baseline mean → WARNING (churn).
   - CRITICAL → `PushNotification` to the user, write `agent_memos(memo_type='watchdog_critical')`. do **not** auto-liquidate in week 1; the human is present.
2. **read the day so far:** trades today with `entry_reason` and exit reason; `entry_block_events` grouped by gate; current scores per ticker from `engine_state`; the promoted config and its id; memos already written today (so it doesn't repeat itself).
3. **decide.** default is **hold steady** and write nothing. a change is proposed only if one of these evidence patterns holds:
   - a gate is vetoing everything: e.g. `reject_1m_noise` fired on >90% of ticks where a window otherwise passed, across ≥3 tickers, for ≥90 minutes.
   - an exit is systematically misfiring: e.g. ≥3 `hard_stop` exits today where the position was up >1% at some point (breakeven logic not doing its job).
   - a ticker is broken: halted, feed stale for that symbol only, or 2+ broker rejections → disable the ticker (remove from `tickers`) rather than tune around it.
   - a safety condition: drawdown warning → tighten `stop_loss_pct` or lower `max_concurrent_positions`; never loosen anything on a losing day.
4. **validate before promoting.** for any non-safety change: run `backtest --date` for the last 5 trading days with the CLI override equivalent (2 concurrent), require P&L ≥ current config's on those days and no increase in `hard_stop` share. this takes ~3–5 min inside a 15-min window. safety changes skip validation.
5. **apply only when flat.** check `engine_state` for open positions across all tickers; if any, write the intent to `agent_memos` and re-check next tick. when flat: insert a new `config_versions` row (`status='promoted'`, `promoted_at=now()`, `parent_version_id=<current>`, `mutation_reason` with the evidence), supersede the old row, write a memo. the engine picks it up within 60s.
6. **log the tick** as one short memo line only when something happened (state changes, warnings, proposals). silent ticks write nothing.

### guardrails (hard rules in the prompt, mirrored from the archived PM prompt)

- **max 1 parameter change per tick, max 3 per day, none in the first 30 minutes after open or the last 30 before `no_new_entries_after`.**
- **no stacking:** don't touch a parameter area while the previous change has <10 trades under it.
- **allowed knobs:** entry-window `conditions` thresholds; `enabled` on a window or the reject gate; `stop_loss_pct`; ATR `multiplier`; `max_hold_ms` / `profit_extension_ms`; `scoring.exit_threshold`; `entry_cooldown_ms`; `no_new_entries_after`; removing a ticker; `max_concurrent_positions` **downward only**; `ticker_overrides` for `stop_loss_pct` / `atr_multiplier` / `max_hold_ms`.
- **forbidden intraday:** sizing `fraction` (any direction); adding tickers (reload doesn't create their state builder); adding/removing indicators or changing an indicator's timescale/period (re-warms the window); `hard_gate_timescales`; `force_exit_by`; `max_concurrent_positions` upward; anything in `scoring.timescale_weights`.
- **invariants checked before insert:** blob deserialises (run `cargo run -p engine -- --validate` or equivalent), exactly the same indicator set as parent, `session_close` enabled, every window has `composite_min` ≥ 0.20, `stop_loss_pct` ∈ [0.005, 0.05].
- **rollback** = insert a copy of the parent blob as a *new* row (the watcher only sees `id > current`; re-promoting an old id does nothing to a running process).
- **the agent never places or closes orders directly** in week 1. it changes config, the engine acts.

### tables the agent uses

- reads: `engine_state`, `trades` (+`entry_reason`, `source`), `entry_block_events`, `config_versions`, `agent_memos`, the `daily_performance` / `performance_by_exit_reason` views (note: `daily_performance.trading_day` is UTC-truncated; fine for US hours).
- writes: `config_versions`, `agent_memos` (reuse the archived table; `memo_type` enum may need `watchdog_warning` / `watchdog_critical` / `intraday_review` values — one migration), `config_changelog` (reuse).
- `created_by` is the `agent_type` enum with no `human`/`claude` value — add `claude_intraday` and `claude_eod` in the same migration so provenance is honest.

### what not to expect

with this config's trade frequency, intraday statistical tuning is not possible; a 15-minute agent that "adjusts the engine" on 2 trades of evidence is a noise amplifier. the value of the loop in week 1 is **(a)** a watchdog with judgement, **(b)** catching plumbing failures within 15 minutes instead of at EOD, **(c)** building the memo trail that the EOD/overnight PM cycle needs. real strategy iteration belongs in the EOD job, validated on multi-day backtests, and promoted for the *next* session.

---

## 5. things deliberately left out of week 1

- implementing `AlpacaBroker` fills feeding back into the engine (bug 10) — record broker fill price alongside engine price in `trades` first, fix once you can measure the gap.
- health HTTP endpoint / kill switch — `engine_state` + the TUI + `docker stop` cover week 1.
- re-validating the strategy on 2026 data with option (b) session times — do it over the weekend only if the P0 list is done.
- cockpit fixes (`/agents` empty-state text, `/backtest` reads CSVs not DB, phantom `high_water_mark` columns) — cosmetic.
- CLAUDE.md is stale in ~a dozen places (8 actions → 12, `BROKER_MODE=alpaca` → `alpaca_paper`, "walk-forward mode", JSON config loading, test counts). worth a rewrite after the fixes land so future agents don't inherit the wrong model.

---

## 6. unattended operation and scheduled check-ins (added 2026-09-11 evening)

### set-and-forget on this desktop

user-level systemd units, installed by `deploy/systemd/install.sh` (idempotent; re-run after editing a unit).
`loginctl enable-linger` is on, so they run without a login session and survive reboots.

| unit | when (America/Los_Angeles, weekdays) | what |
|---|---|---|
| `trading-postgres.service` | on boot / on demand | `docker compose up -d postgres` (podman) |
| `paper-trader.timer` → `paper-trader.service` | 06:10 | waits for postgres, runs `sqlx migrate run`, starts `target/release/paper_trader` with `.env`. SIGTERM flattens and records positions (90 s grace) |
| `paper-trader-stop.timer` | 13:10 | stops the trader after the close so each day starts with a fresh warmup |
| `preopen-check.timer` | 06:15 | `claude -p "/preopen-check"` (sonnet, ≤ $0.50) |
| `intraday-review.timer` | 06:35, 06:50, then every 15 min 07:05–12:50, 13:05 | `claude -p "/intraday-review"` (sonnet, ≤ $1) — cheap short-circuit once flat after `force_exit_by` |
| `eod-review.timer` | 13:30 | `claude -p "/eod-review"` (opus, ≤ $5): same-day backtest diff, one validated change for tomorrow or a falsifiable hold |

controls: `systemctl --user status paper-trader.service`, `journalctl --user -u paper-trader.service -f`,
`systemctl --user start eod-review.service` (run a job now), `systemctl --user disable --now intraday-review.timer` (pause a job).
check-in transcripts append to `logs/checkins/<job>.jsonl` (one JSON per run: `result`, `total_cost_usd`, `session_id`).

**before monday you must rebuild the release binary after any code change** (`cargo build --release -p data_feed -p backtest`); the service runs `target/release/paper_trader` as-is.

### why systemd timers + `claude -p`, not the alternatives

| option | verdict |
|---|---|
| cloud routines (`/schedule`) | 1-hour minimum, run in anthropic's cloud with a fresh clone: cannot reach the local postgres or the running process. fine later for an overnight research job on the repo, not for check-ins |
| `/loop` / in-session cron | needs an open terminal session; jobs die with it and expire after 7 days |
| systemd timer → `claude -p "/skill"` | 1-minute granularity, local DB access, survives logout/reboot, no TTY, one JSON per run. **chosen** |
| agent SDK | only worth it if the check-in needs approval callbacks or to be embedded in a larger program. not needed |

headless auth: the interactive login is not usable by non-interactive runs on this machine, so the units load
`ANTHROPIC_API_KEY` from `.env` (console billing). to bill the subscription instead, run `claude setup-token`
once and put the resulting `CLAUDE_CODE_OAUTH_TOKEN` in `.env` (expires after a year).
tools are pre-approved in `.claude/settings.json` (psql, the scripts, the backtest binary, read-only alpaca
GETs); everything else is denied, not prompted (`--permission-mode dontAsk --permission-prompts none`).
`git push`, `rm -rf`, and `systemctl start/stop` are explicitly denied.

notifications: `scripts/notify.sh <level> "<msg>"` logs to `logs/notifications.log` and, if `NOTIFY_TOPIC` is
set in `.env`, pushes to `https://ntfy.sh/<topic>` (install the ntfy app on your phone, subscribe to a
hard-to-guess topic name). the skills call it on WARNING/CRITICAL and for the EOD one-liner.

### the prompt philosophy: bias for action without over-correcting

three jobs with different authority, so "act" and "don't over-correct" are never in tension inside one prompt:

- **pre-open** (`preopen-check`): verifies the day's data will be trustworthy. no config authority. acts by notifying.
- **intraday** (`intraday-review`): watchdog with judgement. acts fast on *safety and plumbing* (stale feed, orphaned
  position, drawdown, a gate vetoing everything, a broken ticker) and is explicitly told that today's P&L is noise.
  strategy tuning is out of scope. max 1 change per tick, 3 per day, allowed-knob list, forbidden-knob list,
  5-day backtest gate on non-safety changes, no changes in the first/last 30 minutes.
- **end of day** (`eod-review`): the only job with tuning authority, and it acts on the *next* session, never a live
  position. it must **decide something every day** — either one validated change or an explicit hold with a
  falsifiable trigger — which keeps the bias for action without letting "hold" become inertia. guardrails against
  over-correction: plumbing verdict before any strategy verdict (paper vs same-day backtest must match); evidence
  thresholds stated in trades (≥ 15–20), not days or dollars; one parameter per change; 20-day backtest validation
  with explicit pass criteria; **never correct a correction** (a change < 3 sessions old isn't re-judged until it has
  ≥ 15 trades); fridays list candidates for the human instead of applying them.

the memo table (`agent_memos`) is the shared memory between runs, so each job reads what the others decided and
why. every config change carries `parent_version_id`, the evidence, the validation numbers, and the revert condition.

### found by the first end-of-day dry run (2026-09-11 evening) and fixed

the eod skill was run once against the (idle) system. it correctly refused to tune and reported three plumbing defects, all fixed the same evening:

1. **the backtest replayed pre-market and overnight bars** (557–710 bars/day vs 390 in regular hours) while the live engine filters to 09:30–16:00 ET. `crates/backtest/src/alpaca_loader.rs` now filters too. **every earlier backtest number in this repo — v6 through v11 — was produced with extended-hours bars in the indicator windows**; the v12 sweep was restarted on corrected data and is the only baseline to trust from here on.
2. the postgres image lacks the `US/Eastern` tz alias; all check-in SQL now uses `America/New_York` (rust code is unaffected: `chrono_tz::US::Eastern` is fine).
3. no native `psql` on this host: `scripts/psql.sh` wraps psql-in-container and `update_config.sh` uses it.

also: the backtest lookback is now 8 calendar days (`scripts/backtest_range.sh`, the eod skill) to match the live warmup — 5 calendar days gave only 18 hourly candles, fewer than the hourly EMA/Bollinger need.

### the look-ahead bug (found 2026-09-11 evening, after the first corrected sweep leg)

the first corrected-data sweep leg returned +52 % on $10k in 8.5 months with a 1.9 % max drawdown and a daily
sharpe above 7, with 63 % of entries in the 09:30 bar and a 99 % win rate on holds that reached the profit
extension. a naive buy-09:35/sell-11:25 baseline on the same tickers was flat (+$116), and spot-checked fills were
exactly next-bar open plus costs — so the edge was in the *signals*. cause: `crates/backtest/src/replay.rs`
pre-aggregated the 5-minute and hourly series, stamped each candle with its bucket start, and at every 1-minute
tick included every candle whose bucket had *started*. at 09:31 the engine saw the completed 09:30–09:34
five-minute candle and the completed 09:30–10:29 hourly candle, closes included; the "strong core" window's
hourly condition was effectively "the hour will close up".

**every backtest number this repo ever produced (v6 → v11, "+$22,540", the tuning log, and my own first two sweep
legs) had this look-ahead. none of them mean anything.** the live engine was never affected: its aggregator only
sees bars as they arrive. the replay now feeds bars through that same aggregator (moved to
`crates/engine/src/candle_aggregator.rs`), with the same 200-candle rolling window, so backtest and live build
identical windows. the v12 sweep was restarted a third time on this code; only its results are trustworthy.

### honest v12 baseline — 2026-01-02 → 09-10, corrected replay (run 2026-09-11 17:15 PT)

| | |
|---|---|
| P&L on $10k, 36 % sizing | **+$1,407** (+14 %) · 476 trades · 2.6/day |
| win rate / PF / avg trade | 44.1 % / 1.27 / +$2.96 |
| max drawdown / longest DD | −$416 (4.2 %) / 55 days |
| daily sharpe / positive days | 1.93 / 48 % |
| tuning half (Jan–Jun) vs holdout (Jul 1–Sep 10) | +$786 (330 trades) vs +$664 (144 trades) — the edge survives out of sample |
| by ticker | MSFT +631 · AAPL +589 · AMZN +524 · NVDA +109 · **QQQ −446 (PF 0.54)** |
| by window | strong core +1,282 (309) · 5m thrust +334 (67) · **candle reversal −209 (100, PF 0.85)** |
| by exit | max-hold +3,584 (54 % win) · session-close +940 · hard-stop −272 (3) · **score-exit −2,845 (132 trades, 13 % win)** |
| by entry time | 09:30 bar carries 303 of 476 trades at +$4.33 avg; 11:15–11:30 entries lose (−$90) because the 11:55 flat cuts them at ~40 min |

reading: a modest, real, long-only morning momentum edge on the mega-caps. losses are concentrated in one exit
(score-exit at −0.15) and one instrument (QQQ). the naive open→11:25 baseline on the same days was flat, so this is
selection, not drift. everything below is measured against this table.

### batch 1 — single-factor variants on 2026 (corrected replay, all 180 days complete)

| variant | P&L | PF | trades | maxDD | tune (Jan–Jun) | holdout (Jul–Sep 10) | verdict |
|---|---|---|---|---|---|---|---|
| v12 (baseline) | +1,450 | 1.28 | 474 | −416 | +786 | +664 | — |
| full-day session (entries to 15:30, flat 15:55) | +108 | 1.01 | 860 | −723 | −33 | +141 | **rejected** — the afternoon has no edge; morning-only stands |
| score exit −0.30 (was −0.15) | +1,623 | 1.32 | 467 | −420 | +851 | +772 | **adopt** — better on both halves |
| score exit −0.50 | +1,578 | 1.31 | 467 | −435 | +836 | +741 | better than base, slightly worse than −0.30 |
| drop QQQ | +1,896 | 1.45 | 363 | −269 | +1,082 | +814 | **adopt** — better on both halves, drawdown −35 % |
| drop candle-reversal window | +1,445 | 1.29 | 445 | −422 | +771 | +674 | neutral — leave it |

v13 candidate = v12 + `scoring.exit_threshold` −0.30 + tickers AMZN/AAPL/NVDA/MSFT. batch 2 tests the combination
on 2026 and 2025, and stacks seven single factors on top of it (open skip 15 min, hard stop 1.5 %, ATR trail 3×,
entries by 11:00, profit extension 60 min, strong-core composite floor 0.45, SPY as the fifth ticker).

### batch 2, first results — the 2025 problem

| | 2026 (Jan–Sep 10) | 2025 (full year) |
|---|---|---|
| v12 | +1,450 · PF 1.28 | **−3,369 · PF 0.60 · 34 % win** |
| v13 candidate (no QQQ, score exit −0.30) | +2,065 · PF 1.51 · tune +1,112 / holdout +953 | **−2,758 · PF 0.62 · 11 of 12 months negative** |
| naive buy 09:35 / sell 11:25, same 4 tickers | +387 | **−3,108** (every ticker, every quarter negative) |

the strategy is a long-only morning-momentum book whose P&L is dominated by the sign of the morning drift in the
mega-caps: 2025 mornings sold off, 2026 mornings rallied. the selection adds value over naive in both years
(+$350 in 2025, +$1,700 in 2026), but it cannot overcome a −0.09 %/day headwind. 392 of 521 trades in 2025
entered on the 09:30 bar — the first bar's scores are yesterday's state plus one minute, so the dominant trade
is "buy the open after an up day".

**consequences for monday:** the honest expectation for a long-only v12/v13 is "profitable if mornings trend up,
loses steadily if they don't", with ~2 trades/day and a 3–4 % drawdown either way at 36 % sizing. that is a
plumbing test, not an edge. the experiment that could change the picture is symmetry — mirrored short windows so
a negative-drift regime is tradeable — which required: `composite_max` / `timescale_lag` conditions, a
direction-aware score exit (it was long-only and closed every short one bar later), and dropping the hourly hard
gate under `--mirror-short` (it floors the composite at 0 whenever the hourly score is negative, so shorts could
never fire). those are in; `v13a_mirror` legs on 2026 and 2025 decide it.

### batch 2 results and the v13 decision (2026-09-11, ~19:30 PT)

single factors stacked on the candidate (2026, both halves must improve to adopt):

| variant | P&L | tune | holdout | verdict |
|---|---|---|---|---|
| candidate (no QQQ, exit −0.30) | +2,065 | +1,112 | +953 | — |
| + mirrored short windows (hard gate off) | **+2,743** | +1,790 | +953 | **adopt** (also +$826 better on 2025) |
| + hard stop 1.5 % | +1,990 | +1,135 | +855 | wash; revisit on 2025 |
| + entries by 11:00 | +2,082 | +1,142 | +939 | neutral simplification; not adopted |
| + ATR trail 3× | +1,686 | +968 | +718 | rejected |
| + profit extension 60 min | +1,979 | +1,090 | +889 | rejected |
| + strong-core composite floor 0.45 | +1,960 | +1,044 | +917 | rejected |
| + skip first 15 min | +997 | +460 | +538 | rejected (the 09:30 entries are the edge in an up-drift year) |
| + SPY as 5th ticker | +1,844 | +995 | +849 | rejected |

direction breakdown of the mirrored candidate: 2025 longs −$2,635 / shorts +$704; 2026 longs +$2,095 / shorts +$648.
a post-hoc trailing-morning-drift regime filter (10/20/40-day, per-ticker or market-wide) reduces the 2025 loss
at best to −$792 while costing 2026 — not adopted; the drift signal is too slow to be a clean switch.

**v13 = v12 + no QQQ + score exit −0.30 + hourly hard gate removed + short twins of both windows and the reject gate
+ sizing 0.15 (`migrations/20260912000003`).** each step beats the previous in both years. promoted as row 7;
the promoted blob reproduces the CLI-mirrored sweep exactly at equal sizing (3 days checked, trade-for-trade);
the live binary builds it (4 engines, short windows, warmup 35 hourly candles). a final no-override sweep of the
promoted blob on 2026 and 2025 is the last check.

**what to expect on paper at 15 % sizing:** ~3 trades/day, roughly +$100/week if 2026-like, −$40/week if
2025-like, max drawdown ~2 % (2026) to ~10 % (2025). a losing week does not mean the plumbing is wrong.

**open questions for you (not blockers):** the long book is a bet on morning drift and there is no clean
regime switch in the data I have; the short book is real but thin. if you want regime robustness, the next
research step is a different long signal, not more threshold tuning. sizing at 15 % is my call; 36 % is
defensible only on 2026.

### final: the promoted v13 blob, no overrides (run 2026-09-11 21:53 PT)

| | 2026 (Jan 2–Sep 10) | 2025 (full year) |
|---|---|---|
| v12 @ 36 % | +1,450 · PF 1.28 · maxDD −416 (4 %) | −3,369 · PF 0.60 · maxDD −3,555 (36 %) |
| **v13 @ 15 % (promoted, row 7)** | **+1,087 · PF 1.43 · maxDD −198 (2 %) · 560 trades · tune +732 / holdout +356** | **−775 · PF 0.82 · maxDD −974 (10 %) · 748 trades** |

trade selection is identical to the 36 %-sized mirrored sweep (same 560 / 748 trades, same win rates); only the
size differs. this is the config the trader will start on monday 2026-09-14 at 06:10 PT.

### v14 (2026-09-11, late): sizing 0.30 by operator decision

row 8, `migrations/20260912000004`. identical trade selection to v13; at 30 % of $10k the corrected-replay
expectation is 2026 ≈ +$2,280 with max drawdown ≈ 4 %, 2025 ≈ −$1,610 with max drawdown ≈ 20 %. this is the
config the trader starts on monday 2026-09-14.

### the pre-monday sweep

`scripts/run_v12_sweep.sh` (log: `data/sweep_progress.log`, ~6 h sequential to respect alpaca's free-plan rate limit):

| tag | period | question |
|---|---|---|
| `v12` | 2026-01-02 → 09-10 | **out-of-sample** (v11 was tuned on 2022–2025). the number that matters |
| `v12` | 2025 | in-sample comparison against the v11 claims |
| `v12_fullday` | 2026, 2025 | is holding to 15:55 ET better than the morning-only session v11 actually tested? |
| `v12_5pct` | 2026 | the same trades at 5 % sizing: how much of the P&L is leverage |
| `v12_avoid30` | 2026 | does skipping the first 30 minutes (now actually enforced) help? |
| `v12_core4` | 2026 | the validated universe (SPY/QQQ/AAPL/MSFT) vs the promoted one (AMZN/NVDA in, SPY out) |

read results with `scripts/report_backtest.py --compare v12 v12_fullday v12_5pct v12_avoid30 v12_core4` and
`scripts/report_backtest.py v12` for the per-month / ticker / window / exit breakdown.

---

## 7. long-signal research, round one (2026-09-11 night → 09-12 early)

harness: `research/signals/*.json` + `backtest --patch-json`, four session-anchored indicators
(`opening_range`, `gap`, `hourly_trend`, `session_clock`). each candidate measured in isolation (promoted windows
and gates disabled) at 36 % sizing against the v13 long book alone as the control.

| candidate | 2026 | 2025 | verdict |
|---|---|---|---|
| long-book control (gates off) | +2,125 · PF 1.53 | −2,632 · PF 0.63 | the bar |
| ORB-30 + rel. volume | +816 · PF 1.32 | −2,204 · PF 0.56 | loses in a down-drift year too — breakouts get faded |
| ORB-15 + rel. volume | +510 · PF 1.12 | −2,709 · PF 0.61 | same as control in 2025 |
| strong core behind hourly-trend gate | +1,694 | −2,194 | the gate saves ~$440 in 2025 and costs ~$430 in 2026: a wash |
| squeeze breakout after 10:00 | +305 · PF 1.09 | — | eliminated |
| gap-and-go | +193 · PF 1.08 | — | eliminated |
| VWAP reclaim after 10:00 | +89 · PF 1.07 | — | eliminated |
| morning reversal 10:00–10:30 | +12 · PF 1.01 | — | eliminated |
| pullback in hourly uptrend | −2,515 · 7.9 trades/day | — | eliminated (churn) |

market-confirmation premise (SPY's first 15 minutes / overnight gap vs the rest of the morning, 2022–2026, from
raw bars): no direction-following effect in any year; a fade effect after a strong SPY open exists in some cells
but flips sign across years and thresholds with small samples — not a signal worth plumbing.

**conclusion:** the long book's dependence on morning drift is a market property, not an entry-trigger
property. no intraday long pattern tested is regime-robust; the symmetric short book (v13/v14) remains the only
component positive in both years. side finding: both reject gates are slightly negative on 2026 (+$2,125 without
vs +$2,065 with) — a candidate simplification for a later EOD review, not a monday change.

## 8. five-year regime picture (promoted logic at 36 % sizing, corrected replay; run 2026-09-12 00:30–01:10 PT)

| year | long book | short book | both |
|---|---|---|---|
| 2022 | −2,049 · PF 0.79 (529) | **+3,566 · PF 1.56 · t 3.5** (416) | +1,516 |
| 2023 | +512 · PF 1.07 (529) | +469 · PF 1.17 (209) | +982 |
| 2024 | −690 · PF 0.90 (549) | +412 · PF 1.14 (212) | −278 |
| 2025 | −2,659 · PF 0.62 (504) | +704 · PF 1.20 (244) | −1,955 |
| 2026 | +2,095 · PF 1.53 (353) | +648 · PF 1.25 (207) | +2,743 |
| **5-yr** | **−2,790 (2 of 5 years positive)** | **+5,798 (5 of 5 years positive)** | +3,008 |

the long book is a bet on positive morning drift and loses over five years. the short book is positive every
year with max drawdowns of $412–$867 (4–9 % at 36 % sizing). this split is post-hoc (one position at a time, so
longs sometimes occupied the slot); `v15_shortonly` legs on all five years are the decision-grade test.

## 9. v15 — short-only (promoted 2026-09-12 ~02:20 PT, row 9)

real short-only runs (long windows disabled, 36 % sizing) reproduce the post-hoc split:

| year | short-only P&L | PF | trades | maxDD | 5m thrust short / strong core short |
|---|---|---|---|---|---|
| 2022 | +3,564 | 1.56 | 416 | −602 | +2,194 / +1,370 |
| 2023 | +479 | 1.17 | 209 | −412 | +512 / −33 |
| 2024 | +395 | 1.13 | 212 | −870 | +369 / +26 |
| 2025 | +699 | 1.20 | 244 | −660 | +533 / +166 |
| 2026 | +665 | 1.26 | 207 | −533 | −97 / +761 |

positive every year; both windows contribute and each carries a year the other doesn't. v15 =
`migrations/20260912000005`: v14 with `window_5m_thrust`, `window_strong_core`, `window_candle_reversal`
disabled (not removed). sizing stays 0.30. the promoted blob reproduces the short-only sweep exactly on
three checked days (2022-06-13, 2025-04-04, 2026-06-04); the live binary builds it.

**what to expect on paper at 30 %:** ~1 trade a day, all shorts, roughly +$300–600 in a typical year
(2022-like years much more), max drawdown ~5–7 %. quiet weeks with zero trades will happen.

sizing note: the earlier "kelly ≈ 0.17" shortcut was wrong (a losing trade loses ~0.6 % of the position,
not the position); kelly is not the binding constraint here, drawdown tolerance is. 30 % is a preference.

## 10. trade-path analysis round (2026-09-12, afternoon)

goal: instead of sweeping knobs blind, look at *what the trades actually did* — for losers and
breakevens, did we exit too late or too early; for the best shorts of each day, which condition
kept the engine out — and then test the changes that evidence suggests, one at a time.

### 10.1 tooling added (commits 7069a05, 9504db8)

- **local bar cache** `backtest --fetch-bars data/bars --start … --end … --tickers …` writes
  `data/bars/<TICKER>.csv` (RTH 1-minute, epoch seconds). `--bars-dir data/bars` replays from it:
  no alpaca calls, ~0.5 s per day, so a five-year sweep runs in ~3 minutes with one process per year
  (`scripts/run_cached_sweep.sh <tag> [backtest args]`, `DUMP_TICKS=1` for diagnostics).
- **per-tick dump** `--dump-ticks FILE`: one row per bar with ohlcv, vwap, composite/1m/5m/1h scores,
  position state, event, `entry_blocked_by`, `near_miss`. ~1M rows / 50 MB per year.
- **pagination bug found and fixed**: alpaca pages at 10,000 bars and the limit counts the
  pre/post-market bars we filter client-side; `fetch_bars_range` ignored `next_page_token`, so any
  request spanning more than ~10k raw bars lost its tail silently. the first cache build lost 91
  whole NVDA days. the loader now follows the token. the 8-day lookback fetch used by every sweep
  so far stayed under the limit on the days checked — the cached `v15c` sweep reproduces the
  alpaca-fed `v15_shortonly` sweep trade-for-trade for 2023–2026 (2022 differs only in the first
  week of january, where the cache has no prior-year warm-up).

### 10.2 cached baseline `v15c` (v15 short-only, 36 % sizing)

| year | P&L | trades |
|---|---|---|
| 2022 | +3,654 | 414 |
| 2023 | +479 | 209 |
| 2024 | +395 | 212 |
| 2025 | +699 | 244 |
| 2026 (to 09-10) | +665 | 207 |
| total | +5,892 | 1,286 (PF 1.32, maxDD $870, sharpe 1.22) |

two analysis agents were run on this data (exit timing; missed setups). findings and the variant
tests that followed are in §10.3 onward.

### 10.3 agent findings (full reports: `docs/analysis/2026-09-12_exit_timing.md`, `_missed_setups.md`; scripts in `scripts/analysis/`)

**exit timing** (reconstructed every trade's minute path; simulator reproduced 1,285/1,286 exits):

- losers show their hand early: 74 % of eventual losers are underwater at 15 min, 82 % at 30, 90 %
  at 45, and the loss keeps growing. only 7 % of losers ever had ≥ 1 % open profit — "gave back a
  win" is the minority case. winners are the mirror image (89 % positive at 30 min).
- winners are exited about right: MaxHold winners capture 77 % of path MFE; holding to 11:55 adds
  nothing. every `max_hold_ms` extension loses money; `profit_extension_ms` is non-monotone.
- the knob is the existing `loss_reduction_ms`: MaxHold is checked every bar, so
  `max_hold − loss_reduction` is literally "exit the first bar after N minutes on which the position
  is losing". losing limit 75 → 30 min (`loss_reduction_ms` 3,600,000) and winning limit 120 → 90
  (`profit_extension_ms` 0): simulated +5,890 → +7,507, PF up every year, maxDD down 4/5, 2023 flat.
  L 25–40 all beat 75 in every year/ticker/window/fold; exact optimum within noise.
- `stop_loss_pct` 1.5 % is the runner-up on its own (+916, DD down every year) but redundant once
  the time stop is in.
- `exit_threshold` and the score exit: leave alone (irrelevant on top of the time stop).
- **code finding**: `breakeven_stop` is a no-op — `tick_loop.rs` receives `ModifyStop` and
  "just notes it happened". if implemented, 0.5 % breakeven on top of L30/W90 simulates +7,809 with
  the lowest drawdowns of any scenario.
- real replay (old cost model, re-entries included): losing limit 30 alone → +7,089 over five
  years (vs +5,892), 95 extra trades, PF 1.32 → 1.50, maxDD 870 → 721, every year positive.

**missed setups** (labelled every 09:30–11:29 bar with the value of a hypothetical short):

- the great shorts the engine misses are **not near-misses**: 86–94 % of the top-5 % missed bars need
  two or more window conditions changed, and their median 5-minute score is *bullish* (+0.07 to
  +0.22). no threshold on the timescale scores can see them.
- the single most-binding condition is `strong core short: s1h ≤ −0.30`, but the bars it excludes
  have the market's base-rate expectancy (mean −0.066 %, hit 39 %, 12,279 bars). every loosening adds
  ~$3/trade of volume, positive in only 2–3 of 5 years, and is negative in 4–5 years once costs are
  charged against the short. no tightening is robust either (`core s1h −0.40` is the only one with a
  quality signal: PF up 4/5 years, but P&L down 3/5 under the old cost model).
- reject gates are inert at current thresholds (blocked one window-qualifying bar in five years).
- **cost-model bug (the big one)**: `replay.rs` applied "buy pays up, sell receives less" to every
  trade regardless of direction. a short sells at entry and buys at exit, so it was *credited*
  ~3 bps + $0.005 on both legs — ~$4.5/trade, ~$5.7k over the 1,286 five-year trades. under honest
  costs the agent's simulator puts v15 at **2022 +1,860 / 2023 −442 / 2024 −474 / 2025 −478 /
  2026 −291**. fixed in commit b048cdc; §10.4 re-runs everything under the corrected model.

**what this means for the "short book positive five of five years" claim in §8**: it was true under
a cost model that paid the short side. the honest picture is one good year (2022) and four roughly
break-even years before the exit change. the exit change is where the evidence now points.

### 10.4 everything again under the corrected (direction-aware) cost model

same five-year cached replay, 36 % sizing, 3 bps + $0.005 charged *against* the trade on both legs.
tags `h_*` in `data/`.

| variant | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | trades | PF | maxDD |
|---|---|---|---|---|---|---|---|---|---|
| `h_base` — v15 as promoted | +1,852 | −463 | −496 | −470 | −312 | **+112** | 1,286 | 1.01 | 2,333 |
| `h_l30w90` — losing limit 30 min, winning limit 90 | +1,675 | −677 | −187 | −63 | +16 | **+764** | 1,422 | 1.04 | 1,404 |
| `h_stop15` — hard stop 1.5 % | +2,021 | −469 | −456 | −483 | −150 | +463 | 1,305 | | |
| `h_l30w90s15` — L30/W90 + stop 1.5 % | +1,403 | −634 | −215 | +18 | +73 | +645 | 1,430 | | |
| `h_l40w90` — losing limit 40, winning 90 | +1,577 | −449 | −56 | −254 | +57 | +875 | 1,369 | | |
| `h_core40` — strong-core window s1h ≤ −0.40 | +1,791 | +48 | −374 | −71 | −310 | +1,084 | 1,078 | | |

read: under honest costs nothing turns the short book into a real earner — every variant is within
about ±$1,000 over five years on a $10,000 account. what the variants do is cut drawdown (base
$2,333 → ~$1,400) and move the 2024–2026 years from clearly negative to about flat. the two changes
with the most consistent sign are the time stop (`h_l40w90`: 4/5 years better than base) and the
tighter strong-core hourly condition (`h_core40`: 3/5 better, 1 flat, 2022 −61).
| `h_be05` — breakeven stop 0.5 % (now functional) | +1,732 | −646 | −83 | −363 | −246 | +394 | 1,368 | | 1,951 |
| `h_l40w90be05` — L40/W90 + breakeven 0.5 % | +1,553 | −574 | +219 | −289 | +97 | +1,006 | 1,437 | | 1,273 |
| `h_c40l40` — core s1h −0.40 + L40/W90 | +1,357 | −50 | +77 | −105 | −11 | +1,268 | 1,127 | | 1,073 |
| **`h_c40l40be05` — all three = v16** | +1,380 | −177 | +229 | +99 | +153 | **+1,684** | 1,179 | 1.12 | **702** |

v16 per year: PF 1.26 / 0.91 / 1.10 / 1.04 / 1.08, win rate ~34 %, exit mix 62 % max-hold,
23 % breakeven, 8 % session close, 6 % score exit, 2 % hard stop. positive months 32/57 (base 28/57).

### 10.5 decision: v16 promoted (row 10, migration 20260912000007, 2026-09-12 ~11:00 PT)

- v16 = v15 + `max_hold.loss_reduction_ms` 3,000,000 (losing-side limit 40 min) +
  `profit_extension_ms` 0 (winning limit 90) + `window_strong_core_short` hourly ≤ −0.40 +
  `breakeven.trigger_pct` 0.005 (the monitor now actually exits; `exit_reason = 'breakeven_stop'`).
- each change was tested alone (§10.4) and the stack is better than v15 in 4 of 5 years under
  honest costs (2022 gives back $472 of $1,852) with max drawdown $2,333 → $702.
- promoted blob reproduces the `h_c40l40be05` sweep trade-for-trade on 2022-06-13, 2024-03-15,
  2025-04-04, 2026-06-04. live binary (rebuilt with the breakeven fix) loads row 10, builds four
  engines, subscribes. migration 20260912000006 (`breakeven_stop` enum value) applied.
- the honest frame for monday: PF 1.12 over five years, thin. week 1 = does live behave like the
  replay (fills vs bar opens, exit mix, ~1.2 trades/day, many small breakeven exits).
- note on the long book: the old cost model charged longs correctly (buy pays up, sell receives
  less), so the §8 long-book numbers stand; only the short-side numbers were inflated.
- not done: the reject gates are inert at current thresholds and could be removed; the exact
  losing-limit optimum (30–40 min) is within noise and was not tuned further.

## 11. entry research, round two (2026-09-12 afternoon →)

**target set by the user: overall honest-cost profit factor > 1.3 over 2022–2026 before the
config is "good enough for the week".** v16 is at 1.12.

method: every entry idea is one indicator instance (score −1..+1, fires on the bar it completes),
tested (a) standalone as a short window and (b) as an added condition on v16's windows, five-year
cached replay, honest costs, one variant at a time. screening first on the labelled bar set from
§10 (does the feature separate top-5 % bars from the rest, per year), replay only for survivors.
mixing is window JSON only — nothing is baked into the engine.

candidates: multi-bar candle sequences (`candle_sequence` indicator: engulfing, star,
three_crows, pin_bar, cloud_cover, harami, inside_break, three_bar_reversal, first_reversal; 1m and
5m); cross-ticker context (SPY/QQQ session return, peers already red); gap and prior-day levels
(prior-day low break, opening-range low break as a short trigger); OFI / VPIN as hard conditions;
calendar exclusions (FOMC, earnings-adjacent).

### 11.1 tooling (commits 12b2707, 1e56161)

- indicators: `candle_sequence` (pattern per instance, closed candles only), `prior_day_levels`,
  `cross_context` (new `MarketState.cross`, filled by the backtest from the bar cache with
  `--cross-index SPY`; live leaves it `None` so a condition on it fails safe until the feed is
  wired), `event_calendar` (built-in FOMC decision days 2022–2026).
- `--dump-ticks` now carries every indicator score/metadata as a json column;
  `--dump-window-only` stops at `no_new_entries_after`. SPY and QQQ added to the bar cache.
- `research/entries/screen_indicators.json`: 28 candidate features at weight 0. sweep tag
  `scr16` (v16 + these) reproduces v16's trades exactly in every year and dumps the features on
  every morning bar (2.3 GB). earnings 8-K item-2.02 dates for the four names from SEC EDGAR in
  `research/entries/data/`.
- `research/entries/make_variant.py`: feature → `--patch-json` as a standalone short window, as an
  added condition on v16's two windows, or both. `research/entries/summarize.py`: per-year,
  per-window P&L / PF / drawdown for any set of sweep tags.
- process: offline screen (agent, on the labelled bars from §10) → real five-year replay under
  honest costs for survivors, one variant at a time → combine only what survived alone.

### 11.2 offline screen results (agent; full report `docs/analysis/2026-09-12_entry_screen.md`)

564,474 morning bars, join rate 100 %. the reference is v16's own windows simulated with its exit
stack (+1,723 / 1,179 trades / PF 1.12, within 2 % of the real replay).

- **no feature works as a trigger.** all 28 features (and 90 bucket definitions) have negative mean
  short value on the bars where they fire, and all lose money as standalone short windows. v16's
  window is the only positive standalone trigger in the set.
- **multi-bar candle patterns are confirmations, not early signals.** in 16 of 18 pattern/timescale
  cases the pattern is better when the 5-minute score is already ≤ −0.40 than when it is bullish,
  and in every case the "5m still bullish" cell (where the best shorts live) is at or below the
  base rate. none survives as a window condition. a 5m bearish harami *after* a sell-off is the
  strongest negative signal in the set (−0.35 % mean, 235 episodes, negative every year).
- **two filters on v16's windows pass the bar** (keep ≥ 40 % of trades, raise the per-trade mean in
  ≥ 4 of 5 years, smooth threshold sensitivity):
  1. require SPY session return within ±0.2 % → sim +2,741 / 752 trades / PF 1.33 (±0.3 %: PF 1.31)
  2. exclude bars already > 1 % below the prior-day low → sim +2,498 / 778 / PF 1.31
- **one pair beats both parents in 5/5 years:** SPY flat ±0.2 % **and** VPIN top quintile (raw
  ≥ 0.217) → sim +2,769 / 468 trades / **PF 1.64**, every year positive (2023 +31). robust to the
  band (±0.3 % → 1.50) and the VPIN cut (0.18 → 1.53, 0.26 → 1.63). adding filter 2 → PF 2.01 on
  278 trades (about one a week).
- reading: short idiosyncratic weakness; don't chase a market-wide or already-extended move.
  v16's 2022 profit came largely from big gap-down / SPY-down mornings; in 2023–2026 those mornings
  are where it loses.
- negative: opening-range breaks (2022-only), OFI quintiles, peers-red, gap-holding, FOMC and
  earnings-day exclusions (v16 barely trades those days), time-of-day.
- caveat found by the agent: `cross_1m` was null on every bar of the dump — the replay never
  assigned `MarketState.cross` (a failed text replacement in `replay.rs`); the SPY features were
  recomputed offline from `data/bars/SPY.csv`. fixed before the replay tests below.

### 11.3 real-replay results and v17 (promoted row 11, migration 20260912000008)

honest costs, five years, 36 % sizing, filters as added conditions on v16's two windows
(tags `e_*`; every row within a few percent of the offline simulation):

| variant | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | trades | PF | maxDD |
|---|---|---|---|---|---|---|---|---|---|
| v16 | +1,380 | −177 | +229 | +99 | +153 | +1,684 | 1,179 | 1.12 | 702 |
| SPY within ±0.2 % | +1,674 | −150 | +173 | +570 | +435 | +2,703 | 752 | 1.32 | 662 |
| SPY within ±0.3 % | +2,083 | −60 | +182 | +393 | +389 | +2,987 | 876 | 1.30 | 603 |
| not > 1 % below prior-day low | +1,693 | +253 | +277 | +668 | −268 | +2,624 | 774 | 1.33 | 496 |
| not > 1.5 % below prior-day low | +1,433 | +196 | +477 | +639 | −247 | +2,498 | 950 | 1.25 | 615 |
| VPIN raw ≥ 0.217 | +1,308 | −64 | +149 | +42 | +584 | +2,019 | 840 | 1.22 | 578 |
| gap in (−1, +0.3) % | +350 | +152 | +568 | +182 | +244 | +1,496 | 444 | 1.35 | 364 |
| **SPY ±0.2 % + VPIN ≥ 0.217 (v17)** | +1,304 | +60 | +158 | +588 | +675 | **+2,785** | 469 | **1.64** | 401 |
| SPY ±0.2 % + prior-low filter | +1,217 | +214 | +162 | +744 | +30 | +2,367 | 479 | 1.49 | 304 |
| all three | +789 | +337 | +236 | +732 | +289 | +2,383 | 277 | 2.03 | 178 |

candle patterns in the real replay (standalone 5m window with composite ≤ −0.20, and as an added
condition): engulfing −2,820 / PF 0.81; evening star −967 / 0.69; three black crows +62 / 1.03;
as conditions they leave 1–30 trades a year. consistent with the screen: confirmations, not
triggers. (remaining six patterns were queued behind the filter tests; see §11.4 when run.)

**v17 = v16 + SPY-flat + VPIN filters.** chosen over the triple (PF 2.03, ~1 trade/week, three
filters stacked on the same data) for frequency and because the pair was the screen's
pre-registered 5/5 candidate. clears the user's PF > 1.3 bar with every year positive and
positive months 38 of 56. promoted blob reproduces `e_spy02_vpin_cond` trade-for-trade on five
sample days. live: `data_feed/src/cross_tracker.rs` subscribes SPY and fills `MarketState.cross`
(session return vs the first RTH bar's open, same definition as the replay).

### 11.4 remaining candle patterns (standalone 5m short window, composite ≤ −0.20, honest costs)

| pattern | 5y P&L | trades | PF | years > 0 |
|---|---|---|---|---|
| engulfing | −2,820 | 1,222 | 0.81 | 1 |
| evening star | −967 | 232 | 0.69 | 1 |
| three black crows | +62 | 186 | 1.03 | 3 |
| shooting star (pin bar) | −157 | 212 | 0.94 | 2 |
| dark cloud cover | −58 | 14 | 0.58 | 3 (n ≤ 4) |
| bearish harami | −1,264 | 324 | 0.71 | 1 |
| inside-bar breakdown | −1,234 | 1,024 | 0.90 | 2 |
| three-bar reversal | −4,340 | 1,876 | 0.81 | 0 |
| first red after three green | −667 | 466 | 0.88 | 3 |

none is a trigger. the pattern module stays in the tool belt (it is one config line to try any
of them on 1m, with a different composite ceiling, or as a long-side trigger later).

### 11.5 monday readiness checks (2026-09-12 evening)

- headless dry run of `/preopen-check` (sonnet, $0.20, 10 turns): read row 11 (v17) on all four
  `engine_state` rows, positions flat, feed not stale, timers correct for monday 06:10 PT; it
  also noticed that `engine_state` had been written by a manual `paper_trader` run outside
  systemd (the smoke test) — the kind of anomaly it should flag. the check-in plumbing works
  with the v17 schema (new `breakeven_stop` exit reason, cross feed).
- the `git push` deny moved from `.claude/settings.json` (where it also blocked interactive
  sessions) into the three check-in service invocations (`--disallowedTools`).
- in flight: SPY-band × VPIN-cut plateau grid (tags `e_grid_*`), one-bar-lag cross-context
  replay (`e_v17_lag1`, emulates live latest-bar timing), long windows with and without the two
  filters (`e_long_filters`, `e_long_unfiltered`). results in §11.6.

### 11.6 v17 validation: threshold plateau, live-timing lag, long side

**plateau** (SPY band × VPIN cut, honest costs, five years; v17 = ±0.20 % / 0.217):

| band \ VPIN | 0.18 | 0.217 | 0.26 |
|---|---|---|---|
| ±0.15 % | +2,591 / 477 / 1.55 | +2,262 / 414 / 1.57 | +1,972 / 343 / 1.60 |
| ±0.20 % | +2,736 / 531 / 1.52 | **+2,785 / 469 / 1.64** | +2,237 / 392 / 1.62 |
| ±0.25 % | +2,742 / 595 / 1.46 | +2,692 / 519 / 1.53 | +1,947 / 427 / 1.45 |
| ±0.30 % | +3,068 / 652 / 1.47 | +2,937 / 576 / 1.53 | +1,970 / 473 / 1.41 |

(P&L / trades / PF.) every cell is positive over five years and every cell's PF is ≥ 1.41; v17
is the best cell but sits on a broad plateau, not a spike. a wider band buys trades at a lower PF.

**live timing** (`--cross-lag 1`: SPY and peer context served one minute late, which is the worst
case for the live tracker's latest-bar lookup): +2,595 / 470 trades / PF 1.57, every year
positive (2022 +1,123 · 2023 +92 · 2024 +202 · 2025 +437 · 2026 +741). the filter is not
sensitive to intra-minute ordering.

**long side with the same two filters** (long windows re-enabled alongside v17's shorts; long
book only): 2022 +545 · 2023 −645 · 2024 −432 · 2025 −1,524 · 2026 +346 = −1,710 over five
years (unfiltered long book under honest costs: −3,780). the filters help but the long "strong
core" window is a consistent loser (−381 / −369 / −951 in 2023–2025) and the long "5m thrust"
is mixed (+529 / −158 / +27 / −249 / +167). the long book stays disabled.

### 11.7 v17 robustness: leave-one-year-out, tickers, windows, costs

- **leave-one-year-out** over the 12 grid cells: choosing the cell with the best pooled PF on
  the other four years picks v17's cell in 4 of 5 folds (2022 held out picks ±0.20 % / 0.26,
  PF 2.08 on 2022). every held-out year is positive: 2022 +899 · 2023 +60 · 2024 +158 ·
  2025 +588 · 2026 +675.
- **per ticker (5y)**: NVDA +1,441 (167 trades, PF 1.87) · MSFT +708 (118, 1.77) · AAPL +343
  (66, 1.64) · AMZN +293 (118, 1.24; negative 2023–2024). no single name is the result.
- **per window**: 5m thrust short +2,053 (421 trades) · strong core short +732 (48).
- **cost sensitivity**: see below (tags `e_v17_cost5`, `e_v17_cost10`).
- **live hygiene**: the cross tracker now withholds context when the SPY bar is > 2 min older
  than the ticker's bar, so a stalled index feed cannot keep a stale "market is flat" verdict
  alive; the intraday skill's churn threshold changed from "3 × backtest mean" (would trip on
  any two-trade day at 0.4/day) to "> 4 trades in a day".

**cost sensitivity** (v17, five years; per-leg slippage + half-spread charged against the trade):

| cost model | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | PF | maxDD |
|---|---|---|---|---|---|---|---|---|
| 3 bps + $0.005 (assumed) | +1,304 | +60 | +158 | +588 | +675 | +2,785 | 1.64 | 401 |
| 5 bps + $0.005 | +1,089 | +12 | +12 | +407 | +579 | +2,099 | 1.44 | 441 |
| 10 bps + $0.01 | +666 | −290 | −374 | +78 | +255 | +336 | 1.06 | 961 |

the edge is ~7 bps per leg deep. realized slippage on the paper account is therefore the most
important measurement of week 1; the eod-review skill now computes it from
`broker_entry_price` / `broker_exit_price` vs the engine's fill and flags a running mean
> 10 bps round-trip as a strategy-level problem.

## 12. week 1, day 1 (monday 2026-09-14)

- trader started 06:10:02 PT by the timer, no restarts, config row 11 (v17), four engines, SPY
  subscribed. one trade: NVDA short 09:35→10:15 ET, max_hold (40-min losing limit), −$24.43;
  realized cost ≈ 4.6 bps round trip (engine 209.690 vs broker 209.600 on the sell; 211.435 vs
  211.441 on the buy-back) — inside the 6 bps assumption. SPY context present from the 09:31 bar
  on (the only `cross_1m none` near-misses are the 09:30 bar); VPIN was the binding filter later.
- pre-open check ran (06:15, "preopen ok"). **intraday check-ins all failed**: the headless
  `claude -p` calls used `ANTHROPIC_API_KEY` from `.env`, which bills the API account, and it had
  no credit. so no LLM watchdog ran on day 1.
- fix (same day): the check-in units now `UnsetEnvironment=ANTHROPIC_API_KEY` and run on the
  claude.ai login (verified headless inside a transient systemd unit); a no-LLM
  `scripts/watchdog.sh` runs every 5 min 06:25–13:15 PT (`watchdog.timer`) and pushes ntfy alerts
  for: service down, heartbeat > 3/6 min, feed stale, position held past 11:58 ET, daily loss
  > 3 %/5 %, > 4 trades, and a failed check-in (its first run flagged the failed intraday job).
  the intraday LLM review is on-demand (`/intraday-review` or `/loop 15m /intraday-review` from a
  session); pre-open (06:15) and end-of-day (13:30) stay scheduled. ntfy is configured and tested.

### 12.1 v18 (row 12, migration 20260912000009) and the ticker basket test

- **live/replay mismatch found**: the replay runs tickers independently (no cross-ticker cap);
  live had `max_concurrent_positions` 1. applying that cap to v17's five-year trades: 349 of 469
  kept, +1,438 / PF 1.40, the 120 dropped worth +1,347. with 2: 440 / +2,524 / 1.61; with 3:
  463 / +2,587 / 1.60. day 1 blocked three tickers 09:36–10:06 ET while NVDA was short.
- **v18** = v17 + `max_concurrent_positions` 3, `max_capital_deployed_pct` 0.95 (the
  deployed-capital cap is only enforced when the feed sets `total_deployed_capital`, which it
  does not yet — the concurrency cap is the binding one). exposure up to ~90 % of the budget.
- **ticker basket** (pre-registered rule: include the basket if pooled PF holds; drop a name only
  if its PF < 1.2 or it is negative in ≥ 3 of 5 years): META, GOOGL, TSLA, AMD, AVGO, NFLX,
  replayed with v17 unchanged (tag `e_basket`). results in §12.2.

### 12.2 ticker basket result: rejected

v17 unchanged, six pre-registered names, five years, honest costs (tag `e_basket`):
pooled −201 / 1,140 trades / PF 0.99; 2022 +1,515 then −591 / −700 / +211 / −636. per name:
AMD +670 (PF 1.25, 2 negative years) · NFLX +242 (1.12, 3 neg) · META +178 (1.08, 3 neg) ·
GOOGL −49 (0.96) · TSLA −142 (0.97, 3 neg) · AVGO −1,101 (0.62). by the rule stated in advance
(pooled PF must hold; drop a name at PF < 1.2 or ≥ 3 negative years) the basket is rejected
and only AMD survives — one survivor of six is the selection effect the rule exists to catch,
and its PF is below the 1.3 bar anyway. **the edge is not a generic mega-cap property; it lives
in the four names.** volume comes from v18's concurrency, not from more tickers.

day-1 eod review (first run on the claude.ai login, 36 turns): "plumbing ok · strategy hold",
realized cost 4.6 bps vs 6.5 assumed (n = 1), four triggers recorded in memo 4. it also caught
that the skill's own replay command lacked `--bars-dir`/`--cross-index` (would have reported a
false plumbing mismatch every day) — fixed in the skill.

### 12.3 are the v17 filters too aggressive? (asked tuesday 2026-09-15)

v16 (no filters) vs v17 (filters), five years, honest costs, matched on ticker + entry time:

| set | trades | P&L | PF | win |
|---|---|---|---|---|
| kept (in both) | 312 | +2,040 | 1.69 | 40 % |
| removed by the filters | 867 | −356 | 0.97 | 32 % |
| added (slot freed for a later entry) | 157 | +749 | 1.53 | 34 % |

the removed set is break-even in aggregate — the filters did not throw away good trades. by
filter (SPY session return from the bar cache, VPIN from the tick dump at the signal bar):
SPY-only removed 293 / −116 (PF 0.97); VPIN-only 331 / +181 (1.04); both 241 / −379 (0.89).
SPY-removed trades split by direction: SPY down > 0.2 % 441 / −183; SPY up > 0.2 % 93 / −312;
SPY down > 0.5 % 160 / −272 — shorting into a market-wide move loses in every bucket.

the one real cost is regime-dependent: **VPIN-only removals were +674 (PF 1.49) in 2022** and
−262 / −424 in 2023 / 2026. in a strongly trending-down year the VPIN condition gives back
money; in choppy years it is the filter that saves the book. removed per year (all filters):
2022 +758 · 2023 −201 · 2024 +126 · 2025 −609 · 2026 −429. that is the same "2022 gives back
$470" trade-off accepted when v17 was chosen; nothing in the removed set argues for loosening.

`scripts/analysis/near_miss_replay.py <date>` replays a day's live near-miss bars as hypothetical
shorts (v18 exit stack) once that day's bars are in the cache.
