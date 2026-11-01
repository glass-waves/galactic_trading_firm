# indicators desk (LAB_AGENT=indicators), 2026-11-01

Angle: the 2026-10-29 opt-in indicator fixes (standard supertrend, signed ADX, MACD `normalization_pct`), an IEX-native
VPIN neighbourhood, warm-up 12 vs 8, and re-tuning the 1h gates once the 1h sum is directional.
Ledger: **33 distinct specs, 41 runs** (`lab.py cells --agent indicators`). No holdout dates touched, no engine/DB/pipeline edits.
All numbers are from `research/lab/runs/indicators__*`. "2023_25h1" is the post-2022 slice of the train trades (2023-01-01 → 2025-06-30).
Values in parentheses in the screen column are the 2024h2+2025h1 half rows of a train run (the cell was run straight on train).

## headline

1. **The fixes as specified lose.** RQ-13 `dir1h` (standard supertrend + signed ADX) is +767 / 548 / PF 1.12 on train,
   2023 −272, 2024 −392, dd −957 (v18: +1179 / 324 / 1.32). MACD pct-normalisation is flat to negative at every k. VPIN
   `bucket_divisor` does nothing (dead code), lookback 40 swaps 40 % of trades for the same P&L, and warm-up 12 swaps 3 trades.
2. **Signed ADX is a parity lever.** It adds about 10 % more trades at the same quality, mostly NVDA. On v18 it is +1347/331/1.36
   (paired vs v18 +168, P(diff≤0) 0.28). On r21_v020 it is +1562/282/1.51 (vs +1530/250/1.58, P 0.49). Its neighbourhood
   is unstable: ADX period 10 / 14 / 20 gives PF 1.34 / 1.51 / 1.72.
3. **The useful finding came from the control.** Turning the legacy ADX term *off* (`enabled:false`) on the row-21 family is
   +1579 / 236 / PF 1.67 on train. It is positive in all four years, has dd −227, and on 2023_25h1 shows +769 / 158 / 1.43 vs +594 / 173 / 1.31.
   The result holds across every neighbour tried: thrust 1h −0.10 / −0.15 / −0.20 give PF 1.69 / 1.67 / 1.74, and VPIN 0.217 gives 1.66.
   Against the legacy twin at the same gate (thrust −0.10): +478 on train, P(diff≤0) 0.11; +446 on 2023_25h1, P 0.034.

## mechanisms (numbers, not adjectives)

Tick sample: 73 train days (every 12th session), 35,332 morning bars on the 4 names. Extra weight-0 indicators were added to
compute both variants on the same bars (diagnostic replays, not lab cells). "Thrusty" bars are 5m ≤ −0.5 and composite ≤ −0.35.

- **Legacy 1h composition.** The legacy supertrend is never negative: 0.0 % of bars are < 0, mean +0.47, so it adds +0.14 to the 1h.
  Legacy ADX (`adx/50 − 1`) is negative on 85 % of bars, mean −0.27, so it adds −0.05. Net offset +0.09.
  With the legacy indicators, 1h ≤ −0.4 happens on 0.2 % of bars. That is why strong core had only 24 trades on train.
- **Why dir1h fails.** With both fixes, 1h ≤ −0.4 happens on 15 % of bars and on 57 % of thrusty bars. Strong core goes from
  24 to 425 trades at PF 1.07. The thrust window is starved: `timescale_lag` needs 5m ≤ 1h − 0.1, and once the 1h carries the same
  down-move it no longer lags (300 → 123 trades). The 2024h2_25h1 diff: +128 core trades added at +235, 66 thrust trades removed at −106.
  The screen looked good (+482 vs +59) only because base's thrust trades lost in that year. On train, the added trades by year are
  22 +564 / 23 −447 / 24 −699 / 25 +322. Re-tuning core to −0.6 / −0.8 does not fix it: dir1h_c60 is +561/101/1.47 on the screen
  but +1048/397/1.22 on train (23 −331, 24 −376). This is a regime fit.
  Bucketed forward returns on thrusty bars show no gradient in the directional 1h: 1h ≤ −0.4 gives +5.2 bp of 30-min drift against the short.
  The textbook supertrend makes the 1h a lagged copy of the 5m move.
