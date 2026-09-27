# Crash-days hindsight: precursors and anatomy

*research only, 2026-09-27. `research/crash_days/analysis.py`, rerunnable, numpy + csv/json only. Universe: SPY session return (bars_iex open of first RTH bar → close of last), 2022-01-03..2026-09-25, 1,186 sessions — this exactly reproduces the stress study's tail sets (117 days SPY<−1%, 25 days SPY<−2%) and, cross-checked against `data/iex_v18_<year>_trades.csv`, iex_v18's P&L on them: **+$1,022.29 on 38 trades** on the 117 days and **+$197.88 on 6 trades** on the 25 days — matches `research/stress/2026-09-26_stress_mode.md` (+1,022 / +198) to the cent. VIX is not in the local data; every vol feature below is realized (SPY), not implied.*

## 0. Base rates

| open→close | n | close→close | n |
|---|---|---|---|
| < −2% | 25 | < −2% | 36 |
| −2..−1% | 92 | −2..−1% | 124 |
| −1..−0.3% | 237 | −1..−0.3% | 219 |
| flat (±0.3%) | 402 | flat | 353 |
| > +0.3% | 430 | > +0.3% | 454 |

P(close<−1%)=**9.87%**, P(close<−2%)=**2.11%**. By year (<−1%): 2022:54, 2023:19, 2024:15, 2025:18, 2026(partial):11 — over half the tail sample is 2022.

## 1. Precursors (known at 09:30)

Mean value by open→close class:

| feature | <−2% (n=25) | −2..−1% | −1..−0.3% | flat | >+0.3% |
|---|---|---|---|---|---|
| overnight gap % | **+0.21** | −0.07 | +0.03 | +0.03 | +0.04 |
| prior 1d ret % | +0.19 | −0.20 | +0.04 | +0.07 | +0.07 |
| prior 3d ret % | −0.29 | −0.37 | +0.15 | +0.36 | +0.06 |
| prior 5d ret % | −0.88 | −0.49 | +0.35 | +0.62 | +0.03 |
| realized vol 20d (ann. %) | 19.7 | 18.3 | 15.5 | 13.9 | 16.5 |
| dist from 20d high % | −5.0 | −4.1 | −2.2 | −1.6 | −2.9 |
| dist from 52w high % | −11.7 | −8.9 | −5.3 | −3.4 | −6.3 |
| above 50d MA (rate) | 40% | 39% | 70% | 81% | 61% |
| above 200d MA (rate) | 30% | 57% | 77% | 87% | 71% |

**Gap doesn't warn you**: the worst class has the *most positive* mean gap — crash days open flat-to-up and fall afterward. Momentum/highs/vol point the expected direction but weakly (rv20_pct, the trailing-year vol percentile, is *not* elevated on <−2% days: 47th pctile vs 59th on the merely-bad class). Day-of-week P(<−1%) ranges 4.6%(Mon)-15.1%(Thu), not actionable (~230/bucket, 10-36 hits). **FOMC**: 26.3% vs 9.3% (2.8x) but n=38 days → too few to threshold on. **Earnings** (4 names, same/next calendar day — no BMO/AMC split in the data): 15.0% vs 9.3% (n=120), a real but modest lift.

### Best single threshold, predicting close<−1% (base rate 9.87%)

| feature | direction | threshold | fires/5y | precision | recall | lift | LOYO precision |
|---|---|---|---|---|---|---|---|
| dist from 20d high | ≤ | −2.61% | 395 | 20.0% | 67.5% | 2.0x | **15.3%** |
| realized vol 20d | ≥ | 20.2% | 222 | 20.3% | 38.5% | 2.1x | 11.9% (0 fires in 2023) |
| dist from 52w high | ≤ | −11.1% | 214 | 18.2% | 43.3% | 1.9x | 15.6% (0 fires in 2 folds) |
| prior 3d return | ≤ | −1.12% | 240 | 17.9% | 36.8% | 1.8x | 14.2% |
| prior 5d return | ≤ | −0.61% | 360 | 16.7% | 51.3% | 1.7x | 13.0% |
| overnight gap | ≤ | −0.48% | 191 | 16.8% | 27.4% | 1.7x | 14.7% |
| prior 1d return | ≤ | −0.22% | 427 | 14.1% | 51.3% | 1.4x | 10.7% |

