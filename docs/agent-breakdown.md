# agent breakdown — keeping the tuning loop closed

for an agent that keeps the research → backtest → shadow → promotion loop running. this is the
"how it fits together and where it breaks" view; `CLAUDE.md` is the file map, `docs/pipeline.md` the
pipeline operator guide, `research/lab/PLAN.md` the lab's statistical contract. **last updated
2026-11-27** (promoted row 25). when any of the facts below change, change them here in the same
commit — see §8.

---

## 1. what the system is, in one paragraph

a rust intraday engine (`crates/`) trades four mega-caps short-only on an alpaca paper account from a
desktop, 06:10–13:10 PT on weekdays, with every decision keyed to one immutable `config_versions`
row (the promoted row; today **row 25**, `lab-na-bundle`). nothing with an LLM in it places, cancels or
flattens an order. AI tunes the *config*: through the pipeline lane, which ends in a human
`UPDATE … status='promoted'`, or — one guardrailed exception — the eod-review check-in's
`scripts/update_config.sh` (one validated parameter for the next session; allow-listed, never yet used).
the loop is: idea → patch over the promoted blob → multi-year deterministic replay against the
promoted row's own replay → a shadow book trading the candidate live with simulated fills → a parity
check → a written promotion proposal → a human. every stage leaves a row in postgres and a commit.

## 2. engine architecture and how AI tunes it

### 2.1 the engine (what a config *is*)

- `StrategyConfig` (`crates/types/src/config.rs`) = tickers + indicator instances + action instances +
  scoring config + `session` (clocks in US/Eastern: `no_new_entries_after`, `force_exit_by`,
  `max_position_pct`, `warmup_days`) + `ticker_overrides`. it is stored as the JSONB `config_blob` of a
  `config_versions` row. **trades, hot-reload and books are keyed on the row id**, never on the blob's
  `config_id`.
- indicators (`crates/indicators`) turn a `MarketState` (1m/5m/1h candle windows, VWAP, bid/ask, cross
  feed) into scores in [−1, 1]; actions (`crates/actions`) turn scores + position into entry / exit /
  monitor / sizing signals. the tick loop (`crates/engine/src/tick_loop.rs`) runs indicators under
  `catch_unwind`, aggregates per timescale, computes the composite, evaluates actions in phase order.
- row 25's edge is not the composite: it is **entry windows** (`window_*` actions) — four short windows
  gated on 5m/1h scores, a SPY sqrt-time band (`cross_1m.index_session_ret_per_sqrt_min`), VPIN ≥ 0.20,
  clocks (am to 10:00, pm 10:00–11:00, flat 11:55) and a **stress window** that only arms when SPY is
  ≤ −1.5 % on the session (entries 09:45–15:30, flat 15:55). sizing is `sizing_tiered_vpin`: 0.30 of
  capital, ×1.25 at VPIN ≥ 0.26, hard clamp `max_position_pct` 0.40, 3 positions. capital is
  `min(INITIAL_CAPITAL=10 000, account equity)`.
- the live binary is `paper_trader` (`crates/data_feed`): alpaca IEX websocket → 1m bars →
  `CandleAggregator` (5m, 1h) → `MarketStateBuilder` → one `MarketState` per ticker handed to **every
  book** that trades it (§4). the primary uses `AlpacaBroker`; every shadow uses `SimulatedBroker`.
  warm-up at start: per symbol, max over the books trading it of `session.warmup_days` or 8 calendar
  days of SIP history (so a shadow with a longer warm-up would change the primary's hourly state —
  keep shadows at 8 or give the engine per-book warm-up first).

### 2.2 what "AI tunes it" means here

