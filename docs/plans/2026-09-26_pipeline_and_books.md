# research pipeline + shadow books + stress mode — implementation plan (2026-09-26)

owner: the session mastermind (this plan is the contract). implementers: four parallel agents (A–D),
each with an exclusive file set. the human promotes; nothing below changes that.

## 0. what we are building, in one paragraph

a lane that candidate edges and candidate tickers move through like code through CI: **proposed →
backtest gate → shadow trial (live, same bars, simulated fills) → evaluation gate → promotion
proposal → human promotes**. the live trader becomes a host for several *books*: the **primary**
(latest promoted config, real paper broker, unchanged behaviour) plus **shadow** books (a config
row or a ticker set on a simulated broker, own trade tag). a nightly runner advances candidates.
first candidates through the lane: the bear-bounce long book, one or two tickers, and a
**stress mode** for violent down days (studied in parallel by agent C). the watchdog gains an
abnormal-move alert.

## 1. schema (migration `migrations/20260926000001_pipeline_books.sql`) — written and applied by the owner before fan-out

```sql
-- books: engines hosted by the live trader
CREATE TABLE books (
    name               TEXT PRIMARY KEY,                       -- 'primary' is reserved
    role               TEXT NOT NULL CHECK (role IN ('primary','shadow')),
    config_version_id  BIGINT REFERENCES config_versions(id),  -- NULL = follow the latest promoted config
    tickers            TEXT[],                                 -- NULL = the config's own tickers
    capital            DOUBLE PRECISION,                       -- NULL = the process capital
    enabled            BOOLEAN NOT NULL DEFAULT true,
    purpose            TEXT NOT NULL DEFAULT '',
    candidate_id       BIGINT,                                 -- pipeline_candidates.id when the book is a trial
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by         TEXT NOT NULL DEFAULT 'human',
    retired_at         TIMESTAMPTZ,
    retire_reason      TEXT
);
CREATE UNIQUE INDEX books_one_enabled_primary ON books (role) WHERE role = 'primary' AND enabled;
INSERT INTO books (name, role, purpose) VALUES ('primary', 'primary', 'live book: latest promoted config on the real paper broker');

ALTER TABLE trades ADD COLUMN book TEXT NOT NULL DEFAULT 'primary';
CREATE INDEX idx_trades_book_exit ON trades (book, exit_fill_at DESC);
-- shadow trades are written with source = 'shadow' (and book = <name>), so every existing
-- `source = 'paper'` query keeps its meaning without edits.

ALTER TABLE engine_state ADD COLUMN book TEXT NOT NULL DEFAULT 'primary';
ALTER TABLE engine_state DROP CONSTRAINT engine_state_pkey;
ALTER TABLE engine_state ADD PRIMARY KEY (book, ticker);
ALTER TABLE entry_block_events ADD COLUMN book TEXT NOT NULL DEFAULT 'primary';
CREATE INDEX idx_entry_block_events_book_ts ON entry_block_events (book, ts DESC);

-- pipeline_candidates: one row per idea moving through the lane
CREATE TABLE pipeline_candidates (
    id                          BIGSERIAL PRIMARY KEY,
    name                        TEXT UNIQUE NOT NULL,             -- 'ticker:AMD', 'bear-bounce-sma50', 'stress-mode-v1'
    kind                        TEXT NOT NULL CHECK (kind IN ('ticker','config')),
    base_config_version_id      BIGINT REFERENCES config_versions(id),  -- what it builds on (the promoted row at proposal time)
    patch                       JSONB,                            -- config kind: {"disable":[],"indicators":[],"actions":[],"session":{}} ; ticker kind: {"ticker":"AMD"}
    tickers                     TEXT[],                           -- the set the candidate trades (ticker kind: [T]; config kind: NULL = base's)
    stage                       TEXT NOT NULL DEFAULT 'proposed' CHECK (stage IN (
                                    'proposed','backtesting','backtest_failed','backtest_passed',
                                    'shadow','shadow_failed','shadow_passed',
                                    'promotion_proposed','promoted','rejected','withdrawn')),
    gate                        TEXT NOT NULL DEFAULT 'default',  -- named accept-if rule set (scripts/pipeline/gates.py)
    materialized_config_id      BIGINT REFERENCES config_versions(id),  -- config kind: base blob + patch, inserted at backtest time
    backtest_tag                TEXT,                              -- sweep tag under data/
    backtest_result             JSONB,  backtest_at TIMESTAMPTZ,
    shadow_book                 TEXT REFERENCES books(name),
    shadow_started_at           TIMESTAMPTZ,
    shadow_min_sessions         INT NOT NULL DEFAULT 20,
    shadow_min_trades           INT NOT NULL DEFAULT 15,
    shadow_result               JSONB,  shadow_evaluated_at TIMESTAMPTZ,
    proposed_config_version_id  BIGINT REFERENCES config_versions(id),  -- the row the human would promote
    cooldown_until              DATE,
    source                      TEXT NOT NULL DEFAULT 'human',    -- human | routine | scout | agent
    notes                       TEXT,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE pipeline_events (
    id BIGSERIAL PRIMARY KEY, candidate_id BIGINT NOT NULL REFERENCES pipeline_candidates(id),
    ts TIMESTAMPTZ NOT NULL DEFAULT now(), from_stage TEXT, to_stage TEXT NOT NULL, actor TEXT NOT NULL, detail JSONB
);
ALTER TABLE books ADD CONSTRAINT books_candidate_fk FOREIGN KEY (candidate_id) REFERENCES pipeline_candidates(id);
```

