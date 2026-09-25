# Study C — longs only in a confirmed up-regime (IEX cache, 2022-01-01..2026-09-10)

**Question.** The long book (5m thrust / strong core / candle reversal, disabled since v15) is believed to be a
bet on positive morning drift. Does gating it on an up-regime decided at the open — from SPY daily closes
through the prior day, no look-ahead — turn it positive, and how much volume does it add on top of the v18 shorts?

**Design.** Baseline `iex_v18` (live v18 shorts, `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`,
3 bps + $0.005 per leg). Every cell keeps the promoted shorts and adds the three *filtered* long windows from
`research/entries/variants/long_filters.json` (SPY session return within ±0.2 %, VPIN ≥ 0.217). Regime cells add an
`event_calendar` instance `cal_<k>` (score −1.0 on regime days) and the condition `indicator_max cal_<k> ≤ −0.5` on
each long window (`research/volume/rl_<k>.json`). Regimes: sma50 (SPY > 50d SMA), sma20 (> 20d SMA), ret20 (20d
return > 0), lowvol (20d realized vol < trailing-250d median; needs a year of history so 2022 is empty), sma50_lowvol
(both). Follow-ups on the best regime (sma20): candle window removed; VPIN floor removed from the longs only; SPY band
widened to ±0.3 % (cross_1m in [−0.6, 0.6]) on the longs only. Bar to pass: combined PF ≥ 1.3, ≥ 4/5 positive years,
n > 407, long subset positive with PF ≥ 1.2 and no year worse than −300.

## 1. Grid (combined book; per-year cells are P&L (PF); Δ vs iex_v18)

| tag | regime | 5y P&L | n | win % | PF | max DD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | Δn / ΔP&L vs v18 | LONG 5y P&L / n / PF / +yrs |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| iex_v18 | — (baseline, shorts only) | +2,154 | 407 | 39 | 1.48 | -526 | +1,565 (2.98) | -181 (0.83) | -93 (0.92) | +451 (1.57) | +413 (1.60) | 3 | — | — |
| rl_none | none | +2,926 | 1656 | 35 | 1.16 | -1,215 | +2,509 (1.77) | -151 (0.96) | -2 (1.00) | -204 (0.94) | +775 (1.29) | 2 | +1249 / +772 | +775 / 1249 / 1.06 / 4 |
| rl_sma50 | sma50 | +1,688 | 1325 | 35 | 1.12 | -1,080 | +1,532 (1.91) | -487 (0.86) | +154 (1.04) | -11 (1.00) | +499 (1.21) | 3 | +918 / -466 | -470 / 918 / 0.95 / 2 |
| rl_sma20 | sma20 | +2,598 | 1321 | 36 | 1.18 | -909 | +2,148 (2.17) | -207 (0.94) | +375 (1.10) | -305 (0.90) | +586 (1.27) | 3 | +914 / +444 | +444 / 914 / 1.05 / 3 |
| rl_ret20 | ret20 | +1,778 | 1288 | 35 | 1.13 | -960 | +1,833 (2.05) | -529 (0.83) | +222 (1.06) | -127 (0.96) | +379 (1.17) | 3 | +881 / -376 | -379 / 881 / 0.96 / 2 |
| rl_lowvol | lowvol | +1,687 | 1023 | 35 | 1.15 | -956 | +1,565 (2.98) | -487 (0.86) | +443 (1.16) | -145 (0.93) | +311 (1.18) | 3 | +616 / -467 | -465 / 616 / 0.93 / 1 |
| rl_sma50_lowvol | sma50_lowvol | +1,791 | 941 | 35 | 1.18 | -697 | +1,565 (2.98) | -505 (0.83) | +486 (1.18) | +39 (1.02) | +206 (1.13) | 4 | +534 / -363 | -366 / 534 / 0.93 / 1 |
| rl_sma20_nocandle | sma20, no candle window | +2,555 | 1287 | 35 | 1.18 | -859 | +2,174 (2.21) | -246 (0.92) | +332 (1.09) | -257 (0.92) | +553 (1.26) | 3 | +880 / +401 | +405 / 880 / 1.04 / 3 |
| rl_sma20_novpin | sma20, no VPIN on longs | +1,546 | 1906 | 33 | 1.07 | -2,334 | +2,355 (1.89) | -438 (0.91) | -502 (0.91) | -998 (0.79) | +1,129 (1.37) | 2 | +1499 / -608 | -608 / 1499 / 0.96 / 2 |
| rl_sma20_spy03 | sma20, SPY ±0.3 % on longs | +2,203 | 1418 | 35 | 1.15 | -992 | +1,970 (1.93) | -311 (0.91) | +329 (1.09) | -384 (0.88) | +598 (1.25) | 3 | +1011 / +49 | +50 / 1011 / 1.00 / 3 |

