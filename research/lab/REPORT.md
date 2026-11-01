# lab round 2026-10-31 → 11-01: a config with PF > 1.4 on 2025-01 → today

Goal (user, 2026-10-31): a configuration that is profitable now — PF > 1.4 on 2025-01-01 → 2026-10-30 — found by
parallel, pre-registered search. Protocol and success tests: `PLAN.md`. Everything tried before this round: `HISTORY.md`.
Seven desk agents, 198 distinct specs / 245 runs in `ledger.jsonl`, desk reports in `desks/`. Judge output: `REPORT_judge.md`.

## verdict

**`frontier/na_bundle` is the contender.** It passes five of the six pre-registered tests and misses the sixth (the
paired-vs-v18 test) only at the multiplicity-adjusted level: P(no better than v18) = 0.025 against a Šidák bar of
0.017 for six finalists. It is in the pipeline as candidate **#50 `lab-na-bundle`** (gate quality-config, cells_scanned 72);
the sizing-free core is **#51 `lab-na-core`**. The nightly pipeline runs the five-year gate sweep, materializes a row,
verifies it against the patch, and opens a shadow book; the shadow is the plumbing test (PLAN test 6). Promotion stays human.

| window | P&L | trades | PF | max dd | worst day | sign-flip p | per year |
|---|---:|---:|---:|---:|---:|---:|---|
| train 2022-01 → 2025-06 (searched) | +2610 | 320 | 1.75 | −238 | −168 | 0.0007 | 22 +1443 · 23 +136 · 24 +650 · 25h1 +380 |
| **holdout 2025-07 → 2026-10 (never searched)** | **+899** | **111** | **2.00** | −133 | −45 | 0.011 | 25h2 +203 · 26 +695 |
| **target 2025-01 → 2026-10** | **+1279** | **152** | **1.96** | −217 | −168 | 0.008 | 25 +584 · 26 +695 |
| v18 (promoted) on the same target window | +701 | 172 | 1.42 | | | | 25 +332 · 26 +369 |

Research sizing (0.36 base, 0.45 cap, $10k). Paired day bootstrap vs v18 on the target window: +578, 90 % interval
[+97, +1085], P(≤ 0) 0.025.

### what it is