`config_versions.status` keeps its meaning: a materialized candidate row is inserted as
`backtesting`, becomes `validated` when it passes the backtest gate (and stays `validated`
through the shadow trial), and is `rejected` on either failure. the human promotes a
`validated` row exactly as today (`UPDATE ... status='promoted', promoted_at=now()`), which is
what the trader's hot-reload watches.

## 2. backtest CLI (owner, before fan-out)

`backtest --date … --config-id N` loads `config_versions` row N instead of the latest promoted
row (everything else unchanged; `--tickers` still overrides the ticker list). this makes one
artifact — the materialized config row — the thing that is backtested, shadowed and promoted.

## 3. workstream A — multi-book trader (crate `data_feed` only)

**owns:** `crates/data_feed/**`. must not touch other crates, migrations, scripts or docs.

### 3.1 design

- new module `book.rs`:
  - `BookSpec { name, role: BookRole, config_version_id: Option<i64>, tickers: Option<Vec<String>>, capital: Option<f64>, purpose }`
    and `load_books(pool) -> Result<Vec<BookSpec>>` (enabled rows; error if there is not exactly one primary; `MAX_BOOKS` env, default 8, extra shadows skipped with an error log).
  - `Book { spec, follows_promoted: bool, config, config_id, pending, sessions: HashMap<ticker, LiveSession>, broker: Box<dyn Broker>, trade_writer, capital, daily_pnl, daily_pnl_date, last_gate, last_near_miss }`.
  - effective tickers = `spec.tickers` if set, else `config.tickers`. the config a book runs is the
    referenced row, or the latest promoted row when `config_version_id` is NULL (then the book
    follows promotions like the primary; the ticker override is re-applied to every new config).
  - `Book::on_bar(ticker, market: MarketState, shared) -> BarOutcome` does what the primary does
    today for one ticker: capacity check (per book), `session.on_tick`, broker submit/close on
    open/close events (undo on broker failure), daily P&L, trade write, block events, state row.
  - `Book::flatten_all(reason, last_prices)`, `Book::apply_pending_where_flat()`, `Book::state_row(ticker, shared)`.
- brokers: the primary's broker is chosen by `BROKER_MODE` exactly as today. **every shadow book
  gets its own `SimulatedBroker` regardless of `BROKER_MODE`** — enforced in the constructor, not
  by configuration. reconciliation at startup runs against the primary's broker only.
- `SimulatedBroker` gains per-ticker last prices: trait method `fn set_last_price_for(&self, ticker: &str, price: f64)` (default delegates to `set_last_price`); the simulated broker fills from the per-ticker price when it has one. main calls it for every book before the tick.
- `Runtime` keeps the shared, config-independent state (cross tracker, state builders, last bar
  / price / stale maps, tick count, process start) and `books: Vec<Book>`. per bar: build the
  `MarketState` once per ticker, then for each book that trades the ticker, clone it, set
  `market.cross = cross.context_for(ticker, &book_tickers, ts)` (**peers = that book's tickers**,
  matching a replay run with `--tickers`), and call `book.on_bar`.