| who | may do | may never do |
|---|---|---|
| **pipeline** (`scripts/pipeline/pipeline.py`, nightly 20:30 PT, no LLM) | materialize a patch as a `config_versions` row (`status='backtesting'`), run the gate sweep, insert `books` rows for shadows, write `validated` / `rejected`, append a *promotion proposal* to `docs/pipeline/status.md` | set `status='promoted'`; change the primary's tickers; touch crates |
| **lab** (`scripts/lab/`, run by agents in a session) | replay specs over fixed windows, judge them against pre-registered tests, write reports under `research/lab/` | reveal the holdout outside the protocol (`LAB_REVEAL=1` is a decision, write it down) |
| **claude check-ins** (`preopen-check` 06:15, `eod-review` 13:30, `ideate` Sun 18:00; `claude -p` under systemd) | read state, write `agent_memos`, notify, propose. **eod-review alone has tuning authority** (skill §4): one parameter, validated on the last 20 sessions, applied for the next session with `scripts/update_config.sh`, which inserts a *new promoted row* — the one automated path to a promotion. never exercised to date (no `created_by='claude_eod'` rows) | run the trader, place or flatten orders, edit crates, `git push`, promote through the pipeline lane |
| **cloud EOD routine** (`docs/routines/eod-report.md`, 13:35 PT; **silent since 2026-10-20**) | write `docs/reports/<date>.md`, `docs/briefings/<next>.md`, `docs/research_queue.md` | anything else |
| **human** | promote, roll back, withdraw, decide holdout reveals, change gates, change timers / units / settings.json | — |

the chain of custody for one tuning change: patch json → `pipeline.py propose` (row in
`pipeline_candidates`, gate named up front, `cells_scanned` declared) → nightly sweep → `books` row →
20+ sessions / 15+ trades → `status.md` proposal → human SQL. there is no path where an agent's output
reaches the live engine without the human SQL — except the eod-review path above, which the human can
close by removing `update_config.sh` from the check-in allow-list and skill §4.

## 3. backtesting and determinism

### 3.1 what a backtest is

`target/release/backtest` (`crates/backtest`) replays one session per invocation:
`--date D --lookback-days 8 --bars-dir data/bars_iex --cross-index SPY --cross-lag 1 --config-id N
[--patch-json P]… [--tickers …] --sizing-fraction 0.36 --max-position-pct 0.36|0.45`. it warms the
engine with the lookback days, feeds the day bar by bar through the **same** `TradingEngine` the trader
uses, and fills at the **next bar's open** (anti-lookahead) with the house cost model (3 bps slippage +
$0.005 half spread; `--no-costs` is the explicit frictionless run). the trader and the simulated shadow
broker fill at the signal bar's close ± 5 bps instead; the *decision* bar is the same, so "entry within
one bar, same window, same exit reason" is the parity rule, and a one-minute price difference is not a
fault.