Every threshold beats the base rate and degrades under leave-one-year-out (re-fit on 4 years, tested on the 5th): **dist-from-20d-high** degrades least (20.0%→15.3%) and is the only feature firing in all five held-out years — the closest thing here to signal, though 15.3% is still only 1.5x base rate. rv20 and dist-52w-high fire zero times in at least one held-out year: part of their full-sample "best" threshold is an artifact of which years hold the tail events. On the <−2% target (25 events) no threshold fires more than 35 times, and some fire on single digits (gap ≥ 2.6% fires 5 times at "40% precision" — counting, not prediction; not LOYO-checked, too few events to hold a year out).

**Two-feature combinations did not clearly beat the best single feature**: gap≤−0.49% AND rv20_pct≥11.1% gives 15.6%/23.5% (worse than dist-20d-high alone); ret-since-open≤−0.24% AND pct-red≥50% at 10:00 gives 26.1%/45.3% (same as dist-from-VWAP alone at 10:00, 26.9%/42.7%). No pairing moved precision by more than ~1pp at matched recall.

### Intraday features at 10:00 / 10:30, predicting close<−1% (fires / precision / recall / LOYO precision)

| feature | @10:00 threshold | @10:00 | @10:30 threshold | @10:30 |
|---|---|---|---|---|
| **SPY return since open** | ≤−0.24% | 220 / 25.0% / 47.0% / 20.1% | **≤−0.50%** | **111 / 44.1% / 41.9% / 38.2%** |
| SPY dist below VWAP | ≤−0.17% | 186 / 26.9% / 42.7% / 23.8% | ≤−0.26% | 131 / 37.4% / 41.9% / 26.7% |
| mean of 4 names' return | ≤−0.90% | 119 / 28.6% / 29.1% / 22.3% | ≤−1.27% | 81 / 38.3% / 26.5% / 15.0%* |
| share of 4 names red | ≥75% | 448 / 14.1% / 53.8% / 12.7% | =100% | 203 / 21.2% / 36.8% / 17.3% |
| range-since-open / ATR20 | ≥0.34 | 497 / 13.7% / 58.1% / 11.9% | ≥0.39 | 674 / 13.4% / 76.9% / 12.0% |