Row 12 (v18) with:
1. the SPY **sqrt band** instead of the flat ±0.2 % band, morning windows cut over at 10:00 and a 10:00–11:00 extension
   (the #44 mechanism; k = 0.05);
2. **thrust 1h ≤ −0.10** (v18: ≤ 0) and **VPIN floor 0.20** (v18: 0.217);
3. **`adx_14_1hr` switched off.** This is the new finding of the round (indicators desk): the legacy ADX term
   (`adx/50 − 1`, direction-free) pulls the 1h score down in choppy low-ADX hours and lets thrusts through that run
   against the short; with it off, the 1h gate keeps the trending-hour thrusts only. On the sqrt-band family it lifts
   post-2022 PF from 1.31 to 1.44 and makes every year positive;
4. the **stress-s15-core window** (SPY ≤ −1.5 % intraday, core shape, to 15:30) — 33 trades on train, 25 of them 2022;
5. the **×1.25 size tier at VPIN ≥ 0.26** (#28's lever; base 0.30 / cap 0.40 live, `_sweep_args --max-position-pct 0.45`).

Patch: `patches/frontier/na_bundle.pipeline.json` (self-contained on row 12). Lab spec: `specs/frontier/na_bundle.json`.

### the six finalists (all passed the holdout; none passed test 4 at Šidák-over-6)

| finalist | train PF / n | holdout PF / n | target PF / n / P&L | paired P(≤0) vs v18 | tests |
|---|---|---|---|---|---|
| frontier/na_bundle (#50) | 1.75 / 320 | 2.00 / 111 | 1.96 / 152 / +1279 | 0.025 | 1 2 3 5 pass, 4 miss |
| frontier/nb_v15 (VPIN 0.15) | 1.61 / 397 | 1.51 / 137 | 1.50 / 190 / +955 | 0.226 | 1 2 3 5 pass |
| indicators/r21_v020_noadx_t10 = frontier/na (#51) | 1.69 / 287 | 2.02 / 108 | 1.95 / 142 / +1058 | 0.079 | 2 3 5 pass; 1 fails on count |
| universe/nt10_b3 (core without AAPL) | 1.90 / 251 | 1.86 / 95 | 2.00 / 122 / +950 | 0.199 | 2 3 5 pass; 1 fails on count |
| frontier/na_pdl10 (core + prior-day-low exclusion) | 2.11 / 170 | 2.26 / 59 | 2.52 / 75 / +751 | 0.457 | 2 3 5 pass; 1 fails on count |
| na_bundle + QQQ afternoon book (two books) | 1.66 / 549 | 1.82 / 161 | 1.79 / 232 / +1604 | 0.029 | 1 2 3 5 pass, 4 miss |

Every finalist's holdout PF came in at or above the top of the desks' written expectations (1.2–1.45 for na_bundle).

## caveats — read before promoting

- **The holdout is clean for the ADX-off change and the bundle, not for the family.** The sqrt-band / 1h-dial
  mechanism (#44, #45, row 21) was selected in earlier rounds on five-year sweeps that included 2025–26, and HISTORY §C
  listed its 2026 numbers. The row's base edge was therefore known to be good on the holdout before this round; what the
  holdout genuinely tests is the ADX-off change, the stress window and the tier, plus the specific parameter choice.
- **Concentration.** NVDA is ~70 % of P&L; AAPL loses on every engine (−300 on train). 2023h1 is −84. The desks declined to
  drop AAPL as a blind cut (nt10_b3 is the diagnostic); a live book can decide that with its own data.
- **Volume.** ~92 trades/yr on train, ~82/yr after 2022, ~7/month on the target window — the 150-trade bar is met with
  two to spare. Dollars at research sizing: ≈ +$700/yr on $10k. This is a better version of the same thin book, not a
  different business.
- **Multiplicity.** This round alone ran 198 specs; the programme's history is ≥ 2,100 configurations on the same five
  years. The pipeline gate will apply Šidák over `cells_scanned` = 72 to its own paired test; it may fail that gate even
  though it passed the holdout. If it does, the human decision is whether the out-of-sample holdout pass (which the
  gate does not know about) outweighs the family-wise bar.
- **Sizing levers.** The tier and the 0.45 cap are leverage on the same trades (PF 1.96 at VPIN ≥ 0.26 vs 1.52 below on
  train). The core without them (#51) has the same shape of record on fewer trades; if the tier is unwanted, promote #51.
- **Live differences** the lab does not model: fills at next-bar open vs signal-bar close, the 3-position cap, the
  max-hold clock offset (RQ-14). The shadow book is the test for all of these.

## what the other desks established (negatives are results too)

- **meta** (373 archive tags re-scored): post-2022 edge exists only in the sqrt-band + 1h-dial family; stronger triggers,
  stress add-ons, entries after 11:30, rolling bands and loosened 1h gates pay only in 2022. ScoreExit trades lose in every
  year of every cell. The prior-day-low exclusion is a real, independent lever (PF 2.11 / 49 trades a year).
- **indicators**: the three audit fixes as specified do not help — RQ-13 dir1h +767/548/PF 1.12 (the directional 1h makes
  "1h ≤ −0.4" common and starves the thrust window's lag condition); MACD `normalization_pct` churns 45–65 % of trades
  for ±$40; `vpin.bucket_divisor` is dead code; warm-up 12 changes 3 trades. ADX-off is the finding.
- **frontier**: the plateau around na_bundle holds PF 1.64–1.75 (k 0.04–0.06, VPIN 0.15–0.217, 1h −0.10…−0.20, stress
  1.5–2.0 %); cliffs only on the loose side (1h −0.05, stress 1.0 %). Score exit off = no change. Every lever that adds
  post-2022 volume adds it at PF 0.4–1.0.
- **universe**: 37 names screened on three engines; per-name rank correlation between periods −0.04…−0.32, picks do
  worse than the rest next period in 9 of 11 splits. Sector ETFs dead. Engine gains on the four do not carry.
- **long-2026**: morning longs lose under any 1h definition (−5,779 / 2,345 / PF 0.80 on train for row 12's long
  windows; PF 0.83 with the directional 1h). One afternoon long on the four names (row 23's noise-area breakout with a
  VPIN floor) is PF 1.66 on train with 3 trades/month and a 22-day warm-up the trader lacks; kept for a later round
  (`specs/long-2026/r3_q1_vpin.json`).
- **tom-event**: turn-of-month session +2 is the best of 25 calendar slots at z +1.92 where the expected best of 25
  noise draws is 1.97; the shifted-calendar control is not flat. Closes RQ-11 negative in-engine.
- **signals-new**: a new morning window on the same names pre-empts v18's own entries (keeps 35–69 of 97 trades); the
  afternoon VWAP fade, gap fade, prior-low break and engulfing windows all lose on train. No additive trigger exists
  in this state space.

## engine proposals the desks wrote up (not implemented; the binary was frozen for the round)

1. `--patch-json "disable"` should cover indicators or reject unknown ids (silent no-op today; the lab brief now warns).
2. a per-window ticker scope and a window yield/lock-out, so a second trigger or a pool-wide window can share a config
   without cannibalising the primary windows;
3. breakeven / trail / stop actions scoped to a window (the afternoon long cannot live inside row 12 otherwise);
4. remove `vpin.bucket_divisor` or implement real volume buckets; a thrust-lag condition that compares 5m to 1m only;
5. a normalised-stretch metadata key on `vwap_distance` (RQ-10); a relative-volatility key on `noise_area`.

## next

1. Tonight's pipeline advance runs #50 and #51 through the five-year gate (two backtests per night; 7 shadows are
   hosted, cap 12 — the five regate-fail retirements free slots). Watch `docs/pipeline/status.md`.
2. The shadow book replays and live-loop comparison are PLAN test 6. A promotion decision is the human's, informed by the
   holdout pass above and the gate's family-wise verdict.
3. Later rounds: the afternoon long book once warm-up 22 is supported by the trader; the prior-day-low clause as a
   quality dial if volume stops mattering; a universe re-screen under a scale-free MACD (RQ-12 phase C).