five-year sweeps (`scripts/run_cached_sweep.sh`, the pipeline's `cand_<id>` / `live_<id>` tags) and the
lab (`scripts/lab/lab.py`) are just many single-day runs in parallel (slot semaphore in `/tmp/lab_slots`,
`nice 19`), each cached.

### 3.2 where determinism comes from (and where it can leak)

- **no randomness in the engine or the replay.** no RNG in `crates/engine` or `crates/backtest`;
  indicator order within a timescale is weighted-sum, so `HashMap` iteration order does not change a
  score. the same binary + same bars + same config row + same flags ⇒ identical trades, byte for byte.
- **bootstraps are seeded** (`seed=0`) in `scripts/pipeline/metrics.py`, `lab.py`, `judge.py`; a
  re-run reproduces every P-value and interval.
- **the lab cache key** (`lab.spec_fingerprint`) hashes the binary's mtime+size, bars dir, config row,
  patch *contents*, args, tickers, cost/sizing args, capital, lookback. change any of them and the day
  is recomputed; the ledger (`research/lab/ledger.jsonl`) records every (spec, window) run as a scanned
  cell.
- **the bar cache is the only moving part.** `data/bars_iex/<T>.csv` (adjusted) and `data/bars_iex_raw/`
  (exact, used automatically when a `data/splits.csv` split is still ahead of the replay date) are
  untracked, appended daily by `scripts/export_day.sh` for the day it runs. **a day the machine was off
  is a hole** (2026-11-05 → 11-11 is one today). a hole shortens the hourly warm-up of every replay in
  the following eight calendar days and makes live and replay disagree on 1h state at the open (the
  2026-11-13 / 11-17 / 11-19 "entry timing" mismatches). backfill:
  `backtest --fetch-bars data/bars_iex --start A --end B --tickers … --feed iex` (+ `--adjustment raw`
  into `data/bars_iex_raw`). the gate refuses a ticker with < 240 sessions in a full year or < 95 % of
  SPY's coverage.
- **leaks to watch:** the binary (`target/release/backtest`, built 2026-10-29) — rebuild changes the lab
  fingerprint and invalidates the cache, which is correct but costs a night; a patch with the same path
  but new contents (fine, content-hashed); `--cross-lag` omitted (the export's `replay_trades.csv` runs
  lag 0 and shows phantom 09:31 trades — read `replay_primary.csv`, which runs lag 1, same as live);
  the live warm-up is SIP history while the replay's is IEX — hourly scores differ by a few hundredths,
  which is the residual that remains after the cache is whole.

### 3.3 the statistics the gates use

all point estimates are best-of-N. the honest machinery (`scripts/pipeline/gates.py`, `metrics.py`):
PF floor 1.4 and ex-best-year PF (a single 2022 cannot carry a candidate); a **paired day-bootstrap**
of candidate − baseline daily P&L over the same sessions, one-sided 90 % bound, **Šidák-adjusted for
`cells_scanned`** (`1 − (1 − α)^(1/n)`); "improvement" gates need the lower bound > 0, "not worse" gates
need the upper bound > 0. the baseline is `live_baseline()`: the promoted row's own five-year sweep
(`live_25`; row 12 maps to the legacy `iex_v18`). this is why a PF-1.85 candidate failed in november:
scanned over 198 cells, its paired lower bound did not clear zero; the human promoted it on the
holdout anyway and said so in `pipeline_events`.

## 4. shadow books

a **book** (`books` row) is one engine set the trader hosts: `primary` (role primary, exactly one
enabled) or `shadow:<name>` (role shadow). a shadow has either `config_version_id = N` (a specific row)
or NULL (follows the promoted row) and optionally its own `tickers` array. mechanics
(`crates/data_feed/src/book.rs`, `docs/dev/books_trader_notes.md`):

- the main loop builds one `MarketState` per symbol (union of all books' tickers, ≤ `MAX_SYMBOLS` 25)
  and calls every book's engine for that symbol. **a shadow always gets a `SimulatedBroker`** regardless
  of `BROKER_MODE`; it cannot reach alpaca. a shadow whose config fails to build is skipped with an
  error, never takes the process down.
- shadows write `trades.source='shadow'`, `trades.book=<name>`, their own `engine_state` rows (PK
  `(book, ticker)`) and `entry_block_events`; `book_sessions` gets one row per (book, date) hosted — the
  pipeline's session counter.
- **books change only at the next trader start** (06:10 PT). `MAX_BOOKS` 12 (set in
  `deploy/systemd/paper-trader.service` and `pipeline.service`); an excess shadow is created but not
  hosted, and the watchdog says "not hosted".
- a shadow verdict is a **parity / plumbing** result: live trades vs `data/live/<date>/replay_<book>.csv`
  (`scripts/replay_book.sh`), count within ±30 %, mean P&L per trade within 10 bps, fills not worse than
  the replay by 10 bps. its PF and its P&L vs the primary are reported, never decided on — at 15–60
  trades "PF ≥ 1" passes a no-edge book half the time. the edge claim was made at the backtest gate.
- shadows today (2026-11-27): `shadow:lab-tier-x150` (#52, row 28, the ×1.5 VPIN tier; mechanics
  confirmed live 11-20), `shadow:pdl15` (human, row 27, prior-day-low filter), and seven v18-lineage
  pipeline shadows whose trials are complete or stale (`vpin-0.26`, `stress-s15-core`,
  `size-vpin26-x1.25`, `thrust-1h15`, `spy-sqrt-band`, `spy-sqrt-band-1h15`, `spy-sqrt-band-v020`,
  `qqq-noise-pm-vol`); `shadow:ant` is proposed (`research/lab/r3_ant.md`), not yet created.

manual shadow (human SQL; the pipeline's `create_shadow` does the same for candidates):

```sql
INSERT INTO books (name, role, config_version_id, tickers, capital, enabled, purpose, candidate_id, created_by)
VALUES ('shadow:NAME', 'shadow', <row or NULL>, <ARRAY['T'] or NULL>, NULL, true, 'why', NULL, 'human');
-- retire
UPDATE books SET enabled=false, retired_at=now(), retire_reason='…' WHERE name='shadow:NAME';
```

## 5. the research flow, end to end

```
idea ─► lab spec (research/lab/specs/…json: config_id, patches[], args[], tickers)
     ─► lab.py run SPEC --window train | 2024h2_25h1 | 2025h1 | A:B      (holdout = LAB_REVEAL=1, once, fixed list)
     ─► judge.py [--round2] → research/lab/REPORT*.md, runs/judge*.json
     ─► pipeline.py propose NAME --kind config --base <row> --gate <gate> --patch P.json --cells-scanned N --study …
     ─► nightly advance: materialize row (status backtesting) → sweep cand_<id> vs live_<promoted> → gate
            pass → row validated, books row shadow:NAME (hosts next start)
            fail → row rejected, cooldown 180 d
     ─► ≥ 20 sessions & ≥ 15 trades (or 60 sessions): parity → shadow_passed → promotion_proposed in docs/pipeline/status.md (ntfy warning)
     ─► human:  UPDATE config_versions SET status='superseded' WHERE status='promoted';
                UPDATE config_versions SET status='promoted', promoted_at=now() WHERE id=N;
     ─► trader hot-reloads per ticker when flat (ConfigWatcher only sees ids HIGHER than the running row); pipeline retires the shadow
```

rules of the road:

- **windows.** train 2022-01-03 → 2025-06-30; holdout 2025-07-01 → 2026-10-30, refused without
  `LAB_REVEAL=1`, revealed once per pre-declared finalist list; after 2026-10-30 is live data (still
  behind the same flag — say why when you use it). screen on `2024h2_25h1`, decide on train, confirm on
  holdout.
- **research sizing** is 0.36 / cap 0.36 (0.45 for row-25-shaped specs, which carry it in `args`), so
  numbers are comparable across the record; the live book sizes 0.30 tiers on $10 000.
- **patch semantics.** a patch's indicator/action with an existing `instance_id` replaces it in place;
  `disable` only applies to actions — switch an indicator off by re-declaring it `"enabled": false`.
  `_sweep_args` in a patch carries flags the gate sweep must use (`--max-position-pct 0.54` for the tier
  cell); unknown top-level keys are rejected by `propose`.
- **what is dead or inert** (found by the round-2 desks; fix in code before relying on it):
  `max_capital_deployed_pct` is never read; `ticker_overrides.sizing_fraction` is ignored under
  `indicator_tiered` sizing; per-ticker `entry_threshold` does nothing on a window-only config; sizing
  actions do not stack; the replay does not enforce `max_concurrent_positions`.
- **rollback** = insert a copy of the old blob as a *new* promoted row (`scripts/update_config.sh`);
  flipping an old row back is invisible to the watcher.
- **prune.** a closed round keeps its report, generators, judge json and the patch/spec files of
  finalists and anything a later step consumes; the rest is deleted and the round's README names the
  commit that holds it (`git show <sha>:<path>`). caches (`data/bars*`, `data/lab_cache`,
  `research/**/daily`) are never tracked.

## 6. routine health checks and what each failure looks like

| when (PT) | job | healthy | failure signature → meaning → action |
|---|---|---|---|
| 06:10 | `paper-trader.timer` → service | `active`, `NRestarts=0`, journal "warm-up … 8 days", `engine_state` rows for every (book, ticker) updating ≤ 30 s | `inactive`/`failed` → CRITICAL, start by hand after checking `/v2/positions`; "starting cold" / "fewer than 3 trading days" → hourly scores suspect (WARNING); "BROKER HOLDS A POSITION THE ENGINE DOES NOT KNOW ABOUT" → orphan, human closes |
| 06:15 | `preopen-check` (claude -p, sonnet) | memo in `agent_memos`, `logs/checkins/preopen-check.jsonl` `is_error:false` | `is_error:true` + "Failed to authenticate: this login reached its time limit" → the claude.ai login expired (happened 2026-11-24 → 27); human runs `claude /login`, durable fix `claude setup-token` + `CLAUDE_CODE_OAUTH_TOKEN` in the unit. `permission_denials` in the json → the allow-list in `.claude/settings.json` rejected a command (it matches a single leading token; no `$(…)`, no `VAR=…;`) |
| 06:25–13:15 /5 min | `scripts/watchdog.sh` (no LLM) | silent, or `[info]` abnormal-move lines in `logs/notifications.log` | `[critical] ALL tickers feed_stale` → feed down **or a market holiday / early close** (the stack has no holiday calendar: 2026-11-26 and the 13:00 ET close on 11-27 both fired this falsely); a single ticker `feed_stale` → IEX gap, usually self-heals; "position still open 5 min after force_exit_by" → CRITICAL, check the broker; "book NAME not hosted" → WARNING, normal on a shadow's first day |
| 13:10 | `paper-trader-stop` | SIGTERM → flatten → `inactive`, `Result=success` | a position at the broker after stop → CRITICAL; on a **half-day** the flatten runs after the close and the order queues — a known gap, no calendar check yet |
| 13:20 | `export-day` → `data/live/<date>/`, commit+push | 25 files incl. one `replay_<book>.csv` per enabled book; push ok | "could not read Username for github.com" → `gh auth git-credential` returned nothing in the service (gh login expired; human re-logs); "Could not resolve host" → no network; **a day with no export = a bar-cache hole** (§3.2) |
| 13:30 | `eod-review` (claude -p, opus) | memo "plumbing: ok \| mismatch …; strategy …"; a `created_by='claude_eod'` promoted row would mean it used its tuning authority — check `parent_version_id` and the memo's validation numbers | same auth signature as pre-open; "plumbing: MISMATCH" two sessions running → the cache hole / warm-up gap, backfill before believing any replay |
| 13:35 | cloud EOD routine | `docs/reports/<date>.md`, briefing, email | **silent since 2026-10-20** (cloud side; the desktop eod-review covers the plumbing verdict) |
| 20:00 | `research-runner` | runs `status: approved` RQ entries; holds `logs/.research.lock` | lock held past 20:30 → the pipeline waits; a dead sweep leaves a candidate in `backtesting` (`advance` resumes it) |
| 20:30 | `pipeline.timer` → `nightly.sh` | 197 unit tests green, `advance --max-backtests 2`, `report`, commit of `status.md` + `data/cand_*.args` | red tests → night skipped + ntfy; "refusing to gate: bar coverage" → fetch the gap; candidate stuck in `backtesting` → check `logs/sweeps/` |
| 21:15 | `pretest.timer` | `research/pretest/results/summary.md` regenerated, CHANGELOG line per changed verdict | a cell erroring is logged, not fatal |
| Sun 18:00 | `ideate` | `docs/ideation/<date>.md`, ≤ 3 new cells appended to `research/pretest/cells.py` | auth failure as above; "no new cells" is a valid outcome |
| 1st trading day of month | eod-review §7 mechanics audit → `docs/audits/<month>.md`; `scripts/repo_report.sh` | proposals only | — |

infrastructure to glance at weekly: disk (`df -h /`; 96 % on 2026-11-27, `target/debug` 15 GB is the
lever), the postgres container (`podman ps` healthy), `logs/trade_spool.jsonl` absent (a present spool =
trades the DB refused, a human inserts them), `git rev-list --count origin/mainline..HEAD` (unpushed
commits mean the nightly push is failing).

known gaps as of this revision: no holiday / early-close awareness anywhere in the start, stop,
watchdog or `session_clock.rs` path (alpaca `/v2/calendar` or `/v2/clock` would do); the IEX cache hole
2026-11-05 → 11-11; three stale v18-lineage promotion proposals (#6, #28, #44) in `status.md`; the cloud
EOD routine.

## 7. who decides what

**human only** (never do these from a session; write the SQL / command into the report instead):

- promote or roll back a config row; `superseded` flips. (the eod-review check-in's guardrailed
  `update_config.sh` path is the sanctioned exception; an agent in a session never uses it.)
- create or retire a `books` row (the auto-mode classifier denies the INSERT to the agent; write it out).
- withdraw a candidate, change a gate's thresholds, change `cells_scanned` after the fact.
- reveal the holdout (`LAB_REVEAL=1`), or run any window after 2026-10-30 — allowed when pre-declared and
  written down (one cell, no selection), still the human's protocol to loosen.
- `git push`, `git clean`, edits to `.claude/settings.json`, systemd unit changes, credentials
  (`gh auth`, `claude /login`, `.env`). ntfy and `NOTIFY_TOPIC` are the exception: free to change.
- anything that submits, cancels or flattens at the broker by hand.
- bulk regate of shadows; raising `MAX_BOOKS`.
- choosing a new instrument class or a different strategy family (a research *programme*, not a cell).

**an agent can answer from context** (and should, without asking):

- plumbing verdicts: live vs `replay_<book>.csv` twin-matching, feed-stale triage, orphan detection,
  whether an alert was a holiday / half-day artefact.
- running lab cells on train / screen windows, judging them with the registered tests, writing the
  round's report, pruning the grid with the holding commit named.
- `pipeline.py propose` of a cell that passed the lab (gate named, `cells_scanned` honest), `show`,
  `report`, `advance --dry-run`, `regate` of a single candidate when the baseline was wrong.
- diagnosing a failed job from `journalctl --user -u <unit>` and `logs/checkins/*.jsonl`, and naming
  the fix; running the fix when it is a cache backfill or a doc/report regeneration.
- a bar-cache backfill (`--fetch-bars`), rebuilding `target/release` after a code change (and noting
  that it invalidates the lab cache), running the unit suites.
- memory and docs upkeep (`CLAUDE.md`, this file, briefings, skills' "row N note" sections) after a
  promotion or a protocol change.
- declining to tune on today's P&L, a 3-trade streak or a single big day — the eod-review skill's
  "not evidence" list is the rule.

**grey zone → do the work, then ask:** opening a *human* shadow for a variant no gate fits (write the
INSERT, let the human run it); amending a test's clause before a reveal (allowed if written into the
plan before the reveal, as round 2 did); an engine no-op discovered mid-round (report it, do not patch
crates inside a research round).

## 8. keeping this document current

update this file in the same commit whenever one of these changes: the promoted row (§1, §2.1, §4
shadow list), a gate threshold or a new gate (§3.3), the timers or units (§6), `MAX_BOOKS` /
`MAX_SYMBOLS`, a lab window boundary or the reveal rule (§5), a known gap opened or closed (§3.2, §6),
a classifier denial pattern that moves an action between the columns of §7. bump the "last updated"
line. the pre-open and eod-review skills carry a "row N note" per promotion; this file carries the
loop-level view only — do not duplicate the per-row trading notes here.

revision log:

- 2026-11-27 (later) — fact-check of the visual map (`docs/engine-map-review.md`) surfaced that the
  eod-review check-in can insert a promoted row via `update_config.sh`; §1, §2.2, §6, §7 now say so.
- 2026-11-27 — first version. row 25 promoted 11-11; round 2 (`research/lab/ROUND2.md`) found it at a
  local optimum; check-in auth expiry, holiday / half-day gap, cache hole, ANT test recorded.
