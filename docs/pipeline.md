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
| **gate** | a named, pre-registered accept-if rule set over the five-year IEX replay at 36 % sizing (comparable with the research record). `default-ticker`, `volume-config`, `quality-config`, `stress-mode`, `additive-config`, `sizing-config` | `scripts/pipeline/gates.py`; result json in `backtest_result` / `shadow_result` |
| **book** | one engine set the trader hosts. `primary` (role primary; exactly one enabled) or a shadow (role shadow; config row or ticker set; **always** a `SimulatedBroker`) | `books`; per-day hosting in `book_sessions` |
| **materialized config** | for a `config` candidate: base blob + patch inserted as a `config_versions` row (`status='backtesting'`, `created_by='pipeline'`, candidate name in `mutation_reason`). that one row is backtested, shadowed and promoted | `pipeline_candidates.materialized_config_id` |

shadows write `trades.source='shadow'` and `trades.book=<name>`, their own `engine_state`
(primary key `(book, ticker)`) and `entry_block_events` rows. every existing `source='paper'`
query still means "the primary".

## 2. the CLI (`scripts/pipeline/pipeline.py`, python3 + stdlib + `scripts/psql.sh`)

```
pipeline.py propose --name NAME --kind ticker --ticker T [--gate default-ticker] [--notes ...] [--source human]
pipeline.py propose --name NAME --kind config --patch FILE.json [--tickers A,B] [--gate default-config|stress-mode] [--notes ...]
pipeline.py list [--stage S]              # table: id, name, kind, stage, gate, since, key numbers
pipeline.py show NAME                     # everything incl. events and results json
pipeline.py advance [--max-backtests N] [--dry-run]   # run every due transition (the nightly entry point)
pipeline.py backtest NAME | evaluate NAME | withdraw NAME [--reason] | retire-shadow NAME [--reason]
pipeline.py regate NAME --gate GATE       # re-evaluate an existing backtest under a different gate (no new sweep)
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
entry_price ÷ 10,000 capital) ≤ 0.45 — a hard risk cap independent of whatever `--max-position-pct`
the sweep ran at, so a `_sweep_args` override (above) can raise the sweep's clamp without raising
the gate's.

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
  — it appears at the next start; retire it the same way. at most `MAX_BOOKS` (8) books and
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
