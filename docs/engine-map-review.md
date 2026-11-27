# engine map — fact-check of the shareable visual (2026-11-27)

checked against `docs/agent-breakdown.md`, the skills, `scripts/`, `crates/data_feed` and the database
on 2026-11-27. legend: **OK** · **WRONG → fix** · **MISLEADING → better wording**.

one finding changes several lines and the breakdown doc itself (fixed in the same commit): the desktop
**eod-review check-in holds tuning authority** (`.claude/skills/eod-review/SKILL.md` §4 and line 173):
under its guardrails (one parameter, 20-day validation) it may run `./scripts/update_config.sh`, which
inserts a **new promoted row** and supersedes the old — the script is on the check-in allow-list. it has
never used it (`config_versions.created_by` has no `claude_eod` rows; every promotion so far was human or
the archived orchestrator), and the pipeline lane and the cloud routine can only *propose*. so "only a
human promotes" is true of the pipeline, not of the whole system. the human should decide whether to
keep that authority (it predates the pipeline, 2026-09) or remove `update_config.sh` from the check-in
allow-list and skill §4 so the statement becomes true everywhere.

## part 1 — engine

| # | verdict | note |
|---|---|---|
| 1 | **MISLEADING** | "nothing with an LLM can touch an order" is true (the check-ins have read-only broker access, `alpaca_api.sh` is GET-only). but "meta engine only tunes settings" + the finding above: say *"an LLM can never place, cancel or flatten an order; it can only change the config row the engine reads — and in one guardrailed path (the EOD check-in) that change goes live at the next start without a human"*. |
| 2 | **OK** | add "same binary" — the lab cache key includes the binary's mtime+size; a rebuild recomputes everything. |
| 3 | **WRONG → fix** | two routes end in a promoted row: (a) the pipeline lane → human SQL by hand; (b) the EOD check-in's `update_config.sh` (guardrailed, never yet used). write: *"the pipeline lane only ever ends with a human promoting by hand; a second, guardrailed path lets the EOD check-in insert one validated one-parameter change for the next day"*. |
| 4 | **OK** | AMZN, AAPL, NVDA, MSFT. stage list fine; "Sizing & risk" = sizing action + session clamps. |
| 5 | **MISLEADING** | the websocket is alpaca's *market-data* stream (IEX feed), not the broker; symbols are the **union of every book's tickers + SPY** (today that adds QQQ for a shadow). warm-up is 8 calendar days of **SIP** history, which is why live and replay hourly scores never match exactly. |
| 6 | **MISLEADING** | live hourly candles are not hurt by a missing day — the live warm-up comes from alpaca history. a missing day in the **bar cache** (a day the machine was off) shortens the *replay's* warm-up, so replay ≠ live for the next eight calendar days. write: *"a day with no export is a hole in the replay history; replays near it disagree with live until it is backfilled"*. |
| 7 | **OK** | "market move" = SPY session return via the cross feed, lagged one bar. |
| 8 | **WRONG → fix** | "only a person changes it": the pipeline inserts rows (`backtesting`/`validated`/`rejected`), a person (or, guardrailed, the EOD check-in) flips one to `promoted`. keep "one numbered frozen config, exactly one promoted, immutable, every trade stamped with the row id". |
| 9 | **OK** | `catch_unwind` per indicator. |
| 10 | **MISLEADING** | the morning windows gate on 5m/1h scores, the SPY band *scaled by √(minutes into the session)* and VPIN ≥ 0.20; the stress window gates on the **unscaled** SPY session return ≤ −1.5 %. "order flow" = VPIN only; there is no separate OFI gate on row 25. |
| 11 | **OK** | entries 09:45–15:30 ET, max hold 2 h, score exit at −0.6, flat 15:55. |
| 12 | **OK** | 0.30 base, ×1.25 at VPIN ≥ 0.26, cap 0.40 per position, 3 positions, no new entries 15:30, forced exit 15:55 (morning windows have their own 11:55 flat). |
| 13 | **OK** | `AlpacaBroker` for the primary only; `BROKER_MODE=alpaca_paper`. |
| 14 | **OK** | 12 books incl. primary (`MAX_BOOKS`), ≤ 25 websocket symbols; a 13th is created but not hosted. |

## part 2 — loop