- subscriptions = union of every enabled book's effective tickers + SPY; warm-up for the union.
- hot reload: `ConfigWatcher` polls promoted rows as today; on a new row, every book with
  `follows_promoted` gets it as pending (ticker override re-applied); tickers not yet subscribed
  still log "requires a restart" (the daily restart handles it).
- books changes (new/retired rows) take effect at the next start. at startup the trader logs
  the book roster and **deletes `engine_state` rows for (book, ticker) pairs it does not host**.
- `TradeWriter::new(pool, source, book)`; INSERT gains `book`. primary: source `paper`/`demo` as
  today; shadows: source `shadow`.
- `EngineStateRow` gains `book`; upsert `ON CONFLICT (book, ticker)`; `delete_engine_state(pool, book, ticker)`. block events gain `book`. `daily_pnl` in the row is the book's.
- shutdown / ctrl-c / SIGTERM / missed-close safety net: flatten every book (shadows are instant on the simulated broker). the safety net uses each book's own `force_exit_by` (books may differ).
- logging: a `book` field on every book-scoped log line. status line lists open positions per book.
- TUI (feature `tui`): wired to the primary only; must still compile with `--features tui`.
- a book whose engines fail to build is skipped with an `error!` and a state row is NOT written;
  the primary failing to build is fatal as today.

### 3.2 live-loop replay harness (second deliverable, after 3.1 compiles and passes)

`paper_trader --replay-bars <dir> --replay-date YYYY-MM-DD [--replay-speed max]`: instead of the
websocket, read `<dir>/<SYMBOL>.csv` (unix-second timestamp,open,high,low,close,volume; RTH only)
for every subscribed symbol, merge by timestamp, and push `BarEvent`s through the same channel as
fast as possible; force `BROKER_MODE=simulated` for the primary, disable wall-clock staleness and
the clock-based safety net (bar time drives everything), write trades with source `replay-live`,
exit when the bars are exhausted (flattening as at close). purpose: an end-to-end test of the
multi-book loop on real bars, and a **parity check** of the live loop against `backtest --date`
on the same day/config (trade count and P&L within a few dollars per trade; differences only from
fill modelling). warm-up in replay mode = the 8 calendar days before the replay date from the same
csvs.

### 3.3 tests / acceptance

- `cargo build --workspace`, `cargo test -p data_feed`, `cargo clippy -p data_feed -- -D warnings`, and `cargo build -p data_feed --features tui` all clean.
- unit: `load_books` roster validation (no primary / two primaries / disabled rows), effective
  tickers, follow-promoted re-applies the ticker override, `SimulatedBroker` per-ticker prices,
  trade writer + state row carry `book`, shadows always get a simulated broker.
- demo smoke: insert a shadow book (`INSERT INTO books VALUES ('demo-shadow','shadow',NULL,'{AAPL}',…)`), run `BROKER_MODE=simulated ./target/release/paper_trader --demo` for ~90 s, verify `engine_state` has rows for both books, then delete the demo book and its rows.
- replay parity: `--replay-bars data/bars_iex --replay-date 2025-04-04` with the primary only vs `backtest --date 2025-04-04 --bars-dir data/bars_iex --cross-index SPY --slippage-bps 3 --half-spread 0.005 --capital 10000 --sizing-fraction 0.30`: same entries (ticker, minute, direction) and exits within the fill model; report any difference and its cause. then the same day with a shadow book of a different config row and confirm the two books' trades carry their own `book`/`source`.
- keep a copy of the pre-change binary at `target/release/paper_trader.pre-books` before the first build (rollback path for Monday).
- do not commit; leave the tree building. write `docs/dev/books_trader_notes.md` (≤ 80 lines): what changed, how to add a book by SQL, replay-harness usage, parity result.

## 4. workstream B — pipeline runner (python, no rust)

**owns:** `scripts/pipeline/**`, `deploy/systemd/pipeline.service|timer`, `docs/pipeline/status.md` (generated). may read anything. must not edit crates, migrations, other scripts or docs.

### 4.1 CLI contract (`scripts/pipeline/pipeline.py`, python3 + stdlib + `scripts/psql.sh`)

