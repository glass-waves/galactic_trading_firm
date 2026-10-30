# research pipeline and shadow books — operator guide

candidate edges and candidate tickers move through one lane, like code through CI:
**proposed → backtest gate → shadow trial (live, same bars, simulated fills) → evaluation gate →
promotion proposal → human promotes.** the live trader hosts several *books*: the primary (latest
promoted config, real paper broker, unchanged behaviour) plus shadow books. a nightly runner
advances candidates. nothing in this lane promotes anything — the human does, with one SQL statement.

contract: `docs/plans/2026-09-26_pipeline_and_books.md` (§1 schema, §4 runner, §9 binding
adjustments) and its review. implementation notes: `scripts/pipeline/` (runner), `docs/dev/books_trader_notes.md` (trader).

## 1. concepts

| term | what it is | where |
|---|---|---|
| **candidate** | one idea in the lane. kind `ticker` (the promoted config on one more name) or `config` (the promoted blob plus a patch). carries its gate, stage, results, cooldown | `pipeline_candidates`, every transition in `pipeline_events` |
| **stage** | `proposed → backtesting → backtest_passed \| backtest_failed → shadow → shadow_passed \| shadow_failed → promotion_proposed → promoted`; `withdrawn` (human, from anywhere); `rejected` | `pipeline_candidates.stage` |
| **gate** | a named, pre-registered accept-if rule set over the five-year IEX replay at 36 % sizing (comparable with the research record). `default-ticker`, `volume-config`, `quality-config`, `stress-mode`, `additive-config`, `additive-ticker`, `sizing-config`, `standalone-strategy` | `scripts/pipeline/gates.py`; result json in `backtest_result` / `shadow_result` |
| **book** | one engine set the trader hosts. `primary` (role primary; exactly one enabled) or a shadow (role shadow; config row or ticker set; **always** a `SimulatedBroker`) | `books`; per-day hosting in `book_sessions` |
| **materialized config** | for a `config` candidate: base blob + patch inserted as a `config_versions` row (`status='backtesting'`, `created_by='pipeline'`, candidate name in `mutation_reason`). that one row is backtested, shadowed and promoted | `pipeline_candidates.materialized_config_id` |

shadows write `trades.source='shadow'` and `trades.book=<name>`, their own `engine_state`
(primary key `(book, ticker)`) and `entry_block_events` rows. every existing `source='paper'`
query still means "the primary".

## 2. the CLI (`scripts/pipeline/pipeline.py`, python3 + stdlib + `scripts/psql.sh`)

```
pipeline.py propose --name NAME --kind ticker --ticker T [--gate default-ticker] [--notes ...] [--source human]
                     [--cells-scanned N --study PATH --loyo stable|unstable|n/a]
pipeline.py propose --name NAME --kind config --patch FILE.json [--tickers A,B] [--gate default-config|stress-mode] [--notes ...]
                     [--cells-scanned N --study PATH --loyo stable|unstable|n/a]
pipeline.py list [--stage S]              # table: id, name, kind, stage, gate, since, key numbers
pipeline.py show NAME                     # everything incl. events and results json
pipeline.py advance [--max-backtests N] [--dry-run]   # run every due transition (the nightly entry point)
pipeline.py backtest NAME | evaluate NAME | withdraw NAME [--reason] | retire-shadow NAME [--reason]
pipeline.py regate NAME --gate GATE       # re-evaluate an existing backtest under a different gate (no new sweep)
pipeline.py set-provenance NAME [--cells-scanned N] [--study PATH] [--loyo stable|unstable|n/a]
                     # backfill/correct _provenance on an existing candidate; no regate, pass/fail unchanged
pipeline.py restat NAME                   # recompute backtest_result.metrics.stats from the existing sweep tag; no new sweep, no regate
pipeline.py report [--markdown]           # regenerates docs/pipeline/status.md and prints it
```

