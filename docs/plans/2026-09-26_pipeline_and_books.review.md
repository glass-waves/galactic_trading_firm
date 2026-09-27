# review: pipeline + shadow books + stress mode plan (2026-09-26)

state found: `--config-id` is already committed (76e1dd4, `backtest/src/main.rs:1017,1089`); the migration is
drafted but unapplied (`_sqlx_migrations` top = 20260912000009); promoted row 12 (v18): session `11:30 / 11:55`,
`sizing_fixed 0.30`, `exit_threshold −0.30`, `exit_session {force_exit_by 11:55}`, both live windows carry **no**
`entry_before` / `exit_overrides`. severity: **B** blocker · **S** should-fix · **N** nice-to-have.

## 1. multi-book trader (§3)

- **B1 rollback path is broken.** `state_writer.rs:78` upserts `ON CONFLICT (ticker)`. after the PK becomes
  `(book, ticker)` there is no unique index on `ticker` alone → every upsert fails with 42P10 → the old binary
  never heartbeats → watchdog/preopen go critical Monday. §7.4's claim is wrong. fix: ship
  `scripts/rollback_books.sql` (`DELETE FROM engine_state WHERE book<>'primary'; ALTER TABLE engine_state DROP
  CONSTRAINT engine_state_pkey, ADD PRIMARY KEY (ticker);`) and rewrite §7.4 as "restore
  `paper_trader.pre-books` **and** run the rollback sql"; A tests both directions. (splitting the PK swap into a
  second migration does not help: `paper-trader.service:18` runs `sqlx migrate run` on every start.)
- **S1 primary-behaviour changes hidden in the refactor** (byte-for-byte holds only in `alpaca_paper` mode):
  `SimulatedBroker` has one `last_price` (`broker.rs:70,157`), so today `flatten_all` fills every simulated
  ticker at the last-arrived ticker's price; per-ticker prices fix that. `apply_pending_to` (`main.rs:234-236`)
  removes the `state_builders` entry when a config drops a ticker — with shared builders it must not;
  `delete_engine_state` (`main.rs:943`) becomes `(book, ticker)`. demo feed emits bars only for the primary's
  tickers (`main.rs:576-620`) → emit the union + SPY.
- **S2 invariant "shadow never reaches Alpaca" — make it structural.** `Book::new_shadow(…)` constructs
  `SimulatedBroker` inside, `Book.broker` is private, the only `AlpacaBroker::new` call site is the primary
  constructor, `--replay-bars` overrides `broker_mode` in code. test: `Book::from_spec(shadow, "alpaca_paper")` is simulated.
- **S3 per-book vs process-wide, confirm each moves into `Book`:** capacity (`main.rs:741`), `daily_pnl` + roll
  (`main.rs:93,717`), `pending` + `apply_pending_where_flat` (`main.rs:253`), gate/near-miss dedupe maps
  (`main.rs:292-334`), the clock safety net's `force_exit_by` (`main.rs:911`). stays shared: `cross`,
  `state_builders`, `last_bar_at/last_prices/feed_stale`, `tick_count`.
- **S4 old binary vs schema:** `trade_writer.rs:69-88` INSERT lists no `book` → default OK; block-event INSERT
  → default OK; the engine_state UPSERT → **fails** (B1).
- **S5 sessions/hosting counter (answers §8 Q1):** add `book_sessions (book, session_date) PK, first_bar_at,
  last_bar_at, bars INT, trades INT, realized_pnl, config_version_id` upserted by the trader on a book's first
  bar of the day and every heartbeat. exact session count, "not hosted" detection for free, per-day P&L without
  scanning trades. `entry_block_events` only *mostly* works (throttled, absent on quiet days, `no_new_entries_after`
  happens to fire daily). this is schema → decide before fan-out.

## 2. replay harness (§3.2) — feasible, small, but these make it lie

- **S6** heartbeat safety net (`main.rs:911-918`) is **not** gated by `market_open_now` → flattens 30 s into
  any replay run after 11:57 ET wall-clock; disable the heartbeat arm and the config-reload arm in replay.
- **S7** `close_and_record` / `flatten_all` stamp `Utc::now()` (`main.rs:169`) → exit_time = today, negative
  hold; take `now` as a parameter (last bar ts in replay). `Some(ev) = bar_rx.recv()` never sees `None`
  (branch just disables) → explicit end-of-bars: flatten at last bar ts, final persist, `break`.
- **S8** replay writes `engine_state` / `entry_block_events` as `book='primary'` for a 2025 date → pollutes
  what watchdog, skills and near-miss replay read. write them under `book='replay:<name>'` (startup cleanup
  deletes the state rows) or skip. replay-live trades: repeated runs duplicate → write a CSV under
  `data/live/replay/` and, if writing the DB, delete prior `(source='replay-live', book, date)` rows first.