```
pipeline.py propose --name NAME --kind ticker --ticker T [--gate default-ticker] [--notes ...] [--source human]
pipeline.py propose --name NAME --kind config --patch FILE.json [--tickers A,B] [--gate default-config|stress-mode] [--notes ...]
pipeline.py list [--stage S]              # table: id, name, kind, stage, gate, since, key numbers
pipeline.py show NAME                     # everything incl. events and results json
pipeline.py advance [--max-backtests N] [--dry-run]   # run every due transition (the nightly entry point)
pipeline.py backtest NAME | evaluate NAME | withdraw NAME [--reason] | retire-shadow NAME [--reason]
pipeline.py report [--markdown]           # regenerates docs/pipeline/status.md and prints it
```

### 4.2 transitions (all recorded in `pipeline_events`, all idempotent, all resumable)

1. **proposed → backtesting → backtest_passed | backtest_failed**
   - ticker kind: ensure `data/bars_iex/<T>.csv` covers 2022-01-01..yesterday (else `backtest --fetch-bars data/bars_iex --start 2022-01-01 --end <yesterday> --tickers T --feed iex`; **at most 2 new tickers fetched per run**, sequentially, to stay well inside the 200 req/min limit), then `BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh cand_<id> --tickers T --sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`.
   - config kind: **materialize** = base blob (the promoted row at proposal time, or `base_config_version_id`) + patch (`disable` → `enabled=false` on matching action instance_ids; `indicators`/`actions` → append; `session` → shallow-merge into `session`; optional `tickers`), insert into `config_versions` (`status='backtesting'`, `parent_version_id=base`, `created_by`/notes naming the candidate), store `materialized_config_id`; then the same sweep with `--config-id <row>`.
   - compute the metrics (reuse `research/entries/summarize.py` maths: 5y and per-year P&L, n, PF, win, DD, plus the baseline `iex_v18` numbers) and apply the named gate. store `backtest_result` json. on pass set the materialized row `validated`; on fail set it `rejected`, stage `backtest_failed`, `cooldown_until = today + 180 d`.
2. **backtest_passed → shadow**: insert `books` row `shadow:<name>` (ticker kind: `config_version_id NULL, tickers = {T}`; config kind: `config_version_id = materialized`), `candidate_id` set, `purpose` from the candidate; stage `shadow`, `shadow_started_at = now()`. log that it starts at the next trader start. ntfy info.
3. **shadow → shadow_passed | shadow_failed** once `sessions ≥ shadow_min_sessions` (sessions = distinct ET dates in `entry_block_events` or `trades` for the book) **and** `trades ≥ shadow_min_trades`, or after 60 sessions regardless:
   - parity: replay the same period from the IEX cache with the candidate's config (`--config-id` or `--tickers T`) day by day (the export already appends each day's bars for the primary's tickers; the pipeline fetches the period for any other ticker first), compare trade count (±30 %) and mean P&L per trade (within 10 bps of price); a parity failure is reported as **plumbing**, not edge, and does not fail the candidate — it flags it and stops the clock.
   - edge (gate-named): default: shadow PF ≥ 1.0 over the trial and P&L per trade not below the replay's by more than 10 bps; config kind additionally not below the primary's P&L over the same sessions by more than 20 %.
   - pass → stage `shadow_passed` then immediately **promotion_proposed**: ticker kind → insert a `validated` config row = promoted blob with T appended to `tickers` (`parent_version_id` = promoted); config kind → the materialized row is already `validated`. set `proposed_config_version_id`, append a proposal block to `docs/pipeline/status.md` and ntfy warning-level "promotion proposed: NAME → config row N".
   - fail → retire the book (`enabled=false, retired_at, retire_reason`), stage `shadow_failed`, cooldown 180 d.
4. **promotion_proposed → promoted**: detected when `proposed_config_version_id` has `status='promoted'`; retire the shadow book (the primary now trades it).
5. any stage → **withdrawn** by the human (`withdraw`).

### 4.3 gates (`scripts/pipeline/gates.py`, pure functions over the metrics dict; names are stable)