## 2. Long vs short subsets (per year: P&L / n / PF)

| cell | subset | 5y P&L | n | PF | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|---|---|---|---|
| iex_v18 | short | +2,154 | 407 | 1.48 | +1,565 / 91 / 2.98 | -181 / 80 / 0.83 | -93 / 90 / 0.92 | +451 / 77 / 1.57 | +413 / 69 / 1.60 |
| rl_none | long | +775 | 1249 | 1.06 | +942 / 185 / 1.38 | +29 / 298 / 1.01 | +95 / 319 / 1.03 | -652 / 255 / 0.77 | +360 / 192 / 1.18 |
| rl_none | short | +2,151 | 407 | 1.47 | +1,567 / 91 / 2.99 | -181 / 80 / 0.83 | -97 / 90 / 0.92 | +447 / 77 / 1.56 | +414 / 69 / 1.60 |
| rl_sma50 | long | -470 | 918 | 0.95 | -32 / 62 / 0.96 | -307 / 224 / 0.88 | +246 / 273 / 1.09 | -458 / 207 / 0.79 | +81 / 152 / 1.05 |
| rl_sma50 | short | +2,158 | 407 | 1.48 | +1,564 / 91 / 2.98 | -180 / 80 / 0.83 | -93 / 90 / 0.92 | +448 / 77 / 1.56 | +419 / 69 / 1.61 |
| rl_sma20 | long | +444 | 914 | 1.05 | +583 / 90 / 1.56 | -25 / 219 / 0.99 | +470 / 260 / 1.19 | -753 / 212 / 0.67 | +169 / 133 / 1.12 |
| rl_sma20 | short | +2,153 | 407 | 1.47 | +1,565 / 91 / 2.98 | -182 / 80 / 0.83 | -94 / 90 / 0.92 | +448 / 77 / 1.56 | +417 / 69 / 1.60 |
| rl_ret20 | long | -379 | 881 | 0.96 | +268 / 79 / 1.28 | -349 / 199 / 0.84 | +315 / 270 / 1.12 | -575 / 197 / 0.72 | -38 / 136 / 0.97 |
| rl_ret20 | short | +2,157 | 407 | 1.48 | +1,565 / 91 / 2.98 | -180 / 80 / 0.83 | -93 / 90 / 0.92 | +448 / 77 / 1.56 | +417 / 69 / 1.60 |
| rl_lowvol | long | -465 | 616 | 0.93 | +0 / 0 / 0.00 | -306 / 235 / 0.88 | +540 / 173 / 1.36 | -595 / 125 / 0.57 | -103 / 83 / 0.90 |
| rl_lowvol | short | +2,152 | 407 | 1.47 | +1,565 / 91 / 2.98 | -181 / 80 / 0.83 | -97 / 90 / 0.92 | +451 / 77 / 1.57 | +414 / 69 / 1.60 |
| rl_sma50_lowvol | long | -366 | 534 | 0.93 | +0 / 0 / 0.00 | -324 / 174 / 0.83 | +583 / 172 / 1.41 | -412 / 121 / 0.64 | -213 / 67 / 0.75 |
| rl_sma50_lowvol | short | +2,156 | 407 | 1.48 | +1,565 / 91 / 2.98 | -181 / 80 / 0.83 | -97 / 90 / 0.92 | +451 / 77 / 1.57 | +419 / 69 / 1.61 |
| rl_sma20_nocandle | long | +405 | 880 | 1.04 | +609 / 87 / 1.60 | -64 / 209 / 0.97 | +428 / 250 / 1.18 | -703 / 203 / 0.69 | +135 / 131 / 1.09 |
| rl_sma20_nocandle | short | +2,150 | 407 | 1.47 | +1,565 / 91 / 2.98 | -182 / 80 / 0.83 | -96 / 90 / 0.92 | +445 / 77 / 1.56 | +417 / 69 / 1.60 |
| rl_sma20_novpin | long | -608 | 1499 | 0.96 | +787 / 172 / 1.42 | -258 / 343 / 0.93 | -404 / 426 / 0.91 | -1,443 / 340 / 0.63 | +709 / 218 / 1.30 |
| rl_sma20_novpin | short | +2,154 | 407 | 1.48 | +1,568 / 91 / 2.99 | -180 / 80 / 0.83 | -98 / 90 / 0.92 | +445 / 77 / 1.56 | +420 / 69 / 1.61 |
| rl_sma20_spy03 | long | +50 | 1011 | 1.00 | +408 / 106 / 1.31 | -129 / 244 / 0.95 | +423 / 276 / 1.16 | -833 / 230 / 0.66 | +181 / 155 / 1.11 |
| rl_sma20_spy03 | short | +2,152 | 407 | 1.47 | +1,562 / 91 / 2.98 | -182 / 80 / 0.83 | -94 / 90 / 0.92 | +450 / 77 / 1.56 | +417 / 69 / 1.60 |