| # | verdict | note |
|---|---|---|
| 15 | **MISLEADING** | "backtests replay it": the export *appends the day's IEX bars to the cache* and runs one same-day replay per book; **every** later replay (gates, lab) reads that cache. the rest is right. |
| 16 | **OK** | |
| 17 | **MISLEADING** | "only way a config reaches the broker" — see the finding; both paths end as a new promoted row. "two DB updates" is the human path; `update_config.sh` does the same two in one script. hot-reload wording OK; a ticker the process is not subscribed to joins at the next 06:10 start. |
| loop | **OK** | "Candidate (AI)": every one of the 52 candidates was filed from a session (recorded as actor `human`); a person can file one the same way. |
| 18 | **MISLEADING** | the weekly brainstorm adds up to 3 **pretest cells** (stage 0, outside the engine), not candidates; the EOD check-in's proposals go to the research queue or `update_config.sh`; "never tune on one day / a streak / one big day" is the EOD skill's rule — OK. |
| 19 | **OK** | screen `2024h2_25h1`, decide on train 2022-01 → 2025-06, holdout 2025-07 → 2026-10-30 revealed once per pre-declared list. add: dates after 2026-10-30 sit behind the same `LAB_REVEAL` flag. |
| 20 | **OK** | `cells_scanned` → Šidák `1 − (1 − α)^(1/n)`. |
| 21 | **OK** | same `TradingEngine`, next-bar-open fills, 3 bps + $0.005 half-spread. |
| 22 | **MISLEADING** | thresholds are **per gate**: quality-config PF ≥ max(1.4, baseline) and the paired *lower* bound > 0; "not worse" gates need the *upper* bound > 0; sizing gates compare at matched notional; additive gates test the added half. ex-best-year, Šidák, 180-day cooldown: OK. |
| 23 | **OK** | add: 60 sessions with < 15 trades = `insufficient_activity`, not a verdict. |
| 24 | **OK** | |
| 25 | **OK** | `docs/pipeline/status.md` + ntfy warning. |
| 26 | **MISLEADING** | add the EOD-check-in exception from the finding, or remove it from the allow-list and leave the line as is. |
| 27 | **OK** | |
| 28 | **OK** | `live_baseline()` → `live_<row>` sweep. |

## part 3 — day

| # | verdict | note |
|---|---|---|
| 30 | **OK** | heartbeat 30 s; warm-up 8 calendar days; the orphan line is CRITICAL (adopted at start under `ORPHAN_POLICY=adopt`, human closes if not). |
| 31 | **OK** | the pre-open skill says "do not apply it yourself"; its broker access is GET-only. |
| 32 | **OK** | add "book NAME not hosted" = normal on a shadow's first day. abnormal-move lines (name ≥ 1.5 %, SPY ≥ 1 %) are the frequent *info/warning* output. |
| 33 | **OK** | stop = SIGTERM → flatten every book → inactive. the half-day note is correct and still open (no calendar check anywhere). |
| 34 | **OK** | push uses `gh auth git-credential`; "could not read Username" = expired gh login. |
| 35 | **WRONG → fix** | "never promote": the skill's §4 lets it *apply for tomorrow* via `update_config.sh` (a new promoted row), one parameter, validated on the last 20 sessions, memo with `proposed_config_version_id`. never exercised so far. write: *"may apply one validated one-parameter change for the next session (guardrailed; the pipeline lane is separate and never promotes)"*. "two-session mismatches usually = history gaps, backfill first" — OK as of this month. |
| 36 | **OK** | no LLM: `scripts/research_runner.sh` runs the `run_cached_sweep.sh` commands written in approved `docs/research_queue.md` entries and writes results back. |
| 37 | **OK** | `nightly.sh`: pipeline unit tests (197) → `advance --max-backtests 2` → `report`. |
| 38 | **MISLEADING** | the pretest grid is **stage 0**: cheap python replicas (numpy, no engine) of documented anomalies from `docs/edge_catalog.md`, each with a pre-registered kill criterion; verdicts `pass-pretest` / `needs-product` / `dead`; a pass *earns an engine study*, nothing flows into the pipeline automatically. "re-scores … logs changed verdicts" — OK. |
| 39 | **OK** | up to 3 new cells appended to `research/pretest/cells.py`, never editing an existing one. |
| who decides | **MISLEADING** | "AI never … promotion" → "AI never places an order or changes a gate; the pipeline never promotes; the EOD check-in may apply one guardrailed change (never used)". everything else OK. a person can also file ideas, candidates and pretest cells. |

## answers to the three open questions

- **36 — LLM?** no. the research runner is a shell script running backtest sweeps the human approved in the queue.
- **38 — pretest grid?** stage 0 of research: python replicas of catalogued anomalies with kill criteria, outside the engine; a pass earns a lab/engine study.
- **18–20 — can a person file?** yes: `pipeline.py propose` records every proposer as `human` (all 52 so far came from sessions); lab specs and pretest cells are plain files.