- `default-ticker`: 5y PF ≥ 1.3, ≥ 4 of 5 years positive, ≥ 100 trades over 5y, no year < −300 (36 % sizing), 2026 ≥ 0.
- `default-config`: 5y PF ≥ 1.3, ≥ 4 of 5 years positive, trades ≥ baseline's, P&L ≥ baseline's − 10 %, no year < −300.
- `stress-mode`: `default-config` plus, on days where SPY's session return < −1 %: P&L per day ≥ 2 × baseline's and no single day < −300. (metrics for this gate need per-day SPY returns from `data/bars_iex/SPY.csv`; compute them in `metrics.py`.)
- gates return `{pass: bool, checks: [{name, value, threshold, ok}]}` and that json is what `backtest_result` stores.

### 4.4 ops

- `deploy/systemd/pipeline.service` (+ `.timer` at 20:30 PT weekdays, after the research runner): `scripts/pipeline/pipeline.py advance --max-backtests 2 && pipeline.py report`, then `git add docs/pipeline/status.md data/*.args && git commit && git push` (same pattern as `scripts/research_runner.sh`).
- `docs/pipeline/status.md`: a table per stage plus the last 20 events; the routine and the human read this.
- tests: `python3 -m unittest discover scripts/pipeline/tests` for the materializer (all four patch keys), the gates (boundary cases), the session counter; an end-to-end dry run against the local DB: propose `bear-bounce-sma50` as a config candidate (patch = merge of `research/regime/regime_sma50.json` and `research/volume/rl_below_sma50.json`), run `backtest` for it (sweep ~4 min alone, longer if other sweeps are running), show the gate result; propose `ticker:AMD` and `ticker:META` (their IEX bars need fetching: do it, sequentially). leave these three candidates in whatever stage they earn; do not create shadow books for them until the owner says so (`advance --dry-run` prints what it would do). do not commit.

## 5. workstream C — stress mode study (research only)

**owns:** `research/stress/**`, sweep outputs under `data/` and `logs/sweeps/`. no code, no commits.

question: when SPY (or the name) is already down hard, v18 is blocked by the SPY-flat band and
the 11:30/11:55 clocks; on the 25 days since 2022 with SPY < −2 % it made $258 on 7 trades
(correlation of daily P&L with SPY −0.21). does a separate stress rule set capture those days
without diluting the ordinary book?

design (all cells = v18 unchanged **plus** a stress window pair; five-year IEX replay, honest
costs, 36 % sizing, `--cross-index SPY`):
- trigger: a second `cross_context` instance with `scale_pct 0.02` (score = SPY session return / 2 %, so −0.25 = −0.5 %, −0.5 = −1 %); optionally the name's own move via `pdl_5m.dist_close_pct` (percent vs prior close). cells over SPY ≤ −0.5 % / −1 % / −1.5 % and name ≤ −1.5 % / −2.5 %.
- stress windows: the two v18 short windows with the SPY band removed and the stress condition added, `entry_after 09:45`, `entry_before 15:30`, `exit_overrides {force_exit_by: "15:55", max_hold_ms: 7,200,000 | 10,800,000, score_exit_threshold: 0.6}`; VPIN kept vs dropped; also a variant that sizes up (indicator_tiered on the stress instance, ×1.5, `--max-position-pct 0.54`). the session clocks must be opened for the afternoon (see `docs/paper_trading_plan_2026-09.md` §13.2 and `research/eod/*.json` + `data/eod_*.args` for how the EOD study did it: promoted windows re-added with `entry_before 11:30` / `force_exit_by 11:55`, session flags widened).
- report: combined 5y/per-year vs `iex_v18`; **tail-day table**: P&L, trades and worst day on SPY < −1 % and < −2 % days, and on the 25 worst days by name; correlation of daily P&L with SPY for each cell; the stress window's own subset; costs of false alarms (stress days that reversed).
- accept-if (= gate `stress-mode`): combined PF ≥ 1.3, ≥ 4 of 5 years positive, tail-day P&L per day ≥ 2 × v18's, no single day < −300, ordinary-day book unchanged.
- deliverables: `research/stress/2026-09-26_stress_mode.md` (grid, tail tables, verdict, exact commands) and, if a cell passes, `research/stress/stress_mode_v1.patch.json` in the pipeline's patch format (`disable/indicators/actions/session`) ready for `pipeline.py propose --kind config --gate stress-mode`.

