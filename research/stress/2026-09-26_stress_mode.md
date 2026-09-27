# Stress mode study — capturing the SPY-down days without touching the ordinary book

*workstream C, 2026-09-26/27. research only; five-year IEX replay (2022-01-03..2026-09-10), honest costs
(3 bps + $0.005 per leg), 36 % sizing, `--cross-index SPY`. baseline `iex_v18` = the live book (row 12)
on the parity-fixed replay (session VWAP resets each eastern day, binary of 2026-09-27 00:53): +1,728 /
436 trades / PF 1.37, years +991 / −3 / −20 / +332 / +429. a first pass on the pre-fix binary reached the
same conclusions with slightly different numbers; its outputs are parked in `data/st_prevwap/`.*

## 1. Question and design

v18's short windows require SPY inside ±0.2 % of its open (`cross_1m` in [−0.4, 0.4]) and stop entering at
11:30 ET / flatten by 11:55. On the 25 days since 2022 with SPY's session return < −2 % it made $198 on
6 trades; on the 117 days with SPY < −1 % it made $1,022 on 38 trades (+8.7 per day) — 59 % of its
five-year P&L, because a short taken while SPY is still flat pays off precisely on the days that then go on
to close down. Daily P&L correlates −0.20 with SPY; ordinary days (SPY ≥ −1 %) give +706. The question is whether a *separate* stress rule set
can enter once the market is already down hard, all day, without diluting the ordinary book.

**Cell = v18 unchanged + one pair of stress windows.** Every patch (`research/stress/*.json`, emitted by
`make_patches.py`):
- `disable` both promoted short windows and re-add them as `_am` copies that carry their own clocks
  (`entry_before 11:30`, `exit_overrides.force_exit_by 11:55`); conditions verified equal to row 12's
  (script check in `make_patches.py` / `am_only` cell). The session is opened by flags
  `--no-new-entries-after 15:30 --force-exit-by 15:55` (B5: the session gate runs before any window).
- stress windows = the same two windows (`5m thrust stress`, `strong core stress`, priorities 30/40)
  with the SPY-flat band removed and a trigger added; `entry_after 09:45`, `entry_before 15:30`,
  `exit_overrides {force_exit_by 15:55, max_hold_ms 7,200,000 (h3: 10,800,000), score_exit_threshold −0.6
  (se10: −10 = off)}`. B4: shorts score-exit at `composite >= −threshold`, so −0.6 exits only when the
  composite has flipped to +0.6.
- triggers read metadata already produced by v18's instances: `cross_1m.index_session_ret` (SPY open→now,
  percent) and `pdl_5m.dist_close_pct` (name vs prior close, percent; `prior_day_levels` appended at
  weight 0). Cells: SPY ≤ −0.5 / −1.0 / −1.5 % (`s05/s10/s15`), name ≤ −1.5 / −2.5 % (`n15/n25`), combined
  SPY ≤ −1 % & name ≤ −1.5 % (`c10n15`), plus variants on the best trigger: `_h3`, `_se10`, `_novpin`,
  sized-up (`indicator_tiered` on the trigger value, ×1.5 below it, `--max-position-pct 0.54`), and
  `_only` (stress windows without the morning windows, to count pre-emption).
- the morning and stress windows are mutually exclusive at any one bar (SPY flat vs SPY down), so
  pre-emption can only happen through an open position or the 30 s cooldown; §5 counts both directions.

Tail sets (from `data/bars_iex/SPY.csv`, open of the first RTH bar → close of the last): SPY < −1 %:
117 days (2022: 54, 2023: 19, 2024: 15, 2025: 18, 2026: 11); SPY < −2 %: 25 days = the 25 worst.
"Ordinary days" = SPY ≥ −1 % (1,058 days). False alarm = a day with a stress trade on which SPY
closed > −0.25 %.

Accept-if (gate `stress-mode`): combined PF ≥ 1.3, ≥ 4 of 5 years positive, trades ≥ 0.9 × 436, on
SPY < −1 % days P&L ≥ 3 × $1,022 and ≥ +40 per day (i.e. ≥ $4,680), no single day < −300, ordinary-day
P&L within ±10 % of $706.


## 2. Grid (five-year IEX replay, all cells vs the fixed `iex_v18`)