gate names after the review (§9): `default-config` was split into `volume-config` (trades ≥
baseline, PF ≥ 1.3, ≥ 4 of 5 years positive, P&L ≥ baseline − 10 %, no year < −300) and
`quality-config` (PF ≥ baseline + 0.05, ≥ 4 of 5 years, trades ≥ 0.6 × baseline, no year < −300);
`stress-mode` = `volume-config` relaxed to trades ≥ 0.9 × baseline plus the SPY < −1 % day tests;
`default-ticker` = 5y PF ≥ 1.3, ≥ 4 of 5 years positive, ≥ 100 trades, no year < −300, 2026 ≥ 0.
the baseline for a candidate is the promoted row swept on the **candidate's** ticker set.

`additive-config` is for a candidate that leaves every one of the baseline's trades untouched and
only *adds* a new window on top (a purely additive edge, e.g. a new entry window gated so it can
never overlap or crowd out an existing one). "additive" is decided empirically, not declared: the
candidate's trades are split by matching each one's `(date, ticker, entry_time, direction)` against
the baseline's trade set (`metrics.marginal_metrics()`) into `base` (present in the baseline) and
`added` (not). the gate requires `base` to reproduce the baseline's P&L and trade count almost
exactly (±2 % / ±1 %) and `added` to be a real, self-sufficient edge on its own (net positive,
PF ≥ 1.3, ≥ 15 trades over 5y, no year worse than −100, no day worse than −300). the years_positive
/ min_year_pnl checks the other config gates use are replaced by `combined_years_not_worse`, checked
*relative to the baseline's own year*, not an absolute floor: no year of the combined book may fall
more than 50 below what that year already was for the baseline. this matters because a baseline can
itself have flat or negative years (e.g. `iex_v18`: only 3 of 5 positive) that a strictly additive
candidate — one that cannot touch existing trades — has no way to repair; requiring `years_positive`
of the combined book would make such a candidate ungateable no matter how good its new window is.
`combined_years_not_worse` asks only that the new window not make a bad year worse, which is what an
additive candidate can actually promise.

`sizing-config` is for a candidate that changes *how big* a position gets, not *which* trades fire
(e.g. tiering `fixed_fractional` sizing on an indicator reading at entry). like `additive-config`
it uses `metrics.marginal_metrics()`/`split_marginal()` against the baseline's trade set, but the
bar is "the trade set barely moved" rather than "exactly unchanged plus a new edge": trade count
within ±2 % of the baseline's and the matched (`base`) subset ≥ 95 % of the baseline's trade count
(entries/exits can drift slightly — e.g. a size-dependent fill cost — but the set must still be
*this* strategy's trades, not a different one). on top of that: P&L ≥ baseline × 1.10 (sizing up
should show up as more P&L, not just more risk), PF ≥ baseline PF − 0.02, max drawdown no more than
30 % deeper (`dd ≥ baseline dd × 1.3`), every year ≥ that year's baseline − 50, and the largest
single position actually observed in the trades (`metrics.max_position_fraction()`: size ×
entry_price ÷ 10,000 capital) ≤ the candidate's own effective cap × 1.02 — its `_sweep_args`'
`--max-position-pct` if it has one, else the standard 0.36 (`gates.effective_max_position_pct()`),
with a 2 % tolerance for whole-share rounding and fill-price-vs-sizing-price drift. a `_sweep_args`
override therefore raises both the sweep's clamp and the gate's own cap check together, by design.