The short subset is unchanged in every cell: 407 trades, 5y P&L within ±4 of iex_v18 (+2,154), same per-year
numbers to the dollar. The 3-position cap never displaced a short.

### Long subset per window (per year: P&L / n)

| cell | long window | 5y P&L | n | PF | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|---|---|---|---|
| rl_none | 5m thrust +filters | +313 | 146 | 1.20 | +715 / 28 | -253 / 22 | -35 / 42 | -45 / 32 | -70 / 22 |
| rl_none | candle reversal +filters | +190 | 119 | 1.14 | +28 / 17 | +244 / 36 | +105 / 32 | -199 / 25 | +11 / 9 |
| rl_none | strong core +filters | +272 | 984 | 1.03 | +199 / 140 | +38 / 240 | +24 / 245 | -408 / 198 | +419 / 161 |
| rl_sma50 | 5m thrust +filters | -361 | 85 | 0.60 | -82 / 1 | -120 / 17 | +58 / 31 | -11 / 21 | -206 / 15 |
| rl_sma50 | candle reversal +filters | +107 | 88 | 1.12 | -26 / 4 | +82 / 28 | +48 / 29 | -34 / 20 | +38 / 7 |
| rl_sma50 | strong core +filters | -217 | 745 | 0.97 | +76 / 57 | -269 / 179 | +140 / 213 | -412 / 166 | +249 / 130 |
| rl_sma20 | 5m thrust +filters | +71 | 70 | 1.12 | +196 / 3 | -12 / 11 | +104 / 29 | +14 / 15 | -232 / 12 |
| rl_sma20 | candle reversal +filters | +91 | 89 | 1.10 | -31 / 5 | +62 / 27 | +91 / 28 | -113 / 23 | +83 / 6 |
| rl_sma20 | strong core +filters | +282 | 755 | 1.04 | +417 / 82 | -75 / 181 | +274 / 203 | -654 / 174 | +319 / 115 |
| rl_ret20 | 5m thrust +filters | -177 | 87 | 0.79 | +90 / 3 | -77 / 15 | +31 / 32 | -6 / 20 | -216 / 17 |
| rl_ret20 | candle reversal +filters | +72 | 78 | 1.10 | +8 / 5 | -75 / 24 | +80 / 27 | -33 / 19 | +93 / 3 |
| rl_ret20 | strong core +filters | -274 | 716 | 0.96 | +170 / 71 | -197 / 160 | +204 / 211 | -536 / 158 | +84 / 116 |
| rl_lowvol | 5m thrust +filters | -509 | 58 | 0.33 | +0 / 0 | -297 / 19 | -101 / 19 | -132 / 15 | +20 / 5 |
| rl_lowvol | candle reversal +filters | +140 | 63 | 1.21 | +0 / 0 | +166 / 29 | +74 / 20 | -46 / 11 | -54 / 3 |
| rl_lowvol | strong core +filters | -95 | 495 | 0.98 | +0 / 0 | -176 / 187 | +566 / 134 | -417 / 99 | -69 / 75 |
| rl_sma50_lowvol | 5m thrust +filters | -262 | 48 | 0.40 | +0 / 0 | -161 / 15 | -58 / 18 | +3 / 13 | -47 / 2 |
| rl_sma50_lowvol | candle reversal +filters | +83 | 56 | 1.16 | +0 / 0 | +49 / 23 | +74 / 20 | -46 / 11 | +6 / 2 |
| rl_sma50_lowvol | strong core +filters | -187 | 430 | 0.96 | +0 / 0 | -212 / 136 | +566 / 134 | -369 / 97 | -171 / 63 |
| rl_sma20_nocandle | 5m thrust +filters | +69 | 71 | 1.12 | +196 / 3 | -12 / 11 | +104 / 29 | +13 / 16 | -232 / 12 |
| rl_sma20_nocandle | strong core +filters | +336 | 809 | 1.04 | +412 / 84 | -52 / 198 | +324 / 221 | -716 / 187 | +368 / 119 |
| rl_sma20_novpin | 5m thrust +filters | +402 | 111 | 1.42 | +369 / 15 | +68 / 21 | -10 / 42 | -154 / 20 | +129 / 13 |
| rl_sma20_novpin | candle reversal +filters | -203 | 234 | 0.91 | +45 / 21 | +164 / 57 | -8 / 69 | -403 / 59 | +1 / 28 |
| rl_sma20_novpin | strong core +filters | -808 | 1154 | 0.94 | +374 / 136 | -489 / 265 | -386 / 315 | -886 / 261 | +579 / 177 |
| rl_sma20_spy03 | 5m thrust +filters | +30 | 78 | 1.05 | +160 / 5 | -10 / 12 | +123 / 30 | -8 / 16 | -234 / 15 |
| rl_sma20_spy03 | candle reversal +filters | +23 | 100 | 1.02 | -35 / 7 | +28 / 31 | +82 / 30 | -135 / 26 | +83 / 6 |
| rl_sma20_spy03 | strong core +filters | -3 | 833 | 1.00 | +284 / 94 | -147 / 201 | +218 / 216 | -691 / 188 | +332 / 134 |