- **S9 parity command (§3.3) is not like-for-like:** add `--lookback-days 8` (default 5, `main.rs:1006`); live
  `pnl_dollars` is engine-side and cost-free (broker fills are separate columns) while the replay applies
  3 bps + $0.005 → compare against `--slippage-bps 0 --half-spread 0`, then broker_* against the cost model.
  push SPY's bar first at each timestamp (= backtest lag 0) and say so. key check `main.rs:398` must exempt
  replay mode. 2025-04-04 is fine (5 replay trades in `data/iex_v18_2025_trades.csv`).

## 3. schema (§1) — the drafted migration matches the plan; changes needed

- **B2** `config_versions.created_by` is the `agent_type` enum (`agent_analysis, agent_pm, orchestrator,
  human, claude_intraday, claude_eod`) and `mutation_reason` is `NOT NULL`. §4.2.1 "created_by naming the
  candidate" cannot work. add `ALTER TYPE agent_type ADD VALUE IF NOT EXISTS 'pipeline';` (precedent:
  20260912000001) and put the candidate name in `mutation_reason`.
- **S10** `pipeline_candidates.name UNIQUE` forbids re-proposing after cooldown. replace with a partial unique
  index `ON (name) WHERE stage NOT IN ('backtest_failed','shadow_failed','rejected','withdrawn','promoted')`;
  `propose` refuses while the latest terminal row's `cooldown_until` is in the future.
- **S11** add `CHECK ((role = 'primary') = (name = 'primary'))` so 'primary' is actually reserved. circular FK is
  harmless (create book + set `shadow_book` in one transaction); keep.
- **S12** `source='shadow'` vs `book`: right split, two holes — in `--demo` every book must write `source='demo'`,
  and cockpit `trades.ts:36,56` has no source filter (dashboard totals will include shadows; unowned, note it).
- **S13 readers of `engine_state` that break or mislead with shadow rows:** `watchdog.sh:36-66` (per-row loop:
  `hb_$t`/`late_$t` keys collide across books, daily-P&L read from whichever AAPL row comes first),
  `export_day.sh:12-14`, `preopen-check/SKILL.md:33`, `intraday-review/SKILL.md:42`, `eod-review/SKILL.md:52`,
  and **`scripts/analysis/near_miss_replay.py:17` (owned by nobody → give to D)**. cockpit reads no engine_state.

## 4. pipeline stages and gates (§4)

- **B3 `session` patch must mirror the CLI.** `--force-exit-by` rewrites `session.force_exit_by` **and** every
  `session_close` action's `force_exit_by` param (`backtest/src/main.rs:1105-1114`); v18's `exit_session`
  holds `11:55`. a materialized stress config with only `session` shallow-merged keeps the action at 11:55 →
  the shadow flattens every afternoon position at 11:55 while the sweep held them: guaranteed parity failure.
  materializer must do the same rewrite (unit test it). cleaner: owner adds `session` (with that mirror) and
  `tickers` keys to `--patch-json` (`main.rs:583-600`, ~15 lines) before fan-out so B's materializer and C's
  sweeps share one semantic; today `--patch-json` knows only `disable/indicators/actions`.
- **S14 idempotency/crash:** materialize (INSERT row) + candidate update in one `BEGIN…COMMIT` fed to
  `psql.sh` on stdin with `$json$…$json$` quoting (not `-c` string interpolation). `backtesting` older than 3 h
  with no live sweep → resume (tag `cand_<id>` is deterministic; `run_cached_sweep.sh:11` removes old csvs).
- **S15 overlap:** `research-runner.service` starts 20:00 with a 6 h timeout; systemd `After=` does not wait for
  an already-running unit. take `flock logs/sweeps.lock` in both `research_runner.sh` and `pipeline.py`
  (B may only add it to its own; owner adds one line to the runner). `git add data/*.args` would sweep up C's
  research `.args` → add only `data/cand_*.args`.
- **S16 not-hosted shadow:** stage `shadow` with no `book_sessions` row within 3 trading days of
  `shadow_started_at` → plumbing flag + stop the clock (do not let "60 sessions regardless" turn a build
  failure into `shadow_failed` + 180 d cooldown). after 60 sessions with trades < min → `insufficient_activity`,
  not a PF verdict on 3 trades.
- **S17 sizing mismatch:** the 5y gate sweeps run at `--sizing-fraction 0.36 --max-position-pct 0.36`
  (`data/iex_v18.args`) while live/shadow run the blob's 0.30 / 0.30. parity replay must pass **no** sizing
  override; per-trade bps comparison is sizing-free but "no year < −300" is at 36 % — state it in status.md.
- **S18 bar fetch:** `run_fetch_bars` (`main.rs:1656-1672`) logs a chunk `ERROR` and continues → silent 20-day
  holes; verify bars per month after fetching and re-fetch gaps. fetch from 2021-12-20 (warm-up). rate: 20-day
  chunks, ~2 pages each, 400 ms gap → ≤ ~150 req/min per ticker, 3 retries with 1/2/4 s backoff; two per run is
  fine. `export_day.sh:31` does merge daily IEX bars, but only for the four names + SPY — D's union change is required.