## 6. workstream D — alerts, reporting, docs

**owns:** `scripts/watchdog.sh`, `scripts/export_day.sh`, `docs/routines/eod-report.md`, `.claude/skills/*/SKILL.md`, `docs/pipeline.md`, `CLAUDE.md` (the wheel + repository map lines only), `deploy/systemd/install.sh` (register `pipeline.timer` by name). no rust, no commits.

- **abnormal-move alert** in `scripts/watchdog.sh` (runs every 5 min in-session, no LLM): one Alpaca snapshot request for the primary's tickers + SPY (`GET https://data.alpaca.markets/v2/stocks/snapshots?symbols=…&feed=iex`, keys from `.env`); alert **warning** when a name is ≥ 1.5 % below (or above) today's open or ≥ 2 % off its intraday high, **critical** when SPY is ≥ 1 % below its open or ≥ 1.5 % off its high; thresholds via env `MOVE_ALERT_NAME_PCT` / `MOVE_ALERT_SPY_PCT`; one alert per key per day per direction (existing state-file mechanism), message says what the primary book is doing on that name (position or not, from `engine_state` where `book='primary'`) and whether any shadow is in a trade there.
- watchdog: position / P&L / late-position checks filter `book='primary'`; heartbeat and staleness may use any row; new check: an enabled `books` row with no `engine_state` rows 10 min after the open → warning "book NAME not hosted (build failed or restart pending)".
- `scripts/export_day.sh`: fetch the day's IEX bars for the **union of enabled books' tickers** (not the hard-coded four) so trial tickers accumulate history; write `books.json` (per book: trades, P&L, open positions at close) and `shadow_trades.json` (source='shadow' trades of the day); keep every existing file unchanged.
- `docs/routines/eod-report.md`: a "books and pipeline" section: primary vs each shadow for the day and trial-to-date, plus `docs/pipeline/status.md` summarized; proposals may include "promote candidate NAME (row N)" but the routine still never edits configs.
- skills: `engine_state` queries add `WHERE book='primary'`; note shadows in the intraday-review checklist.
- `docs/pipeline.md`: operator guide — concepts (candidate, book, stage, gate), the CLI (as in §4.1), how to propose a ticker / a patch, what happens each night, how to read `status.md`, how to promote (the SQL), how to retire, failure modes (parity flag, build failure, restart pending), and the invariants (one primary; shadows never touch the broker; promotion is human).
- CLAUDE.md: add the `books` / pipeline lines to the wheel section and the file map (data_feed `book.rs`, `scripts/pipeline/`).

## 7. sequencing, ownership, safety

1. owner: this plan → reviewer agent → adjustments → migration applied (`sqlx migrate run`; the trader is down at the weekend) → `--config-id` in the backtest CLI → `cargo build --release` → commit "pipeline: schema + --config-id".
2. fan out A, B, C, D in parallel. exclusive file sets as above; **no agent commits**; the owner reviews and commits per workstream.
3. CPU: A compiles; C runs sweeps one at a time; B runs at most one sweep at a time; the machine has 12 cores.
4. Monday: the multi-book trader goes live only if A's parity check and demo smoke pass and the owner has reviewed the diff; otherwise `paper_trader.pre-books` is restored and the migration is harmless (defaults keep the old binary's writes valid — **A must confirm the old binary still runs against the migrated schema**: its `INSERT INTO engine_state` conflicts on `(book,ticker)` with the default, its trades default to `book='primary'`).
5. invariants that the reviewer should attack: exactly one enabled primary; a shadow can never reach the Alpaca broker; the primary's behaviour with zero shadow books is byte-for-byte today's; every existing `source='paper'` query is unaffected; a broken shadow config cannot take the process down; books changes never apply mid-session.

## 8. open questions for the reviewer

- is `entry_block_events` the right session counter for shadows, or should the trader write a tiny `book_sessions` row per book per day at first bar?
- should the pipeline runner also own the "same-day replay per book" (parity) or should `export_day.sh` do it for every enabled book nightly (cheaper to reason about, one place)?
- `MAX_BOOKS` = 8: enough? each book × ticker is one engine; cost is negligible but log volume and `engine_state` rows grow.
- ticker candidates follow the promoted config; is a config candidate that also changes tickers a legitimate single candidate (yes in the schema; the gate compares against the baseline on the same tickers?) — define.