### Regime coverage (days flagged) vs long trades taken (sessions 250 / 250 / 252 / 249 / 183)

| regime | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---|---|---|---|---|
| rl_none | all d / 185 t | all d / 298 t | all d / 319 t | all d / 255 t | all d / 192 t |
| rl_sma50 | 80 d / 62 t | 175 d / 224 t | 216 d / 273 t | 185 d / 207 t | 134 d / 152 t |
| rl_sma20 | 99 d / 90 t | 169 d / 219 t | 201 d / 260 t | 176 d / 212 t | 109 d / 133 t |
| rl_ret20 | 93 d / 79 t | 158 d / 199 t | 212 d / 270 t | 174 d / 197 t | 112 d / 136 t |
| rl_lowvol | 0 d / 0 t | 212 d / 235 t | 128 d / 173 t | 118 d / 125 t | 84 d / 83 t |
| rl_sma50_lowvol | 0 d / 0 t | 148 d / 174 t | 125 d / 172 t | 112 d / 121 t | 67 d / 67 t |
| rl_sma20_nocandle | 99 d / 87 t | 169 d / 209 t | 201 d / 250 t | 176 d / 203 t | 109 d / 131 t |
| rl_sma20_novpin | 99 d / 172 t | 169 d / 343 t | 201 d / 426 t | 176 d / 340 t | 109 d / 218 t |
| rl_sma20_spy03 | 99 d / 106 t | 169 d / 244 t | 201 d / 276 t | 176 d / 230 t | 109 d / 155 t |


### What each gate keeps vs removes (rl_none long trades split by the regime flag on their entry date)

| regime | kept (regime days) | removed (other days) |
|---|---|---|
| sma50 | −460 / 918 / PF 0.95 | +1,235 / 331 / PF 1.32 |
| sma20 | +457 / 914 / PF 1.05 | +318 / 335 / PF 1.07 |
| ret20 | −364 / 881 / PF 0.96 | +1,139 / 368 / PF 1.26 |
| lowvol | −463 / 616 / PF 0.93 | +1,238 / 633 / PF 1.17 |
| sma50_lowvol | −364 / 534 / PF 0.93 | +1,139 / 715 / PF 1.13 |

The "kept" columns reproduce the gated sweeps to within ±10 (e.g. sma50 −460 vs rl_sma50 long −470), which confirms
the calendar gate did what it was meant to and that the cap/ordering effects are negligible.

## 3. Sanity checks

- All 9 cells: 260 / 260 / 262 / 261 / 181 days replayed for 2022..2026, `0 skipped`; no non-holiday "skipped:" lines in
  any `logs/sweeps/rl_*.log`.
- `entry_reason` shows exactly the expected names: `window:5m thrust +filters`, `window:candle reversal +filters`,
  `window:strong core +filters` plus the promoted `window:5m thrust short` / `window:strong core short` (360 + 47 in every
  cell, identical to iex_v18). `direction` column is capitalised (`Long` / `Short`); subsets were split on it, not on
  window names.
- Regime day counts in `regime_days.csv` match the brief (sma50 80/175/216/185/134 etc.). lowvol / sma50_lowvol take
  zero long trades in 2022 by construction, so their 2022 row equals iex_v18 exactly.
- Nothing odd: in 2022 sma20 flagged 99 days and the cell took 90 long trades on them — those are bear-market rally
  weeks (Mar, Jul, Oct 2022), which is where the ungated long book made most of its 2022 money.
- Machine load was ~20 on 12 cores (other studies), so each sweep took ~12 min instead of 3–8; results are unaffected.