- **Why turning ADX off works (row 21's thrust gate 1h ≤ −0.15).** Split thrusty bars by 1h ADX level:

  | ADX | thrusty bars | pass, legacy | 30-min fwd of passers | pass, ADX off | 30-min fwd of passers |
  |---|---|---|---|---|---|
  | < 20 | 93 | 54 | +30.4 bp | 14 | +76.4 bp |
  | 20–30 | 586 | 437 | +5.8 bp | 301 | +5.6 bp |
  | ≥ 30 | 1130 | 582 | −16.5 bp | 545 | −18.0 bp |

  (+ = against the short.) The legacy term subtracts 0.06–0.12 from the 1h exactly in low-ADX (choppy) hours, which lets
  chop-hour thrusts through a gate meant to require a down-sloping hour. Turning it off removes most of those passes and keeps
  the trend-hour passes. Signed ADX does the same thing in chop (10 of 93 pass) but also adds trend-hour passes (615 vs 545).
  Those added trades are where its NVDA gain (+408 on r21) and AAPL loss (−214) come from.
  The lab diff r21_v020_noadx vs r21_v020: 73 trades added at +178, 87 removed at +136. Removed by year: 2023 −77, 2024 −133.
  Added: 2022 +238. The thrust-am window goes 150 → 116 trades and PF 1.44 → 1.75.
  This is not a plain threshold shift. The legacy gate at −0.10 gives 307 trades at PF 1.46. ADX-off at −0.10 gives 287 at PF 1.69.
- **Bollinger bandwidth is the opposite case, so this is not "drop non-directional terms".** bb_bw is a non-directional
  *positive* offset (mean +0.45 × 0.1). Removing it as well (noadx_nobbw) floods strong core with 164 trades at −269
  (total +524/376/1.12). The ADX result is specifically about a term that pushes the 1h *down* in chop.
- **On v18 the effect is small** (v18_noadx +1282/308/1.35, 2025h1 −37). v18's thrust gate is 1h ≤ 0, where a −0.05
  offset matters less than at −0.15. That fits the mechanism. It also means the gain depends on a gate that was itself chosen by search (row 21).
- **MACD pct.** Trades churn 45–65 % at k 0.003 / 0.004 with P&L ±40. Larger k gives monotonically fewer trades
  (117 → 71) and lower P&L (k 0.005 −97, k 0.007 −34 on the screen). In the 73-day sample, dollar MACD clamps on 7.8 % of
  AMZN 5m bars and 4.8 % of NVDA; pct 0.005 clamps on < 1.1 %. The "48 % of P&L from pre-split AMZN/NVDA" is sensitivity, not edge,
  and removing it removes trades of the same quality. Not taken to train.
- **VPIN.** `bucket_divisor` only feeds an epsilon check (crates/indicators/src/custom/vpin.rs:151), so 0.8 reproduces base to the cent.
  Lookback is the only real knob. Percentile-matched floors (82nd pctile in the sample: lb10 0.285, lb20 0.217, lb30 0.190, lb40 0.174):
  lb40 on train is +1197/294/1.36 vs +1179/324/1.32, with 95 trades added at +816 and 125 removed at +803. Pure churn, no lever.
- **warm-up 12.** 3 trades swapped, P&L identical on the screen (+59 / 97). Not run further.

## every cell

| cell | change | screen 2024h2_25h1 P&L/n/PF | train P&L / n / PF | dd | sign-flip p | ex-best-yr PF | train per year 22/23/24/25h1 | 2023_25h1 P&L/n/PF |
|---|---|---|---|---|---|---|---|---|
| v18 | promoted row 12 (baseline) | +59/97/1.05 | +1179 / 324 / 1.32 | -469 | 0.061 | 1.031 | +1089 / -3 / -17 / +110 | +90/237/1.03 |
| r21_v020 | meta: row 21 sqrt band + thrust 1h≤−0.15, VPIN 0.20 (comparator) | (+431/69) | +1530 / 250 / 1.58 | -451 | 0.006 | 1.313 | +936 / +136 / +166 / +292 | +594/173/1.31 |
| cand45_r21 | meta: row 21, VPIN 0.217 (comparator) | (+392/65) | +1317 / 233 / 1.54 | -344 | 0.015 | 1.285 | +806 / +125 / +81 / +306 | +512/162/1.29 |
| scaffold | v18 windows re-created in place | 2025h1 +110/45/1.20 (= base) | – | | | | | |
| vpin_bd08 | VPIN bucket_divisor 0.8 | 2025h1 +110/45/1.20 (= base) | – | | | | | |
| st_std | (a) supertrend standard | +435/159/1.24 | – | | | | | |
| adx_signed | (b) ADX signed | +377/96/1.35 | +1347 / 331 / 1.36 | -475 | 0.041 | 1.132 | +971 / -45 / +224 / +197 | +376/236/1.13 |
| dir1h | (a)+(b), v18 gates | +482/154/1.28 | +767 / 548 / 1.12 | -957 | 0.220 | 0.95 | +997 / -272 / -392 / +435 | -230/381/0.95 |
| dir1h_c60 | dir1h, core 1h≤−0.6 | +561/101/1.47 | +1048 / 397 / 1.22 | -911 | 0.114 | 0.96 | +1186 / -331 / -376 / +569 | -138/265/0.96 |
| dir1h_c80 | dir1h, core 1h≤−0.8 | +140/56/1.19 | – | | | | | |
| adx_signed_t00_c30 | signed ADX, core 1h≤−0.3 | +408/116/1.29 | +1386 / 395 / 1.29 | -534 | 0.054 | 1.088 | +1085 / -160 / +246 / +215 | +301/276/1.09 |
| adx_signed_t00_c50 | signed ADX, core 1h≤−0.5 | +75/88/1.07 | – | | | | | |
| adx_signed_t15_c40 | signed ADX, thrust 1h≤−0.15 | +326/69/1.36 | – | | | | | |
| adx_signed_t15 | = same cell, train run | (+326/69) | +1212 / 232 / 1.45 | -298 | 0.028 | 1.134 | +919 / -33 / +70 / +256 | +293/163/1.13 |
| macd_p003 | (c) MACD pct 0.003 | +96/117/1.07 | – | | | | | |
| macd_p004 | MACD pct 0.004 | +136/99/1.10 | – | | | | | |
| macd_p005 | MACD pct 0.005 | -97/83/0.92 | – | | | | | |
| macd_p007 | MACD pct 0.007 | -34/71/0.97 | – | | | | | |
| vpin_lb40_f174 | VPIN lookback 40, floor 0.174 (pctile-matched) | +317/97/1.30 | +1197 / 294 / 1.36 | -434 | 0.040 | 1.079 | +995 / -83 / +29 / +256 | +202/219/1.08 |
| vpin_lb10_f285 | VPIN lookback 10, floor 0.285 | +148/83/1.14 | – | | | | | |
| warmup12 | warm-up 12 days | +59/97/1.05 | – | | | | | |
| adx_signed_vl40 | signed ADX + VPIN lb40/0.174 | (+563/95) | +1494 / 304 / 1.45 | -435 | 0.020 | 1.174 | +1048 / -113 / +146 / +414 | +447/224/1.17 |
| r21_v020_adx | r21_v020 + signed ADX | (+618/79) | +1562 / 282 / 1.51 | -414 | 0.011 | 1.31 | +847 / -27 / +392 / +351 | +715/191/1.31 |
| r21_v020_adx_t10 | … thrust 1h≤−0.10 | (+634/90) | +1276 / 329 / 1.34 | -681 | 0.047 | 1.226 | +658 / -62 / +435 / +244 | +617/225/1.23 |
| r21_v020_adx_t20 | … thrust 1h≤−0.20 | (+662/72) | +1256 / 245 / 1.46 | -430 | 0.031 | 1.301 | +665 / -100 / +329 / +364 | +592/161/1.30 |
| r21_v020_adx_p10 | … ADX period 10 | (+567/85) | +1231 / 310 / 1.34 | -697 | 0.054 | 1.202 | +698 / -117 / +413 / +236 | +532/211/1.20 |
| r21_v020_adx_p20 | … ADX period 20 | (+513/71) | +1729 / 243 / 1.72 | -186 | 0.004 | 1.431 | +924 / +215 / +334 / +255 | +805/166/1.43 |
| r21_v020_noadx | r21_v020, ADX off (control → nominee) | (+487/69) | +1579 / 236 / 1.67 | -227 | 0.005 | 1.431 | +810 / +190 / +343 / +236 | +769/158/1.43 |
| r21_v020_noadx_t10 | … thrust 1h≤−0.10 | (+704/81) | +1997 / 287 / 1.69 | -250 | 0.002 | 1.437 | +1056 / +78 / +592 / +271 | +941/193/1.44 |
| r21_v020_noadx_t20 | … thrust 1h≤−0.20 | (+662/56) | +1486 / 202 / 1.74 | -247 | 0.007 | 1.615 | +617 / +87 / +485 / +298 | +869/131/1.62 |
| r21_v0217_noadx | … VPIN 0.217 | (+445/64) | +1475 / 220 / 1.66 | -239 | 0.005 | 1.44 | +738 / +184 / +294 / +259 | +737/149/1.44 |
| r21_v020_noadx_nobbw | … and bollinger bandwidth off | (+201/105) | +524 / 376 / 1.12 | -901 | 0.248 | 0.929 | +763 / -340 / +68 / +34 | -238/271/0.93 |
| r21_v020_t10 | legacy twin of noadx_t10 | (+269/87) | +1519 / 307 / 1.46 | -426 | 0.014 | 1.197 | +1024 / +55 / +386 / +53 | +495/216/1.20 |
| v18_noadx | v18, ADX off | (+178/90) | +1282 / 308 / 1.35 | -328 | 0.038 | 1.128 | +927 / +227 / +166 / -37 | +356/219/1.13 |

Dead ends / bookkeeping: `dir1h_c60_vl40` was written but never run (dir1h_c60 died on train first). The first run of
`r21_v020_noadx` used `{"disable": ["adx_14_1hr"]}`, which is a no-op: the backtest's patch `disable` only applies to
**actions** (crates/backtest/src/main.rs:584). It reproduced meta/r21_v020 exactly, and that run is in the ledger. The fixed patch
uses `enabled:false` on the indicator. **Every desk that "disabled" an indicator through `disable` ran the unmodified config.**

## proposals that need engine changes (not implemented)

1. The patch `disable` key should also cover indicators, or reject unknown ids. Right now it silently no-ops.
2. `vpin.bucket_divisor` is dead code: delete it or implement real volume buckets.
3. A fair test of a directional 1h needs a thrust condition that lags the **1m only** (e.g. `timescale_lag` with `against: ["OneMinute"]`).
   With today's vocabulary, a directional 1h removes the thrust window by construction.
4. If ADX-off is promoted, the clean form is a config change only (`adx_14_1hr.enabled=false`, other 1h weights renormalise automatically). No engine change needed.

## nominations (best of 33 desk specs; the family itself, row 21, came out of the ~2,100-config history)

### 1. `research/lab/specs/indicators/r21_v020_noadx_t10.json` (primary)
Row 21 (sqrt band, am/pm windows) with VPIN 0.20, thrust 1h ≤ −0.10, ADX off.
Train +1997 / 287 / PF 1.69, dd −250, sign-flip p 0.002, ex-best-year PF 1.44. Years 22 +1056 / 23 +78 / 24 +592 / 25h1 +271.
Halves: 22h1 +707, 22h2 +349, 23h1 −72, 23h2 +150, 24h1 +159, 24h2 +433, 25h1 +271.
2023_25h1 +941 / 193 / PF 1.44. Paired: vs v18 train +818 (P 0.051), 2023_25h1 +851 (P 0.020); vs r21_v020 train +467 (P 0.13),
2023_25h1 +347 (P 0.11); vs its legacy twin r21_v020_t10 train +478 (P 0.11), 2023_25h1 +446 (P 0.034).
Tickers: NVDA +1435 / 137, MSFT +479, AMZN +298, AAPL −216 (AAPL negative in every row-21 cell).
Why this cell: the most trades on the plateau (≈ 6.8/month), and the paired comparison against its own legacy twin is the strongest on the desk.
**Holdout expectation (written before reveal):** PF 1.30 (80 % band 1.0–1.7), P&L +250 to +650 on ~95–115 trades. AAPL negative,
NVDA > 50 % of P&L. Beats r21_v020 on the holdout by about +100 (not significant). Target-window trade count is borderline for the
150 bar (≈ 6.5/month × 22 months ≈ 140–150).

### 2. `research/lab/specs/indicators/r21_v020_noadx.json` (one-knob twin of meta/r21_v020)
Exactly meta/r21_v020 plus `adx_14_1hr.enabled=false`. Train +1579 / 236 / PF 1.67, dd −227 (vs −451), p 0.005, ex-best-year PF 1.43.
Years 22 +810 / 23 +190 / 24 +343 / 25h1 +236. 2023_25h1 +769 / 158 / 1.43.
Paired vs r21_v020: train +49 (P 0.44), 2023_25h1 +175 (P 0.23), daily correlation 0.85.
Why: the cleanest test of the mechanism. If the coordinator reveals r21_v020 too, the pair tells whether the ADX-in-chop story holds out of sample.
**Holdout expectation:** about the same P&L as r21_v020 (±150), ~10 % fewer trades, smaller drawdown. PF 1.3 ± 0.3. Fails the 150-trade bar on the target window (≈ 5.6/month).

(No third nomination. Signed-ADX cells, best r21_v020_adx_p20 at +1729/243/1.72, sit on a monotone period gradient (1.34 / 1.51 / 1.72)
that ends at "weakest ADX ≈ no ADX". They are a noisier version of nomination 2, not a separate idea.)