Scaffold check `st_am_only` (v18 with the `_am` copies, `pdl_5m` at weight 0 and the opened session, no
stress windows) reproduces `iex_v18` trade for trade: 436/436 entries, exits, sizes and P&L identical.
In every cell below the morning book keeps the same entries and exits (except where "lost" says
otherwise); its P&L moves by a few dollars only because sizes are whole shares of a capital that carries
the lookback days' stress P&L. Columns: `stress subset` = the stress windows' own trades; `SPY<-1 %` /
`SPY<-2 %` = the cell's whole-book P&L on those days (stress trades in brackets), baseline +1,022 / +198;
`ordinary Δ` = P&L on SPY ≥ −1 % days vs the baseline's $706; `corr` = daily P&L vs SPY session return
(baseline −0.20); `false alarms` = stress-trade days on which SPY closed > −0.25 %; gate flags in order
PF · +yrs · trades · tail 3× · tail +40/d · worst day · ordinary ±10 %. Phase-1 cells: VPIN kept, hold 2 h,
score-exit −0.6, both windows. Variants: `_core` strong-core window only, `_novpin` VPIN dropped, `_late`
entry_after 10:30, `_fall` + SPY 15-min return ≤ −0.1 %, `_h3` hold 3 h, `_se10` score-exit off, `_big`
`indicator_tiered` ×1.5 below the trigger with `--max-position-pct 0.54`, `_only` stress windows alone.

| cell | 5y P&L / n / PF | +yrs | stress subset | SPY<-1 % (117 d) | SPY<-2 % (25 d) | ordinary Δ | corr | worst day | false alarms | gate |
|---|---|---|---|---|---|---|---|---|---|---|
| iex_v18 (baseline) | +1728 / 436 / 1.37 | 3 | — | +1022 | +198 (6 trades) | — | −0.20 | −123 | — | — |
| st_s05 | +146 / 756 / 1.01 | 2 | -1592 / 320 / 0.70 | +2562 (157 st) | +1678 (58 st) | -442 % | -0.28 | -208 | 41 d -1833 | fail xx.xx.x |
| st_s10 | +1152 / 592 / 1.16 | 4 | -582 / 156 / 0.78 | +1704 (106 st) | +1262 (52 st) | -178 % | -0.26 | -208 | 8 d -784 | fail x..xx.x |
| st_s15 | +1794 / 512 / 1.32 | 5 | +60 / 76 / 1.07 | +1186 (66 st) | +773 (38 st) | -14 % | -0.24 | -149 | 2 d -174 | fail ...xx.x |
| st_s20 | +1382 / 465 / 1.26 | 3 | -347 / 29 / 0.42 | +867 (25 st) | +313 (20 st) | -27 % | -0.21 | -124 | 1 d -183 | fail xx.xx.x |
| st_n15 | -169 / 817 / 0.98 | 2 | -1828 / 397 / 0.71 | +2729 (134 st) | +1568 (50 st) | -511 % | -0.28 | -269 | 102 d -2826 | fail xx.xx.x |
| st_n25 | +40 / 671 / 1.00 | 2 | -1715 / 237 / 0.60 | +1784 (93 st) | +1001 (35 st) | -347 % | -0.23 | -138 | 55 d -1679 | fail xx.xx.x |
| st_c10n15 | +1385 / 551 / 1.21 | 4 | -348 / 115 / 0.81 | +1522 (80 st) | +979 (42 st) | -119 % | -0.25 | -123 | 7 d -435 | fail x..xx.x |
| st_s15_core | +2003 / 459 / 1.41 | 3 | +274 / 23 / 3.32 | +1292 (20 st) | +496 (13 st) | +1 % | -0.23 | -123 | 1 d +8 | fail .x.xx.. |
| st_s15_core_novpin | +1955 / 472 / 1.38 | 4 | +224 / 36 / 1.56 | +1298 (31 st) | +504 (20 st) | -7 % | -0.23 | -123 | 1 d -18 | fail ...xx.. |
| st_s15_novpin | +2001 / 575 / 1.32 | 5 | +266 / 139 / 1.17 | +1422 (120 st) | +1076 (73 st) | -18 % | -0.25 | -220 | 2 d -296 | fail ...xx.x |
| st_s15_late | +1826 / 508 / 1.33 | 5 | +92 / 72 / 1.11 | +1218 (62 st) | +758 (36 st) | -14 % | -0.25 | -149 | 2 d -174 | fail ...xx.x |
| st_s15_fall | +1757 / 510 / 1.31 | 5 | +23 / 74 / 1.03 | +1147 (65 st) | +733 (37 st) | -14 % | -0.24 | -149 | 2 d -174 | fail ...xx.x |
| st_s15_h3 | +1700 / 511 / 1.29 | 5 | -34 / 75 / 0.97 | +1154 (65 st) | +793 (37 st) | -23 % | -0.24 | -150 | 2 d -227 | fail x..xx.x |
| st_s15_se10 | +1794 / 512 / 1.32 | 5 | +60 / 76 / 1.07 | +1186 (66 st) | +773 (38 st) | -14 % | -0.24 | -149 | 2 d -174 | fail ...xx.x |
| st_s15_core_big | +2144 / 459 / 1.44 | 3 | +412 / 23 / 3.30 | +1428 (20 st) | +648 (13 st) | +2 % | -0.24 | -123 | 1 d +13 | fail .x.xx.. |
| st_s15_big | +1816 / 512 / 1.30 | 5 | +84 / 76 / 1.06 | +1251 (66 st) | +1049 (38 st) | -20 % | -0.25 | -222 | 2 d -256 | fail x..xx.x |
| st_s15_core_only | +385 / 24 / 4.26 | 2 | +385 / 24 / 4.26 | +271 (20 st) | +299 (13 st) | -84 % | -0.17 | -44 | 1 d +8 | fail .xxxx.x |