## 9. review adjustments (2026-09-26 evening) — binding; where §9 conflicts with §1–§8, §9 wins

source: `docs/plans/2026-09-26_pipeline_and_books.review.md` (read it; every finding there is
accepted unless listed under "declined" below).

**owner-side, done before fan-out**
- `--config-id` is in (76e1dd4). `--patch-json` now also accepts `session` (shallow-merged; `force_exit_by` mirrored into every `session_close` action, same as the `--force-exit-by` flag) and `tickers` (replaces the list). `--fetch-bars` exits 3 when any chunk fails. `research_runner.sh` takes `logs/.research.lock` (flock, 4 h wait); the pipeline runner must take the same lock.
- migration adds: `agent_type` value `pipeline` (use it for `created_by`; put the candidate name in `mutation_reason`, which is NOT NULL), table `book_sessions (book, session_date, first_bar_at, config_version_id)` written by the trader on a book's first bar of each eastern date, and `pipeline_candidates.name` is unique only among *active* stages (re-proposal after cooldown works).
- rollback path (B1): the old binary's `ON CONFLICT (ticker)` upsert fails after the PK change. rollback = old binary **plus** `scripts/rollback_books.sql` (deletes non-primary rows from `engine_state`, restores `PRIMARY KEY (ticker)`), written by **A** and tested once against a scratch copy of the table. Monday's go/no-go decision happens before 06:10 PT.

**A (trader)**
- `apply_pending_to` must not remove the shared `state_builders` entry when a ticker leaves one book (other books / the cross tracker may still need it); drop a builder only when no book trades the ticker.
- demo feed emits the union of all books' tickers (+ SPY).
- `MAX_BOOKS` 8 and `MAX_SYMBOLS` 25: excess *shadows* are skipped with `error!` (the watchdog's "not hosted" alert makes it visible); the primary is never skipped.
- write `book_sessions` on each book's first bar of the eastern date (`INSERT … ON CONFLICT DO NOTHING`).
- replay harness (§3.2): no database writes at all — trades go to `--replay-out FILE` as the backtest's trade CSV format; the clock safety net, feed-staleness, config-reload and heartbeat arms are disabled; every timestamp that is `Utc::now()` today (exit stamps, `force_close`) uses the bar time in replay mode; the run ends by flattening at the last bar and exiting 0. parity command for the report: `backtest --date D --lookback-days 8 --bars-dir data/bars_iex --cross-index SPY --capital <INITIAL_CAPITAL> --slippage-bps 0 --half-spread 0` vs the harness with the simulated broker at 0 bps (compare entries/exits/sizes exactly; then rerun both with costs and expect only fill-model differences). no sizing override on either side (the blob's 0.30).
- shadows with a *different* ticker set pass **their own** ticker list to `cross.context_for` (peers), as planned.

