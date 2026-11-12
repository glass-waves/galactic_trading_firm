# lab round 2 (2026-11-11 →): permute from row 25

Goal (user, 2026-11-11): "start from the best we have and permutate from there" — brainstorm changes, additions and
tweaks to the promoted row 25 (`lab-na-bundle`, promoted 2026-11-11 over v18), narrow them by backtest to the top
few, and run those as shadow books. Round 1 (`PLAN.md`, `REPORT.md`, `HISTORY.md`) found row 25; this round starts
there. Everything in HISTORY §D (do-not-repeat) still stands unless the base change makes it a new question.

## base

`research/lab/specs/base/row25.json` — config row 25 replayed at research sizing (`--sizing-fraction 0.36
--max-position-pct 0.45`, house costs, lookback 8, IEX cache). Verified equal to `frontier/na_bundle` trade for
trade on 2025h1. Every tweak is a patch on `config_id: 25` and is compared against this spec with `lab.py compare`.

Row 25 in one line: four names (AMZN AAPL NVDA MSFT), short-only; `window_5m_thrust_short_am/_pm` and
`window_strong_core_short_am/_pm` (am to 10:00, pm 10:00–11:00, flat 11:55), `window_strong_core_stress` (SPY ≤ −1.5 %
intraday via `cross_1m.index_session_ret`, entries 09:45–15:30, max hold 2 h, score exit −0.6, flat 15:55); SPY sqrt
band k 0.05; thrust 1h ≤ −0.10; `vpin_1m.raw_vpin` ≥ 0.20; `adx_14_1hr` disabled; `sizing_tiered_vpin` base 0.30
(research 0.36) ×1.25 at VPIN ≥ 0.26; cap 0.40 (research 0.45); max 3 positions, 95 % capital; `reject_1m_noise(_short)`
gates; exits `trailing_stop_atr`, `hard_stop`, `max_hold`, `exit_session`, `breakeven`. Dump the blob:
`./scripts/psql.sh -tA -c "select config_blob from config_versions where id=25" | python3 -m json.tool`.

Record (round 1, research sizing): train 2022-01→2025-06 +2610 / 320 / PF 1.75 (22 +1443 · 23 +136 · 24 +650 · 25h1 +380);
holdout 2025-07→2026-10 +899 / 111 / PF 2.00 (revealed once, for six finalists); October 2026 −11 / 9. NVDA ≈ 70 % of
P&L, AAPL loses every year, ~7 trades/month, ScoreExit trades lose in every cell ever run.

## protocol (unchanged from round 1)

- train 2022-01-03 → 2025-06-30 is searched; **holdout 2025-07-01 → 2026-10-30 is never run by a desk** (the lab refuses
  it without `LAB_REVEAL=1`; do not set it). The coordinator reveals it once, for a fixed finalist list of ≤ 6.
- screen cheaply on `2024h2_25h1` (≈ 250 sessions), confirm on `train`; every run is a ledger line (`LAB_AGENT=r2-<desk>`),
  and the ledger count is the multiplicity denominator the pipeline charges. Do not run cells you do not need.
- the binary (`target/release/backtest`, 2026-10-29) is frozen; no edits under `crates/`. Engine ideas go in the desk
  report as proposals.
- **warm-up stays 8** (`lookback 8`; do not set `session.warmup_days`): the trader warms each symbol to the max over the
  books trading it, so a shadow with a longer warm-up on AMZN/AAPL/NVDA/MSFT/SPY would change the primary's own trades.
- specs under `research/lab/specs/r2/<desk>/`, patches under `research/lab/patches/r2/<desk>/`, report at
  `research/lab/desks/r2-<desk>.md`. No DB writes, no pipeline, no trader, no commits (coordinator commits).

## success tests (pre-registered; the coordinator applies them with judge.py against base/row25)

**T (tweak — replaces the primary):** train: paired vs row 25 P(diff ≤ 0) < 0.05, PF ≥ 1.6, no year < −300, ex-best-year
PF ≥ 1.4, trades ≥ 250; holdout: P&L > 0, PF ≥ 1.5, paired vs row 25 P(diff ≤ 0) < 0.25 (at least not worse);
target 2025-01→2026-10: PF ≥ 1.4 and ≥ 150 trades (the programme's goal); dd ≥ −600, worst day ≥ −300.
**A (additive — a second book or extra window alongside row 25):** train: the added trades alone ≥ 60, PF ≥ 1.3,
worst year ≥ −200; pooled PF ≥ row 25's; daily P&L correlation with row 25 < 0.3; holdout: added P&L > 0.
Multiplicity: the paired test is judged at Šidák over the number of finalists; the pipeline will charge
`cells_scanned` = this round's ledger count for the desk that found the cell.

## desks and assignments

- **r2-exits** — exits and holds on row 25's windows: score exit off / level per window; max_hold per window (am, pm,
  stress); breakeven / ATR trail / hard stop parameters; a VWAP stop scoped to the pm or stress windows (`vwap_stop`,
  `scope_min_max_hold_ms`); pm window force_exit later than 11:55 (12:30, 13:00); a one-bar max-hold shift as the RQ-14
  sensitivity check. Known: "score exit off = no change" on the bundle (frontier); ScoreExit trades lose everywhere.