What the grid says:
- **the stress windows do capture the crash days.** on the 25 SPY < −2 % days every SPY-triggered cell
  makes 2.5–8.5× the baseline (+496 to +1,678 vs +198), the worst such day never falls below −44, and the
  daily-P&L correlation with SPY moves from −0.20 to −0.23..−0.28. the subset on days that then *close*
  ≤ −1 % is profitable in every cell but `s20` (PF 1.2–2.7; `s20` 0.6 on 25 trades).
- **but the trigger is an intraday touch and half the touches reverse.** 221 days touch −1 % and only 117
  close there; 506 touch −0.5 % and 256 close there. what the windows earn on the days that hold they give
  back on the days that bounce: `s10` +684 on 58 holding days vs −1,265 on 32 reversing/in-between days;
  `s05` +1,538 vs −3,130. the bleed sits in 2.5 % hard stops (`s10`: −938 on 10 trades, e.g. the
  2022-01-24 / 01-28 reversals, four of the ten) and breakeven knock-outs (48 trades at ≈ −4 each: a 2 h short in a
  ±0.5 %-per-hour tape). loosening the trigger only buys more of both.
- **the name's own move is not a stress detector.** `n15` puts on stress trades on 264 of the 1,224 replay
  days, 102 of them false alarms (−2,826), and pre-empts 16 baseline entries because a name down 1.5 %
  while SPY is flat is exactly a morning-window day. `n25` is the same picture; the combined `c10n15` is
  `s10` with fewer trades.
- **tightening SPY improves the subset monotonically** (−1,592 → −582 → +60 for −0.5/−1.0/−1.5 %) by
  removing false alarms (41 → 8 → 2 days) at the price of trades (320 → 76) and capture (+1,678 → +773);
  at −2 % (`s20`) the sample is 29 trades and the thrust window alone loses −444 (4 hard stops).
- **inside every cell `strong core stress` (1 h ≤ −0.4) is the good half and `5m thrust stress` the bad
  half**: `s15` +216 / 20 / PF 3.9 vs −156 / 56 / PF 0.8; `s20` +97 vs −444; `c10n15` +2 vs −350. the thrust
  window's 5 m lag/thrust logic finds the bounce, not the continuation.
- exits and clocks do not move the needle on `s15`: score-exit off (`_se10`) is identical (a stress short
  never sees composite +0.6), a 3 h hold (`_h3`) costs −94, entry after 10:30 (`_late`) +32, the
  "SPY still falling" filter (`_fall`) −37. dropping VPIN (`_novpin`) nearly doubles the trades (76 → 139)
  and adds +206 with 5/5 positive years, but −18 % on ordinary days and a −220 day.
- **sizing up does what it says and no more**: `s15_core_big` (×1.5 below −1.5 %, cap 0.54) turns +274 into
  +412 on the same 23 trades, worst day unchanged; `s15_big` adds +24 and a −222 day.