`standalone-strategy` is for a candidate that is a second, unrelated strategy — its own book, its
own tickers, often its own entry logic, not an addition to the live book's existing trades (e.g.
candidate #48 `qqq-noise-pm-vol`: QQQ only, afternoons, long and short, with the promoted short
windows disabled). the other config gates all compare the candidate against the promoted config
swept on the *candidate's own tickers* — meaningless here, since v18 on QQQ alone is a different,
barely-traded strategy, not a baseline for this one. like `additive-ticker`, the baseline is
forced to `iex_v18` (the live book's own record) regardless of the candidate's ticker set
(`baseline_for()`'s `gate_name in ("additive-ticker", "standalone-strategy")` branch). the
candidate is judged two ways at once: on its own 5y record (a higher bar than `default-ticker`,
since this is a brand-new book, not a single extra name on the existing one), and on what it adds
to the live book if run alongside it. `metrics.standalone_metrics()` builds a daily P&L series for
the candidate and for `iex_v18` over the union of SPY's sessions (`data/bars_iex/SPY.csv`,
zero-filled on days without trades), their Pearson correlation, the combined book (baseline rows +
candidate rows, a plain concatenation — exact, since the live replay runs every book independently)
with its drawdown computed on the combined *daily* series, active-day counts (days with ≥ 1
trade), and the candidate's PF with its single best year excluded (`ex_best_year_pf()` — guards
against a one-year wonder; the index-momentum study flagged that 55 % of #48's P&L is 2022, which
is exactly what this check is for). checks: `standalone_pf` ≥ 1.4; `standalone_years_positive`
≥ 4 of 5; `standalone_trades` ≥ 150 (5y); `standalone_worst_year` ≥ −300 (36 % sizing);
`daily_corr_with_baseline` ≤ 0.30 (Pearson on the daily series over all sessions); `combined_pf` ≥
baseline PF (not dilutive); `combined_max_dd` ≥ baseline max DD × 1.25 (no more than 25 % deeper,
computed on the combined daily series); `combined_active_days` ≥ baseline active days × 1.25.
the live 3-position cap is reported as info over the combined set
(`metrics.concurrency_exceedance()`) and never gated, same rationale as `additive-ticker`: the
replay runs each ticker set independently, so a straight concatenation is exact except for that
cap. a standalone candidate commonly needs a longer warm-up than the primary's 8 days (e.g. #48's
14-session lookback needs ≥ 22 calendar days): put `_sweep_args: "--lookback-days N"` in the patch
(`run_backtest()` appends it to the gate sweep's argv after the standard sizing args, and
`crates/backtest/src/main.rs`'s `get_arg()` resolves a repeated flag to its last occurrence, so it
overrides `backtest_cached_range.sh`'s own `--lookback-days 8`); the patch's `session.warmup_days`
is what the *live* trader reads for its own per-symbol warm-up once shadowed
(`crates/data_feed/src/book.rs`), independent of the gate sweep's `_sweep_args`.

`advance --dry-run` prints what it would do and writes nothing. `regate` reuses the tag's sweep CSVs
and, for a config candidate, the already-materialized `config_versions` row — no new sweep and no
re-materialization — so it is cheap to try a different gate against a backtest already on disk; it
records a `regate` event with the old and new gate names and is only allowed from `backtest_passed`
or `backtest_failed` (a pass clears any cooldown and moves the materialized row to `validated`; a
fail leaves cooldown as it was). run everything from the repo root.

## 3. proposing

**a ticker** — `scripts/pipeline/pipeline.py propose --name ticker:AMD --kind ticker --ticker AMD --notes "why"`.
the runner fetches the IEX bars it lacks (at most two new tickers per night, 2022-01-01..yesterday,
coverage verified before any gate), sweeps the promoted config on `[AMD]`, applies `default-ticker`.

**a patch (config candidate)** — write a patch file in the `--patch-json` format the backtest
CLI already understands (the materializer mirrors it exactly, so a sweep with `--patch-json`
and a replay with `--config-id <materialized row>` produce the same trades):

```json
{
  "disable":    ["window_5m_thrust_short"],            // action instance_ids set enabled=false
  "indicators": [{ "instance_id": "pdl_5m", "...": "..." }],   // appended (or replaced by instance_id)
  "actions":    [{ "instance_id": "window_stress_short", "...": "..." }],
  "session":    { "no_new_entries_after": "15:30", "force_exit_by": "15:55" },  // shallow-merged; force_exit_by is mirrored into every session_close action
  "tickers":    ["AMZN", "AAPL", "NVDA", "MSFT"],       // optional: replaces the list
  "_sweep_args": "--max-position-pct 0.45"              // optional: see below
}
```

then `pipeline.py propose --name bear-bounce-sma50 --kind config --patch <that file>.json --gate volume-config`.
`--tickers` on a config candidate changes the set it trades; its baseline is then the promoted
row on that set, and its promotion row is swept as a whole before it is proposed.
seeds already in the repo: `research/volume/rl_*.json` (long regime cells), `research/regime/*.json`,
`research/stress/stress_mode_v1.patch.json` when the stress study delivers one.

**`_sweep_args` (optional, string)** — a meta key, not a config-blob key: it never reaches
`materialize.apply_patch()` (which only reads the five keys above — the backtest CLI's
`--patch-json` does the same, so a materialized row and a `--patch-json` replay stay identical)
and the materializer never writes it into the `config_versions` blob. the runner (`run_backtest()`
in `pipeline.py`) reads it off the candidate's stored patch and appends its tokens to the gate
sweep's argv *after* the standard `GATE_SWEEP_ARGS` and the candidate's own `--config-id`/`--tickers`
— every flag the gate sweep always passes is still passed, this just adds more after it. that
ordering matters because `get_arg()` in `crates/backtest/src/main.rs` resolves a flag that appears
twice to its **last** occurrence (`args.iter().rposition(...)`), so e.g. `_sweep_args:
"--max-position-pct 0.45"` overrides the standard sweep's `--max-position-pct 0.36` instead of
being shadowed by it. this is for a candidate whose whole point is a cap the standard sweep would
otherwise clamp away (e.g. a sizing tier that only pays off above the research sizing's own
position cap) — most candidates have no need for it. the resolved argv (including the override)
is recorded in `backtest_result.sweep.args` and also separately as
`backtest_result.sweep_args_override`, so `pipeline.py show NAME` always shows what actually ran.

**config candidates jump the queue ahead of ticker candidates.** `pipeline.py advance` builds its
nightly backtest queue from the `proposed`/`backtesting` candidates ordered by id — except within
each of those two groups, `kind == 'config'` candidates sort before `kind == 'ticker'` candidates
(ties keep id order). this is a code-level ordering choice in `cmd_advance()`, not a schema change:
there is no `priority` column and no migration, because a config edge (a decided, specific idea) is
cheaper to resolve and more valuable to get an answer on than the next name in a 20-ticker screen
batch, and the existing `notes`/`gate` columns are not suited to carrying an ordering key. a config
candidate proposed today is swept before any ticker candidate already queued, regardless of id.

rules: one idea per candidate; the gate is chosen at proposal time and not changed afterwards;
a name can be re-proposed only after its 180-day cooldown; anything that needs new engine code is
not a candidate — it is a PR.

## 4. what happens each night (PT)

| when | who | what |
|---|---|---|
| 13:10 | `paper-trader-stop.timer` | trader flattens every book and stops |
| 13:20 | `export-day.timer` → `scripts/export_day.sh` | fetches the day's IEX bars for the union of enabled books' tickers (+ SPY); writes `data/live/<date>/` incl. `books.json`, `shadow_trades.json` and one `replay_<book>.csv` per enabled book (`scripts/replay_book.sh`, no sizing override); commits, pushes |
| 13:35 | cloud EOD routine | report + briefing + research queue; a "books and pipeline" section compares primary vs each shadow and summarizes `docs/pipeline/status.md`; may *propose* "promote candidate NAME (config row N)" |
| 20:00 | `research-runner.timer` | approved research-queue entries (holds `logs/.research.lock`) |
| 20:30 | `pipeline.timer` → `pipeline.py advance --max-backtests 2 && pipeline.py report` | every due transition, under the same lock; commits only `docs/pipeline/status.md` and `data/cand_*.args` |
| 06:10 next day | `paper-trader.timer` | trader starts, reads `books`, subscribes the union of tickers, deletes `engine_state` rows for pairs it does not host, writes `book_sessions` on each book's first bar |
| 06:15 | `preopen-check` | verifies the primary; reports shadows not hosted as warnings |

what `advance` does per stage:

1. **proposed → backtesting → backtest_passed | backtest_failed.** ticker kind: ensure bars, sweep
   `cand_<id>` at 36 % sizing with `--tickers T --cross-index SPY`. config kind: materialize the
   row, sweep with `--config-id <row>`. compute the metrics (5y and per-year P&L, n, PF, win, DD,
   plus the baseline), apply the gate, store the json. pass → row `validated`; fail → row
   `rejected`, `cooldown_until = today + 180 d`.
2. **backtest_passed → shadow.** insert `books` row `shadow:<name>` (ticker kind: follows the
   promoted config with `tickers = {T}`; config kind: `config_version_id = materialized`),
   `candidate_id` set, `shadow_started_at = now()`. **the book starts at the next trader start**
   (books never change mid-session). ntfy info.
3. **shadow → shadow_passed | shadow_failed** once `book_sessions ≥ shadow_min_sessions` (20) and
   trades ≥ `shadow_min_trades` (15), or after 60 hosted sessions. parity first: the trial period
   replayed from `data/live/*/replay_<book>.csv` (missing days backfilled with `replay_book.sh`),
   trade count within ±30 % and mean P&L per trade within 10 bps — a parity failure is
   **plumbing**: it flags the candidate and stops its clock, it does not fail it. then the edge
   gate: shadow PF ≥ 1.0 and P&L per trade not below the replay's by more than 10 bps; a config
   candidate also not below the primary's P&L over the same sessions by more than 20 %. after 60
   sessions with too few trades: `insufficient_activity`, not a verdict.
   pass → `shadow_passed` → **promotion_proposed**: the row the human would promote is set in
   `proposed_config_version_id` (ticker kind: a new `validated` row = promoted blob + T, swept on
   the full set first; config kind: the materialized row), a proposal block is appended to
   `status.md`, ntfy warning "promotion proposed: NAME → config row N".
   fail → the book is retired with `retire_reason`, `shadow_failed`, cooldown 180 d.
4. **promotion_proposed → promoted** is *detected*, never done: when `proposed_config_version_id`
   has `status='promoted'` the runner retires the shadow (the primary now trades it).

## 5. reading `docs/pipeline/status.md`

one table per stage (id, name, kind, gate, since, the key numbers: 5y P&L / PF / trades and the
baseline's; for shadows: hosted sessions, trades, P&L, replay P&L, parity flag), then the last 20
events, then any open **proposal blocks** — each names the candidate, the config row to promote,
the evidence (backtest gate json, shadow vs replay vs primary over the same sessions) and the exact
SQL. sizing note printed in the header: gate sweeps are at 36 % sizing; live, shadow and parity
replays at the blob's fraction (0.30 today), so dollar figures are not comparable across the two
without scaling — per-trade bps and PF are.

## 6. promoting (human only)

```sql
-- the row named in the proposal block; it is 'validated' and has parent_version_id set
UPDATE config_versions SET status = 'superseded' WHERE status = 'promoted';
UPDATE config_versions SET status = 'promoted', promoted_at = now() WHERE id = N;
```

what follows: the trader's `ConfigWatcher` sees the new row within its poll interval (it only
sees rows with an id **higher** than the running row — a materialized/proposed row always is),
every follow-promoted book takes it as `pending` and swaps engines **per ticker as each goes
flat** (`engine_state.pending_config_version_id` shows the wait). a ticker the process is not
subscribed to yet is logged as "requires a restart" and joins at the **next daily start** (06:10 PT);
if it must trade today, restart the service by hand once flat. the pipeline retires the shadow
at its next `advance`. if the promotion turns out wrong: insert a copy of the previous blob as a
new promoted row (`scripts/update_config.sh`), never flip an old row back — the watcher would
not see it.

## 7. retiring and withdrawing

- `pipeline.py retire-shadow NAME --reason "…"` (or SQL: `UPDATE books SET enabled=false,
  retired_at=now(), retire_reason='…' WHERE name='shadow:NAME'`) — the book stops at the next
  trader start; the trader deletes its `engine_state` rows then; its trades and `book_sessions`
  stay for the record.
- `pipeline.py withdraw NAME --reason "…"` — the candidate leaves the lane from any stage (and
  its shadow is retired). withdrawing sets no cooldown.
- a manual shadow (no candidate), e.g. to watch a config row live:
  `INSERT INTO books (name, role, config_version_id, tickers, purpose) VALUES ('shadow:row-14', 'shadow', 14, NULL, 'watch row 14 live');`
  — it appears at the next start; retire it the same way. at most `MAX_BOOKS` (12 since 2026-10-23, set in `paper-trader.service`; the pipeline warns when a new shadow would exceed it) books and
  `MAX_SYMBOLS` (25) websocket symbols; excess shadows are skipped with an error and the
  watchdog's "not hosted" warning, the primary is never skipped.

## 8. failure modes

| symptom | meaning | what to do |
|---|---|---|
| watchdog "book NAME not hosted (build failed or restart pending)" after 09:40 ET | the books row is enabled but the running trader has no engines for it: created after today's start (normal on the first day), its config row failed to build, or it was skipped for the symbol cap | nothing on day one; otherwise read the start-up log (`journalctl --user -u paper-trader.service | grep book=NAME`). the runner reports "not hosted after 3 trading days" and stops the clock; it never fails the candidate for this |
| `status.md` shows `parity: FLAG` on a shadow | live trades and the same-day replay disagree beyond ±30 % count / 10 bps per trade | plumbing, not edge: compare `shadow_trades.json` with `replay_<book>.csv` for the day; typical causes: warm-up depth, a ticker not yet subscribed, a mid-day promotion, the concurrency cap. the clock is stopped until the flag is cleared (`evaluate NAME` re-runs it) |
| candidate stuck in `backtesting` | a sweep died or is waiting on `logs/.research.lock` | `pipeline.py advance` resumes it (tags are deterministic); check `logs/sweeps/` |
| `backtest_failed` / `shadow_failed` with `cooldown_until` | the gate said no | read the checks json in `show NAME`; re-propose only after the cooldown, with a changed idea |
| `insufficient_activity` | 60 hosted sessions and still < min trades | the candidate trades too rarely to judge; withdraw or widen the idea |
| "refusing to gate: bar coverage" | a fetched ticker has holes (< 240 sessions in a full year or < 95 % of SPY's) | `backtest --fetch-bars data/bars_iex --start … --end … --tickers T --feed iex` for the gap, then `backtest NAME` |
| a shadow's `engine_state` rows stop updating while the primary's do not | that book's engine panicked or its ticker feed is stale | warning only; the primary is isolated. journal, then retire if it repeats |
| `pending_config_version_id` set all day on the primary | a ticker never went flat after a promotion | expected; it swaps at the next entry or at the daily restart |

## 9. invariants

1. exactly one enabled primary (`books_one_enabled_primary`), named `primary`.
2. a shadow can never reach the Alpaca broker: shadows construct their own `SimulatedBroker`
   regardless of `BROKER_MODE`; reconciliation at start-up is against the primary's broker only.
3. the primary with zero shadow books behaves exactly as before books existed; every
   `source='paper'` query keeps its meaning.
4. a broken shadow config cannot take the process down: it is skipped with an error, no state
   row is written, the watchdog says so.
5. books changes (new, retired) never apply mid-session; they take effect at the next start.
   config promotions hot-reload per ticker when flat, as always.
6. promotion is human. the runner inserts `backtesting`/`validated`/`rejected` rows and proposes;
   the EOD routine proposes; only the `UPDATE … status='promoted'` above promotes.
7. gates are pre-registered and named; the json of every check is stored with the candidate.
8. the research lock (`logs/.research.lock`) serializes sweeps: research runner and pipeline never overlap.

- **fill timing, backtest vs live.** the backtest fills at the NEXT bar's open (anti-lookahead,
  `crates/backtest/src/replay.rs`); the trader and the shadow books fill at the signal bar's close
  (the broker's market fill, or the simulated price ± 5 bps). the decision bar is the same, so "entry
  within one bar" is the parity rule and a one-minute price difference between `replay_<book>.csv`
  and the live trade is not a plumbing fault (audit 2026-10-23).

## 10. where books show up in ops

- **watchdog** (`scripts/watchdog.sh`, every 5 min, no LLM): position / P&L / late-position
  checks read `book='primary'`; heartbeat and staleness use any row; "not hosted" warning per
  enabled shadow after 09:40 ET; abnormal-move alert (one Alpaca snapshot per run for the primary's
  tickers + SPY; `MOVE_ALERT_NAME_PCT` 1.5 / `MOVE_ALERT_SPY_PCT` 1.0) whose message says what the
  primary holds on that name and whether any shadow is in a trade there.
- **export** (`data/live/<date>/`): `books.json` (per book: day trades / P&L / open positions at
  close, sessions and trades to date), `shadow_trades.json`, `replay_<book>.csv`, `replay_books.log`.
  the per-ticker files (`engine_state.json`, `block_events.json`, `near_miss_replay.txt`) are the primary's.
- **skills**: `preopen-check` and `intraday-review` query `engine_state WHERE book='primary'`
  and list shadows separately; a shadow's problem is a WARNING, never CRITICAL, and never a reason
  to tune anything. `eod-review` compares the primary to its replay as before.
- **cockpit**: its trade queries have no `source`/`book` filter yet — dashboard totals include
  shadow trades until that is fixed (noted, not changed).

## 11. statistical honesty (point estimates are best-of-N)

a candidate is judged on PF / P&L from ~300–500 trades, usually chosen as the best cell of a
research grid (`research/<dir>/make_patches.py` sweeps a dozen-plus variants; the report proposes
the one that clears the owner's bar). that point estimate is biased upward by the search that
picked it, and the gates (§2, `gates.py`) never changed to account for it — they still compare
flat numbers against fixed thresholds, deliberately. `metrics.py` instead adds a second, purely
descriptive layer that sits next to the point estimate without touching any gate:

- **`metrics.bootstrap_pf(trades, n=2000, seed=0)` / `bootstrap_pnl(trades, n=2000, seed=0)`** — the
  5th/95th percentile of PF (resp. total P&L) over `n` trade-level bootstrap resamples: each
  resample draws `len(trades)` trade P&Ls WITH replacement from the trades' own P&Ls (the standard
  bootstrap). seeded with python's own `random` (this package is stdlib only, no numpy) so the CI is
  reproducible across runs and in tests.
- **`metrics.sign_flip_p(candidate_daily_series, n=5000, seed=0)`** — the p-value that a zero-edge
  version of the candidate (every active day keeps its P&L size and gets a random sign) reaches
  total P&L ≥ the observed one. it replaced a random-session draw on 2026-10-29: that test compared
  the candidate with a shrunken copy of itself and gave p < 0.05 to 13 % of zero-edge strategies
  (5.5 % got p < 0.01); the sign-flip null keeps the candidate's own day sizes and clustering and is
  calibrated at the day level. still a single-test number: it knows nothing about how many cells
  were searched before this one.
- **`metrics.ex_best_year_pf(trades)`** — PF with the single best year removed (the standalone
  gate's one-year-wonder guard), now reported for every candidate because the record's edge is
  concentrated: iex_v18 is PF 1.40 with 2022 (59 % of the P&L) and 1.19 without it.
- all three are folded into every `metrics.full_metrics()` call automatically, under
  `metrics['stats'] = {"pf_ci90": [lo, hi], "pnl_ci90": [lo, hi], "signflip_p": p, "ex_best_year": {...}, "n_trades": n}`
  (`None` entries when there are no trades — nothing to test).

**what the numbers can and cannot say (statistics audit, 2026-10-29; scripts in that session's
scratchpad, method: day-block sign flips over the 218 complete five-year cells on disk).** ≥ ~2,100
configurations have been evaluated on the same 2022–2026 data (370 engine cells, ≥ 594 pretest
points, 1,152 index-momentum cells); the effective number of independent tests among the 218
engine cells is 44–134. the best of those 218 under a joint zero-edge null reaches PF 1.25 (median),
1.42 (90th pct), 1.48 (95th): volume-config's PF ≥ 1.3 floor is beaten by luck one time in three,
and quality-config's "+0.05 PF" is 0.27 standard deviations of a 450-trade cell. the family is not
null as a whole (16 of 218 cells beat the null's 95th percentile) — the problem is that the
*increments* being promoted are noise-sized. the shadow stage cannot tell PF 1.4 from PF 1.0 either:
at 15 / 24 / 60 trades the "shadow PF ≥ 1.0" check passes a true-1.4 book 67 / 72 / 82 % of the time
and a true-1.0 book 49 % — separating them at 5 % / 80 % power needs ≈ 450 trades. read a shadow
verdict as a plumbing and parity result (those are paired and informative), not as an edge verdict.
live: at 0.4 trades a day, live PF < 1.0 becomes 5 %-level evidence against the backtest only after
≈ 200 trades (≈ 500–600 sessions). proposed gate changes (human decision, not applied): PF ≥
max(1.4, baseline − 0.02) and ex-best-year PF reported next to it; drawdown ≤ 1.25 × baseline and
$/trade ≥ 0.85 × baseline on volume-config; an "improvement" over its parent only when the paired
day-bootstrap lower 90 % bound of the P&L difference is > 0 at a level adjusted for
`cells_scanned`; added-trade minimums of 60–100 (the 15 / 40 floors admit about a third of
zero-edge additions); a single-name share report.

**provenance.** a candidate's patch json may carry an optional `_provenance` meta key (ignored by
the materializer exactly like `_sweep_args` — never reaches the config blob):

```json
"_provenance": {"cells_scanned": 20, "study": "research/trigger/2026-10-02_entry_trigger_study.md", "loyo": "stable"}
```

`cells_scanned` is how many cells the study swept before picking this one (the size of the best-of-N
search — the thing that biases the point estimate); `study` is the report that did the picking;
`loyo` is that study's own leave-one-year-out verdict (`stable` | `unstable` | `n/a` when none was
run). set it at propose time (`pipeline.py propose ... --cells-scanned 20 --study research/trigger/2026-10-02_entry_trigger_study.md --loyo stable`)
or backfill/correct it later with `pipeline.py set-provenance NAME --cells-scanned N --study PATH
--loyo stable|unstable|n/a` (no re-sweep, no regate — this only edits the stored patch's meta key).
a ticker candidate, or any candidate proposed by hand rather than out of a grid study, simply has
no `_provenance` — `cells_scanned` is never required and nothing defaults it to 1.

**where it shows up.** `pipeline.py show NAME` prints a `provenance` block (the three fields plus
the current `pf_ci90`/`pnl_ci90`/`signflip_p`/ex-best-year PF, and — only when `cells_scanned > 1` — a
`selection_discount` sentence spelling out that the headline PF is biased high and naming the CI's
low end). `pipeline.py report` / `docs/pipeline/status.md`'s per-candidate line appends the same
numbers right after the PF figure: `PF 1.46 [1.31–1.63] · sign-flip p 0.02 · ex-22 PF 1.27 · best of 20 cells`. none of this
changes `gate_result.pass` — the gate still runs on the flat point estimate, exactly as before this
section existed.

**`pipeline.py restat NAME`** recomputes just `backtest_result.metrics.stats` from the candidate's
already-complete sweep tag (`metrics.sweep_files_present()`) and rewrites `docs/pipeline/status.md`
— no new sweep, no re-materialization, no regate, pass/fail untouched. it is the way to add stats
to a candidate backtested before this section existed (the shadow candidates at the time this was
written: #4 `vpin-0.26`, #6 `stress-s15-core`, #28 `size-vpin26-x1.25`, #29 `thrust-1h15`, #44
`spy-sqrt-band`, #45 `spy-sqrt-band-1h15`, #48 `qqq-noise-pm-vol`). if another run is mid-rewrite of
that tag's sweep CSVs (a missing year), `restat` skips and says so rather than compute stats on a
trade set that is about to change out from under it — the research lock (§9.8) serializes actual
*sweeps*, but `restat` reads finished CSVs off disk without taking it, so this check is its own
guard against a half-written tag.