- **r2-entries** — entry filters and window clocks: VPIN floor 0.15–0.25 and the tier split point; thrust 1h −0.05…−0.20
  per window; sqrt k 0.04–0.06; am/pm cut-over 09:50–10:10 and pm end 11:00 → 11:30; stress threshold 1.0–2.0 % and its
  entry clock; the prior-day-low exclusion (`prior_day_levels` `pdl_5m.dist_low_pct ≥ −1.0`, round-1 `meta/v18_pdl10`,
  `frontier/na_pdl10`) as a dial, not a cut; quality conditions from `vwap_distance`, `relative_volume`, `noise_area`
  metadata; peer / cross conditions. Known: every lever that adds post-2022 volume added it at PF 0.4–1.0.
- **r2-sizing-universe** — tiers (split 0.22/0.26/0.30, mult 1.0–1.5, a third tier), base 0.30 vs 0.36 live, cap,
  `volatility_scaled` on top of the tier, per-ticker overrides (NVDA entry/exit thresholds and sizing, AAPL down-weight
  rather than cut — run the AAPL cut only as a control), `max_concurrent_positions` 2/3/4 with `max_capital_deployed_pct`,
  `max_daily_loss_pct`, `entry_cooldown_ms`; adding QQQ / SPY / one or two names under row 25's engine (round 1: picks
  do not persist — show the control).
- **r2-second-books** — additive books with ~0 correlation, judged as a union (`judge.py` takes "a.json+b.json"):
  the QQQ afternoon noise book (row 23 family, `meta/qqq_noise_pm_dm07`, `specs/frontier/*qqq*`) re-based next to
  row 25; the afternoon long (`long-2026/r3_q1_vpin`) at warm-up 8 (it needed 22 — does anything survive at 8, or on
  QQQ/SPY only?); an afternoon short-continuation or VWAP-reclaim-fail window on the four names inside row 25 (check it
  does not pre-empt the existing windows — diff the trades); stress variants (1.0 % with a tighter 1h, 2.0 % with a
  longer hold, a stress-reversal long on day +1). Known: morning longs dead under any 1h; new morning triggers pre-empt.

Coordinator's own list (folded into the desks above): pm window to 11:30; tier ×1.5 at 0.26; NVDA-only size-up;
pdl dial; stress 2 % + long reversal; AAPL down-weight; VWAP stop on pm trades; `max_concurrent` 2 with bigger size.

## phases

1. desks brainstorm within their theme (write the idea list into the report first, with the mechanism and the
   expected failure mode), then screen ≤ 25 cells each on 2024h2_25h1, confirm ≤ 8 on train, nominate ≤ 3.
2. coordinator: fixed finalist list (≤ 6) written here **before** the reveal; `LAB_REVEAL=1 judge.py … --baseline
   research/lab/specs/base/row25.json`; verdicts table in `REPORT2.md`.
3. top ≤ 3 → `pipeline.py propose` (base row 25, gate by kind: quality-config for tweaks, additive-config /
   standalone-strategy for books) → nightly gate → shadow books (3 slots free under MAX_BOOKS 12).
4. prune: keep the report, the base spec, the finalists' specs/patches; delete the rest of the grid (git holds them).

## finalists (fixed 2026-11-12 before the holdout reveal)

Desk hand-backs: exits nominated nothing (every hold/stop/VWAP-stop variant flat or negative; RQ-14's clock offset
immaterial); sizing nominated one leverage cell; entries two near-misses; second-books an afternoon long book and its
un-dialled control. **No cell passed test T's train clause (paired vs row 25 P < 0.05).** The reveal is run anyway, once,
for the list below, so the holdout can say whether any of them deserves a shadow slot:

1. `r2/entries/peersnotall_thr` — T; no am thrust when all three peers are red (train +2839 / 282 / PF 1.98, paired P 0.169).
2. `r2/entries/pdl15` — T (quality variant); prior-day-low ≥ −1.5 % on the four am/pm windows (+2634 / 256 / PF 2.16, P 0.47).
3. `r2/sizing/tier_x150` — T (leverage); ×1.5 at VPIN ≥ 0.26 (+2984 / 320 / PF 1.77; matched-notional +75, P 0.195).
4. `base/row25 + r2/books/m5_r3_ls5_dm175` — A; afternoon high-vol breakout long book, four names, warm-up 8
   (book +1071 / 97 / PF 1.69, corr +0.004; one day, 2025-04-09, is +757 of it).
5. `base/row25 + r2/books/m1_r3_ls5` — A (control for 4, round-1 gate 1.5): book +935 / 153 / PF 1.42.

Amendment to test A, made before the reveal: "pooled PF ≥ row 25's" is impossible for any book below PF 1.75 (a pooled PF
lies between the two books'), which was not the intent. It is replaced by: pooled PF ≥ 1.5 **and** pooled max drawdown no
deeper than 1.5 × row 25's on the same window. The other A clauses stand (added trades ≥ 60, added PF ≥ 1.3, added worst
year ≥ −200, correlation < 0.3, holdout added P&L > 0).