- **the tail gate is out of reach by a factor of ~4.** it asks for ≥ 3 × $1,022 = $3,066 and ≥ +40/day
  = $4,680 on the 117 SPY < −1 % days; the best clean cell adds +270 there (`s15_core`, +406 sized up),
  the loosest +1,540 (`s05`) while losing twice that elsewhere. at 36 % of $10k a stress short is
  ≈ $3,600; +$3,700 of new tail P&L would need ~100 extra trades averaging ~100 bps, on windows whose
  live siblings average ≈ 15 bps.

## 3. The one clean cell: `s15_core` (SPY ≤ −1.5 %, strong-core window only)

Dropping the `5m thrust stress` window leaves a stress rule that is purely *additive*: 23 trades in five
years, +274, PF 3.3, win 52 %, median hold 78 min; no hard stop and no trailing stop ever hit; 18 of the 23
trades in 2022, 2 in 2024, 3 in 2025, none in 2023/2026. Morning book: 436/436 entries and exits unchanged
(+1,729 vs +1,728), ordinary-day P&L +711 vs +706 (+0.7 %). Pre-emption: no baseline entry lost; one
stress entry lost to an open morning position (the stress-only cell `s15_core_only` has 24 trades / +385;
the missing one is NVDA 2022-05-05, +110).

| day set | s15_core P&L | per day | trades (stress) | worst day | iex_v18 P&L | per day | trades | worst day |
|---|---|---|---|---|---|---|---|---|
| SPY < −1 % (117 d) | +1292 | +11.0 | 58 (20) | −71 | +1022 | +8.7 | 38 | −71 |
| SPY < −2 % = 25 worst (25 d) | +496 | +19.9 | 19 (13) | −23 | +198 | +7.9 | 6 | −23 |
| ordinary, SPY ≥ −1 % (1058 d) | +711 | +0.7 | 401 (3) | −123 | +706 | +0.7 | 398 | −123 |

Correlation of daily P&L with SPY −0.23 (baseline −0.20). False alarms: one day (2022-11-04, SPY −0.2 %
at the close after touching −1.5 %: +8 on 2 trades). Worst single day unchanged (−123, 2025-02-28, a
morning-book day). Per year: 2022 +1,226 (PF 2.21) / 2023 −3 / 2024 −24 / 2025 +376 (1.46) / 2026 +429;
the subset itself +235 / 0 / −4 / +43 / 0.

The 23 trades (ET entry, exit, P&L, SPY close): 2022-03-07 AAPL 15:18 close +5 (−2.8); 04-21 NVDA 14:07
breakeven −1 (−2.4); 04-22 NVDA 12:09 max-hold +44 (−2.5); 04-29 NVDA 14:39 close +54 (−2.8); 05-05 MSFT
10:40 +5 and NVDA 13:57 −2 (−2.4); 05-10 MSFT 11:15 max-hold −44 (−1.4); 05-18 NVDA +2 (−2.9); 05-20 MSFT
−4 (−0.9); 08-26 NVDA 10:39 max-hold +97 (−3.4); 09-27 MSFT −5, AMZN −22 (−1.3); 10-14 AMZN 14:18 close
+32 (−3.0); 11-01 AMZN 10:53 +70 (−1.4); 11-02 AMZN 15:26 +19 (−2.3); 11-04 AMZN +15, AAPL −7 (−0.2);
12-06 AAPL 14:38 −23 (−1.4); 2024-08-07 NVDA −2 (−1.9); 09-06 MSFT −2 (−1.7); 2025-03-03 AMZN 15:02 +14
(−2.1); 04-08 AAPL 15:06 +33 and AMZN 15:08 −5 (−4.9; both entered in the last half hour the 15:30 clock
allows). 13 of the 23 fall on the 25 worst days (+298); half the crash days produce no entry at all — SPY
≤ −1.5 % together with composite ≤ −0.35, 5 m ≤ −0.4, 1 h ≤ −0.4 and VPIN ≥ 0.217 on the same bar is rare
(nothing on 2025-04-04 −3.4 % or 2025-11-20 −3.1 %). `s15_core_novpin` (VPIN dropped) adds 13 trades for
−50: +224 / 36 / PF 1.56, one hard stop (−203 on 2 trades), 4/5 years positive, ordinary −7 %.