- **S19 gates vs the docs:** `default-ticker` is stricter than §12.2's rule (AMD PF 1.25 / 2 neg years fails
  both) — fine. `default-config` "trades ≥ baseline" excludes every quality candidate (§15.1: VPIN 0.26, PF 1.58,
  306 trades) — say it is intentional or add `quality-config`. `stress-mode` "P&L/day ≥ 2 × baseline" is ≈ $20/day
  (baseline $258 on 25 days) — trivially met; add stress-subset PF ≥ 1.3 and ≥ 30 stress trades; "ordinary-day
  book unchanged" = trade-identical (entry ts, ticker) on non-stress days, not P&L. define the SPY day return as
  the trigger does (vs first RTH open). 2026 data ends 09-10 (`run_cached_sweep.sh:13`) for candidate and baseline.
- **N** materializer: append-**or-replace** by `instance_id` (`indicator_configs` keeps duplicates otherwise).
  ticker-candidate fairness is OK: replay peers = `config.tickers` (`main.rs:1238`) = `[T]` = the book's peers.

## 5. stress study (§5) — exact recipe for agent C

- **B4 sign bug:** shorts score-exit when `composite >= -threshold` (`tick_loop.rs:261-262`). `0.6` means
  "exit when composite ≥ −0.6" — every stress short (entered at composite ≤ −0.35) exits on the next bar. use
  `score_exit_threshold: -0.6` (exit at +0.6) or `-10` to disable.
- **B5 clocks:** the `no_new_entries_after` gate (`tick_loop.rs:580-587`) runs before any window, so a window's
  `entry_before 15:30` above the session's 11:30 is dead; and once the session is widened the morning windows lose
  their 11:55 flat unless they carry it themselves. recipe (what the EOD study did, `data/e_eod_1530_t2.5.args`):
  `BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh stress_<cell> --sizing-fraction 0.36 --max-position-pct 0.36
  --cross-index SPY --no-new-entries-after 15:30 --force-exit-by 15:55 --patch-json research/stress/<cell>.json`
  with the patch: `disable: [window_5m_thrust_short, window_strong_core_short]`; `actions`: the two `_am` copies
  from `research/eod/eod_1530_t2.5.json` (conditions verified identical to row 12; `entry_before 11:30`,
  `exit_overrides.force_exit_by 11:55`) plus the stress windows (`entry_after 09:45`, `entry_before 15:30`,
  `exit_overrides {force_exit_by 15:55, max_hold_ms …, score_exit_threshold −0.6}`). the SPY trigger needs no
  second instance: `cross_1m.index_session_ret` metadata is in percent (`cross_context.rs:42`) →
  `indicator_max … -0.5`; the name's move is `pdl_5m.dist_close_pct` (`level_signals.rs:75`, signed %), but
  `pdl_5m` is **not** in v18 → append it (weight 0, as the EOD patch does). the delivered
  `stress_mode_v1.patch.json` must carry `session: {no_new_entries_after: "15:30", force_exit_by: "15:55"}` (B3).

## 6. workstream conflicts and order

- owner before fan-out: apply the migration with B2/S5/S10/S11 folded in; copy `target/release/paper_trader` to
  `.pre-books` (A's first build overwrites it); drop §7.1's `--config-id` step (done); consider B3's patch-json
  change. A should build `-p data_feed` only — a workspace release build relinks `backtest` under B's and C's
  running sweeps.
- unowned files that need edits: `scripts/analysis/near_miss_replay.py` (→ D), `scripts/research_runner.sh`
  flock line (→ owner), cockpit (leave, note). D's `install.sh` must use the unit name B creates (`pipeline.timer`).
  no two agents share a file; B and C share `data/bars_iex` read-only (B's AMD/META fetch adds new files only).

## 7. answers to §8

- **Q1** `book_sessions` written by the trader (S5); `entry_block_events` stays diagnostics.
- **Q2** `export_day.sh` runs the same-day replay for every enabled book through one shared
  `scripts/replay_book.sh <book> <date>` (reads the `books` row: `--config-id` or promoted, `--tickers`, capital,
  no sizing override) into `data/live/<date>/replay_<book>.csv`; the pipeline's evaluate step only aggregates
  those files and backfills missing days with the same script. one place, versioned in git, visible daily.
- **Q3** 8 is fine; the binding limit is websocket symbols (free IEX plan: 30 per connection) → add
  `MAX_SYMBOLS=25` and refuse (error) rather than skip a shadow that would exceed it.
- **Q4** define: a `config` candidate may set `tickers`; its baseline is the promoted row replayed on the
  candidate's ticker set with identical args (tag `base_<sha>`, cached), never `iex_v18` unless the sets match;
  a `ticker` candidate is patch `{}` on `[T]`. any candidate whose set differs from the primary's gets its
  promotion row sweep-tested as the full row (`--config-id <proposed>`) before the proposal block is written.