**B (pipeline)**
- materializer mirrors the four `--patch-json` keys exactly (`disable`, `indicators`, `actions`, `session` with the `session_close` mirror, `tickers`); test: materialize the bear-bounce patch, run one day with `--config-id` and with `--patch-json`, trade rows identical.
- sessions for the shadow clock come from `book_sessions` (hosted days only); a book that is never hosted never times out — the watchdog reports it instead.
- parity replays use **no** sizing override (live and shadow size at the blob's fraction); gate sweeps stay at 36 % for comparability with the research record, and gate thresholds are stated at 36 %.
- verify bar coverage after any fetch (≥ 240 sessions per full year, ≥ 95 % of SPY's session count otherwise) before running a gate; refuse to gate on holes.
- take `logs/.research.lock` (flock) around every sweep; commit only `docs/pipeline/status.md` and `data/cand_*.args` (never `data/*.args`).
- gates: replace `default-config` with `volume-config` (trades ≥ baseline, PF ≥ 1.3, ≥ 4 of 5 years, P&L ≥ baseline − 10 %, no year < −300) and `quality-config` (PF ≥ baseline + 0.05, ≥ 4 of 5 years, trades ≥ 0.6 × baseline, no year < −300); `stress-mode` = `volume-config` relaxed to trades ≥ 0.9 × baseline **plus** on SPY < −1 % days: total P&L ≥ 3 × baseline's and ≥ +40 per such day on average, no single day < −300, and ordinary-day P&L within ±10 % of baseline. baseline for a candidate = the promoted row swept on the candidate's ticker set (`--tickers`). a ticker candidate's promotion-proposed row (promoted + T) is itself swept on the full set before it is proposed.
- a failed shadow is retired with `retire_reason`; a "not hosted after 3 trading days" state is reported, not failed.

**C (stress study) — exact recipe**
- run with the session opened by flags: `--no-new-entries-after 15:30 --force-exit-by 15:55` (as `data/e_eod_1530_t2.5.args` did). the patch must `disable` both promoted short windows and re-add them as the `_am` copies from `research/eod/eod_1530_t2.5.json` (`entry_before "11:30"`, `exit_overrides {force_exit_by: "11:55"}`; their conditions equal row 12's) so the ordinary book is unchanged, then add the stress windows. `pdl_5m` is not in v18: append it. the SPY trigger can read `cross_1m.index_session_ret` (metadata, in percent) directly — no second instance needed.
- shorts score-exit when `composite >= -threshold`: a stress window's `score_exit_threshold` must be **negative** (e.g. −0.6 = exit only when the composite has flipped to +0.6) or −10 to disable. `0.6` would exit every stress short on the next bar.
- for the candidate hand-off, write the patch with the `session` key (`{"no_new_entries_after": "15:30", "force_exit_by": "15:55"}`) instead of flags, and verify one day with `--patch-json` equals the flag run.

**D (ops/docs)** additionally owns `scripts/analysis/near_miss_replay.py` (filter `book='primary'`), `scripts/replay_book.sh <book> <date>` (replays one enabled book's config/tickers for a date on the IEX cache into `data/live/<date>/replay_<book>.csv`; `export_day.sh` calls it for every enabled book; the pipeline's parity step reads those files and calls the script for missing days), and the **repo hygiene** work in §10. cockpit queries are noted, not changed.

**declined**: refusing to start when there are too many books (a shadow must never stop the primary; skipping + alert is the invariant); making the old binary schema-compatible (rollback SQL instead).

## 10. repo hygiene (owner policy; D executes the first pass)

the working tree is small (~10 MB tracked) but `.git` is 253 MB because `agents-ts/node_modules`
and a `target2/` build tree were committed long ago (largest blobs 10–23 MB). that needs a history
rewrite + force push and is the human's call (single branch, single remote; `git filter-repo
--path agents-ts/node_modules --path target2 --invert-paths`). the rest is policy:

- **caches are never tracked**: `research/**/daily/`, `research/**/cache/`, `data/bars*`, `data/*.csv`, `data/labels/`. `research/swing/daily/*.csv` (5.3 MB, regenerable by `fetch_daily.py`) is untracked in the first pass; `research/entries/data/sec_*.json` (660 KB raw EDGAR; the derived `earnings_*.txt` stay) is deleted.
- **a closed research round keeps**: its report, its scripts, and the patch files of the *best / candidate* cells plus anything a later step consumes; every other grid patch is deleted, and the round's README lists the commit that still holds them. first pass: `research/entries/variants/` keeps `iex_*.json`, `grid_b0.4_v0.217.json`, `long_filters.json`, `long_unfiltered.json`; `research/volume/` keeps the md write-ups, `report_spec.json`, `build_report.py`, `walk_forward.*`, `smoke_*.json`, `rl_*.json` (pipeline seeds), `vp_w1950_p0.8.json`, `tier_l0.12_m0.67.json`, `rs_win_x0.3.json`, `rs_solo_x0.5.json` and the analysis scripts; the rest goes.
- **archives go**: `scripts/archive/` and `docs/archive/` (history keeps them); the plan doc's §-history is the record.
- **generated artifacts** (`docs/reports/*.html`, `docs/pipeline/status.md`) stay tracked because the human and the routine read them from the repo; one HTML per research round.
- `scripts/repo_report.sh` prints tracked size by directory, the 15 largest tracked files and untracked caches; the EOD routine runs it on the first trading day of each month and proposes deletions (never deletes).
- CLAUDE.md gets a five-line "repo hygiene" section with these rules.