Gate for `s15_core`: PF 1.41 ok, trades 459 ok, worst day ok, ordinary ±10 % ok (+0.7 %); **fails**
positive years (3/5 — the baseline's 2023 −3 and 2024 −20 are untouched by a subset with two trades in
those years) and both tail clauses (+1,292 vs the required ≥ 3,066 / ≥ 4,680 on SPY < −1 % days; +11.0
vs +40 per day).

## 4. Verdict

**No cell passes `stress-mode`; no `stress_mode_v1.patch.json` is written.** The gate's tail clause
(≥ $4,680 on SPY < −1 % days, 4.6× what the live book makes there) is not reachable by re-using v18's two
windows without their SPY band: those windows are continuation entries, and on an intraday −1 % touch the
continuation happens on roughly half the days, which the false alarms eat. The owner's stated aim —
capture the violent days without touching the ordinary book — is met literally by one cell, `s15_core`
(SPY ≤ −1.5 %, strong-core window only, 09:45–15:30, flat by 15:55, hold 2 h, score-exit −0.6, VPIN kept):
+274 on 23 trades / PF 3.3 over five years, 2.5× the baseline on the 25 worst days, morning book and
ordinary days unchanged, worst day unchanged, one false alarm (+8), zero stops hit — but it is 4–5 trades a
year, 18 of 23 in 2022, and it does nothing for 2023/2024 or for the days it was built for when the 1 h
score is not yet ≤ −0.4 (no entry on 2025-04-04 or 2025-11-20). It is a small, safe, additive rule, not a
stress mode.

If the owner wants it anyway, `research/stress/s15_core.pipeline.json` is that cell in pipeline format
(`session {no_new_entries_after 15:30, force_exit_by 15:55}`, both promoted shorts disabled and re-added as
the `_am` copies, `pdl_5m` appended, one stress window); a one-day `--patch-json`-only run equals the
flag-based run on 2022-08-26, 2025-04-08 and 2022-11-04 (5/5 trades identical, rebuilt binary). It would need
the gate relaxed to "tail P&L ≥ 2 × baseline, ordinary within ±10 %, no stop-outs" to pass; with `×1.5`
sizing (`s15_core_big`, +412) the tail ratio is 1.4× on SPY < −1 % days and 3.3× on SPY < −2 % days.
Recommended next step if the crash-day capture matters: a different entry, not a different trigger — the
`strong core` shape at 09:45+ on SPY ≤ −1.5 % is the only thing here with an edge, and it is throttled by
the 1 h ≤ −0.4 requirement arriving late; a session-VWAP-anchored short on the name below its VWAP with
SPY ≤ −1.5 % is the obvious candidate, and it needs the rule set from `s15_core` as its comparison.

## 5. Exact commands

```
python3 research/stress/make_patches.py                     # emits research/stress/*.json (all cells)
research/stress/run_cell.sh <cell> [extra args]             # tag st_<cell>; takes logs/.research.lock; =
#   BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh st_<cell> --sizing-fraction 0.36 \
#     --max-position-pct 0.36 --cross-index SPY --no-new-entries-after 15:30 --force-exit-by 15:55 \
#     --patch-json research/stress/<cell>.json ; then fixup_skipped.sh (re-runs days lost to a cache rewrite)
MAX_POS=0.54 TAG_SUFFIX=_big EXTRA="--patch-json research/stress/size_s15_x1.5.json" research/stress/run_cell.sh s15_core
research/stress/queue_pass2.sh                              # the order actually run on 2026-09-27
python3 research/stress/analyze.py --grid st_s05 st_s10 ... # §2 table
python3 research/stress/analyze.py --only st_s15_core_only st_s15_core   # §3 (tail tables, pre-emption, gate)
python3 research/stress/make_patches.py --pipeline research/stress/s15_core.pipeline.json research/stress/s15_core.json
research/stress/verify_pipeline_patch.sh s15_core research/stress/s15_core.pipeline.json 2022-08-26 2025-04-08 2022-11-04
```
Baseline SPY tail sets come from `data/bars_iex/SPY.csv` inside `analyze.py` (open of the first RTH bar →
close of the last). First-pass (pre-VWAP-fix) outputs are parked in `data/st_prevwap/` and
`logs/sweeps/st_prevwap/`; they are not cited above.