## 4. Interpretation

- **On IEX the filtered long book is not the −1,710 loser it was on SIP**: ungated it makes +775 on 1,249 trades, but at
  PF 1.06, +0.62 per trade (shorts: +5.28 per trade) and a −1,387 long-only drawdown. That is noise-level edge, and it
  is one feed away from being negative — consistent with §14 feed-mismatch findings.
- **Up-regime gating does not help; four of five regimes make the long book negative.** The gates keep the worse half:
  on sma50 / ret20 / lowvol days the longs lose (PF 0.93–0.96) while the non-regime days they discard are the profitable
  part (+1,139..+1,238, PF 1.13–1.32). The long windows earn on oversold bounces inside down-trends (2022 Mar/Jul/Oct),
  not on drift — the premise of the study is inverted by the data.
- **sma20 is the only regime whose kept set is positive (+457 / PF 1.05), and it is still worse than not gating**
  (+775). It shaves the drawdown (−909 vs −1,215 combined) by removing 335 trades, not by improving the ones it keeps.
- **2025 is a hole no regime fixes**: long subset −652 ungated, −412..−753 gated. SPY sat above its 20/50-day SMAs for
  most of May–Aug 2025 while the four mega-caps chopped; the monthly long P&L shows −227 / −57 / −200 / −204 for
  May–Aug. A daily SPY regime carries no information about that.
- **The VPIN ≥ 0.217 floor is doing real work on the long side**: dropping it adds 585 long trades worth about −1,050
  (long −608, PF 0.96, DD −2,714). Widening the SPY band to ±0.3 % adds 97 trades worth about −400 (long +50, PF 1.00).
  Both v17 filters should stay if the longs are ever revisited. Removing the candle window is neutral (its setups are
  re-taken by strong core: 809 vs 755 trades, long +405 vs +444).
- **strong core is churn**: 755–984 of the long trades, PF 1.03–1.04 in every cell; 5m thrust and candle reversal are
  small (70–146 trades) with no stable sign across years.
- **Where the long P&L comes from** (rl_none): MaxHoldTimeout +2,937 (892 trades) and SessionClose +1,592 (98) vs
  ScoreExit −1,798 (36 trades, ≈ −50 each) and HardStop −940 (10). NVDA +1,664 carries the book; AMZN −804, MSFT −591.
  A score-exit rule that is tuned for shorts is the most obvious next lever if longs are pursued — out of scope here.
- Volume: the best regime adds 914 trades (3.2× iex_v18) for +444, drops combined PF from 1.48 to 1.18 and doubles max
  DD (−526 → −909). The volume is real; the edge is not.

## 5. Verdict

**No cell passes.** Best regime cell is `rl_sma20`: combined +2,598 / 1,321 trades / PF 1.18 / 3 positive years;
long subset +444 / 914 / PF 1.05 with 2025 at −753. It fails combined PF (1.18 < 1.3), positive years (3 < 4), long PF
(1.05 < 1.2) and the −300 year floor; it passes only the trade-count criterion. The ungated reference `rl_none` is the
best long book in the study (+775, PF 1.06, 4/5 long years positive) and fails on PF, 2 positive combined years, and
2025 −652. Regime gating at the open is not the missing ingredient for the long book; the long book's small IEX edge
lives in counter-trend bounces, which is the opposite of what a regime gate keeps. Recommend leaving the longs disabled
and, if they are revisited, testing exit handling (ScoreExit) and per-ticker inclusion (NVDA/AAPL only) rather than
regime gates.

## Commands

```bash
B="--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY"
LF=research/entries/variants/long_filters.json
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rl_none $B --patch-json $LF
for k in sma50 sma20 ret20 lowvol sma50_lowvol; do
  BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rl_$k $B --patch-json research/regime/regime_$k.json --patch-json research/volume/rl_$k.json
done
for f in nocandle novpin spy03; do
  BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh rl_sma20_$f $B --patch-json research/regime/regime_sma20.json --patch-json research/volume/rl_sma20_$f.json
done
python3 research/entries/summarize.py iex_v18 rl_none rl_sma50 rl_sma20 rl_ret20 rl_lowvol rl_sma50_lowvol rl_sma20_nocandle rl_sma20_novpin rl_sma20_spy03 --by-window
# rl_<k>.json = long_filters.json windows + {"type":"indicator_max","instance_id":"cal_<k>","max_score":-0.5} on each;
# rl_sma20_nocandle/novpin/spy03.json derived from rl_sma20.json (window_candle_reversal_x dropped / vpin_1m.raw_vpin
# condition dropped / cross_1m bounds set to ±0.6).
```