\* unstable: 0% in the 2026 LOYO fold. First-15-min return (fixed by 09:45, doesn't change with the checkpoint) at ≤−0.23%: 152 fires, 23.0% precision, 29.9% recall, 18.0% LOYO.

By 10:30 the market has told you far more than at 10:00 or 09:30: SPY's own return-since-open is the strongest, most stable feature (44.1% full-sample, 38.2% LOYO — a ~4x lift surviving every held-out year) — the best early trigger here. "New session low in the trailing 15 minutes" is a weaker version: P(<−1%) 17.9% vs 6.6% (1.8x, n=347/839). On the <−2% target, ≤−1.07% fires only 13 times in 5y (53.8% precision, lift 25x) — headline-looking, but LOYO fold counts are 0/1/1/2/6/year at 0%–100% precision: a small-sample artifact, not a stable predictor, though the direction is obviously true.

**The literal −1.0% trigger**: taking the owner's phrasing literally — SPY return-since-open ≤ −1.0% at 10:30, no fitting — fires on **20 of 1,186 days (~4/year)**: **60% go on to close < −1%, 35% close < −2%, only 10% (2 days)** close back above −0.3% (the false-alarm bar, part 2). The cleanest single number here: rare, and when it fires the day is already most of the way to a tail event — though part 2 shows most of the *move* is still ahead of it.

**Calibration, plainly**: 117 <−1% events are enough to fit and sanity-check a threshold — the LOYO gap for the best ones (dist-20d-high, dist-VWAP@10:00, return-since-open@10:30) is a few points, not a collapse, that's signal. 25 <−2% events is not enough to fit anything: every "best" threshold there comes from a handful of firings and LOYO folds routinely hit zero; treat those numbers as descriptive only.

## 2. Anatomy

### Fraction of the open→close move completed by checkpoint (median [Q1, Q3])

| checkpoint | <−1% (n=117) | <−2% (n=25) |
|---|---|---|
| 10:00 | 14.6% [−3.4, 26.0] | 7.9% [−4.4, 19.2] |
| 10:30 | 25.9% [3.8, 42.5] | 19.0% [−3.0, 36.6] |
| 11:30 | 39.0% [16.6, 60.1] | 36.5% [19.2, 53.6] |
| 13:00 | 55.2% [28.3, 75.2] | 56.0% [40.4, 66.5] |
| 14:00 | 69.9% [40.3, 89.3] | 68.3% [52.9, 80.7] |
| 15:00 | 81.7% [65.2, 93.7] | 83.3% [74.4, 90.2] |

Even on the worst days the median session has done only ~19–26% of its move by 10:30, ~37–39% by 11:30: most money is made midday-to-close, so a 10:30 trigger fires with most downside still ahead.

### Session low timing, close behavior, VWAP

| | <−1% (n=117) | <−2% (n=25) |
|---|---|---|
| session low before 11:30 | 1% | 0% |
| session low 11:30–14:00 | 12% | 4% |
| session low after 14:00 | **87%** | **96%** |
| accelerated (close within 0.3% of the low) | 67% | 76% |
| V-reversed (close recovers >50% of the drop) | **1%** | **0%** |
| VWAP retests/day (mean, median) | 7.0, 6.0 | 4.7, 4.0 |
| retest resolved above VWAP 30min later | 23% | 26% |
| retest resolved above VWAP 60min later | 20% | 21% |

The session low is almost always made in the last two hours and the close is almost always near it — V-reversals are essentially absent (1/117, 0/25). A VWAP retest resolves back above VWAP only ~20–26% of the time within an hour, i.e. these are trend days once the morning threshold has fired, not mean-reverting.

### The four names vs SPY (beta = name return / SPY return, end of day)

| name | beta, <−1% days | led (of 117) | beta, <−2% days | led (of 25) |
|---|---|---|---|---|
| NVDA | 2.01 | 65 (56%) | 1.94 | 18 (72%) |
| AMZN | 1.53 | 32 (27%) | 1.45 | 6 (24%) |
| MSFT | 1.21 | 8 (7%) | 1.06 | 0 |
| AAPL | 1.09 | 12 (10%) | 1.11 | 1 (4%) |

NVDA leads most tail days at ~2x SPY beta; AAPL/MSFT are followers near SPY-beta — the same asymmetry `iex_v18` already shows informally (NVDA is its biggest single-name mover on these days).

### The 25 worst days

None V-reversed (all "no", per above). `low` = session-low time (all `pm`, i.e. ≥14:00, except 2022-12-13 at 12:50). `accel` = closed within 0.3% of the low.

| date | oc% | low | accel | VWAP RT | lead | v18 trades | v18 pnl |
|---|---|---|---|---|---|---|---|
| 2025-04-08 | −4.86 | 15:49 | no | 2 | AMZN | 0 | 0.00 |
| 2025-04-04 | −3.38 | 15:57 | yes | 6 | NVDA | 1 | +100.87 |
| 2022-08-26 | −3.36 | 15:59 | yes | 2 | NVDA | 0 | 0.00 |
| 2025-11-20 | −3.05 | 15:59 | yes | 2 | NVDA | 0 | 0.00 |
| 2022-10-14 | −2.95 | 15:59 | yes | 1 | NVDA | 0 | 0.00 |
| 2024-12-18 | −2.91 | 15:59 | yes | 0 | AMZN | 1 | −23.35 |
| 2022-05-18 | −2.87 | 15:54 | no | 6 | NVDA | 0 | 0.00 |
| 2025-10-10 | −2.81 | 15:56 | yes | 1 | NVDA | 0 | 0.00 |
| 2022-03-07 | −2.79 | 15:59 | yes | 3 | NVDA | 0 | 0.00 |
| 2022-04-29 | −2.76 | 15:58 | yes | 7 | NVDA | 1 | −6.55 |
| 2022-04-22 | −2.47 | 15:59 | yes | 5 | NVDA | 0 | 0.00 |
| 2022-02-23 | −2.44 | 15:58 | yes | 5 | NVDA | 0 | 0.00 |
| 2022-05-05 | −2.44 | 15:31 | no | 0 | AMZN | 1 | +97.27 |
| 2022-04-21 | −2.39 | 15:50 | yes | 4 | NVDA | 0 | 0.00 |
| 2022-06-28 | −2.39 | 15:51 | yes | 1 | AMZN | 0 | 0.00 |
| 2022-11-02 | −2.33 | 15:54 | yes | 12 | AMZN | 0 | 0.00 |
| 2022-04-26 | −2.27 | 15:59 | yes | 7 | NVDA | 0 | 0.00 |
| 2022-09-21 | −2.26 | 15:59 | yes | 13 | AMZN | 0 | 0.00 |
| 2022-09-13 | −2.18 | 15:48 | yes | 4 | NVDA | 2 | +29.64 |
| 2024-04-15 | −2.07 | 15:19 | yes | 4 | NVDA | 0 | 0.00 |
| 2025-03-03 | −2.05 | 15:44 | no | 16 | NVDA | 0 | 0.00 |
| 2023-03-09 | −2.02 | 15:22 | yes | 4 | NVDA | 0 | 0.00 |
| 2022-09-02 | −2.02 | 15:45 | no | 6 | NVDA | 0 | 0.00 |
| 2022-12-13 | −2.02 | 12:50 | no | 2 | AAPL | 0 | 0.00 |
| 2024-04-04 | −2.01 | 15:41 | yes | 4 | NVDA | 0 | 0.00 |

iex_v18 traded only 6 of these 25 days — its morning-only, SPY-flat-band windows are structurally absent for most worst days, the gap the (rejected) stress-mode study tried to close.

### False alarms: <−1% by 10:30 but close > −0.3%

Only **2 days in 5 years** meet this bar (2022-05-24, 2024-09-11), both V-reversals with no acceleration — the fraction-of-move metric is meaningless here since the open→close move is near zero. iex_v18 still made +$32.29 on 2 trades: a morning-flat short doesn't need the close down, only the entry window flat.

## 3. Synthesis

**Best early trigger**: SPY return-since-open ≤ −1.0% by 10:30 ET — fires ~4 days/year, 60% close <−1% (35% <−2%, 10% false alarm). A looser F1-fit version (≤−0.50%) fires ~22 days/year at 44.1% precision (38.2% LOYO) — frequency/precision trades off roughly linearly between these two points. Nothing at 09:30 comes close (best precursor, dist-from-20d-high, tops out at 15.3% LOYO, 1.5x base rate). **VIX is not in this dataset**, the obvious missing precursor; realized vol and its percentile are weak substitutes.

**What the ride looks like**: at the moment a 10:30 trigger fires, only ~19–26% of the eventual move has happened; the low comes after 14:00 on 87–96% of tail days; the close is within 0.3% of the low 67–76% of the time; V-reversals are almost nonexistent (1/142); a VWAP retest resolves back above VWAP within an hour only ~20–26% of the time. None of this argues for exiting early — it argues for holding into the close (already what `session_close`/`max_hold_ms` do); the finding is about *when to enter*, not how to manage an open position.

**Against the earlier studies**: reproduces the stress study's core claim — "on an intraday −1% touch the continuation happens on roughly half the days" (`stress/2026-09-26_stress_mode.md` §1) matches the 44.1%/38.2% precision found here at the loose (−0.50%) 10:30 threshold, and its finding that only the deepest trigger (SPY ≤ −1.5%, `s15_core`) was clean matches precision rising sharply with depth (16.8%→44.1%→60%). Stress mode already built and gated this shape and **failed the promotion gate** (`s15_core`: +274/23 trades/PF 3.3, 18 of 23 in 2022, nothing 2023/2024 — "a small, safe, additive rule, not a stress mode", §4). New here: the exact −1.0% cutoff's false-alarm rate is genuinely low (10%, 2/20 — entry isn't the weak link), and NVDA leads 56–72% of tail days at ~2x SPY beta, unused by either study. Frequency, 2022 concentration, and weak 09:30 precursors all corroborate the earlier work.

**Not there**: no VIX; FOMC/earnings flags are real but too rare to threshold on; no combo beat the best feature; <−2% (25 events) can't support a fitted threshold at all.

**Verdict**: not worth a new standalone strategy study. Stress mode tested this trigger shape end-to-end and rejected it; this confirms the same ceiling from the precursor side (no early warning beats ~1.5x lift) and floor from the trigger side (clean, but 4x/year). NVDA-relative sizing/filtering, not SPY's absolute level, is a plausible next cell if stress mode is revisited — not a new thread on its own. 