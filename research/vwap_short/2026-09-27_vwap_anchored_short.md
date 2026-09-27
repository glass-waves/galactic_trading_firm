# VWAP-anchored short — does a down-day VWAP retest turn a crash into capital?

*2026-09-27, follows `research/stress/2026-09-26_stress_mode.md` (its verdict proposed this entry). Research
only. Five-year IEX replay (2022-01-03..2026-09-10), honest costs (3 bps + $0.005 per leg), 36 % sizing,
`--cross-index SPY`, VWAP-fixed binary (session VWAP resets each eastern day, plan §16) rebuilt 13:44 with
the two code additions below. Baseline `iex_v18` = live book (row 12): +1,728 / 436 / PF 1.37; SPY < −1 %
days +1,022 on 38 trades; 25 worst days +198 on 6.*

## 1. Design

Hypothesis: on a genuine down day a name sits below its session VWAP and every rally back to VWAP is sold;
short the retest, stop just above VWAP.

**Code (two additions, nothing else in crates/ changed):**
- `crates/indicators/src/composable/vwap_distance.rs` — metadata only (score unchanged): `dist_pct`,
  `high_dist_pct`, `min_dist_pct_today` / `max_dist_pct_today` (deepest close vs the *running* typical-price
  VWAP today), `rebound_since_min_pct` (highest close-vs-VWAP after that low, current bar excluded) and
  `mins_since_min`. Stateless: scans today's bars each tick. The 1 m window holds 200 bars (to ~12:50 ET),
  so the part of the day before it is filled from today's completed 5 m bars.
- `crates/actions/src/exit/vwap_stop.rs` (registered as `vwap_stop`) — short exits when close >
  VWAP × (1 + `buffer_pct`/100), long mirrored; reported as `FilterAlignment`. **Scope:** exit actions act
  on every position, so `scope_min_max_hold_ms` limits it to positions whose entering window set a max
  hold ≥ that value (the engine exposes it in `position_context.max_hold_ms` and resets it on exit). The
  VWAP window sets 3 h; the morning windows run on the 90 min default and are never touched. 6 unit tests.

**Cell = v18 unchanged + one window** (`make_patches.py`): both promoted shorts disabled and re-added as the
stress study's `_am` copies (11:30 / 11:55 clocks); `vwap_1m` (vwap_distance, OneMinute, weight 0);
`vwap_stop` (priority 2, scoped at 2 h); window `vwap retest stress`, short, 10:00–15:15 ET:
1. day trigger `cross_1m.index_session_ret` ≤ −1.0 / −1.5 / −2.0 % (`s10/s15/s20`), none for the control `c`;
2. `min_dist_pct_today` ≤ −D (D = 0.5 / 0.8 %), i.e. the name was ≥ D below VWAP earlier;
3. `dist_pct` ∈ [−0.15, +0.10] — at VWAP now;
4. `rebound_since_min_pct` ≤ −0.16 — the first close back in that band since the day's low. This is the
   one-entry-per-name-per-day rule: the engine cooldown is global and would cut the morning book's
   re-entries, so the rule is structural instead; a second entry needs a new low vs VWAP (`_multi` drops it).
- `exit_overrides {force_exit_by 15:55, max_hold_ms 10,800,000, score_exit_threshold −10}`; hard stop 2.5 %,
  breakeven 0.5 % and ATR 7× stay as outer guards. Session opened by flags `--no-new-entries-after 15:30
  --force-exit-by 15:55`.
- **priority −5**: the window is evaluated *before* v18's two `1m noise` reject gates. A VWAP retest is by
  construction a 1 m counter-move; the long-side gate (1 m leads, 5 m ≤ 0.35) rejects it on most retest bars
  (seen in the 2025-04-08 tick dump). The window only fires when the morning windows cannot (SPY band vs
  SPY trigger) or on the control's overlap, and the pre-emption counts below show the ordinary book intact.

Scaffold `vw_am_only` (`_am` copies + `vwap_1m` + scoped `vwap_stop`, no window) reproduces `iex_v18`
**trade for trade: 436/436 entries, exits, sizes and P&L identical**, so the new metadata and the scoped
stop change nothing on their own.

Gate `stress-mode` (flags in order): PF ≥ 1.3 · ≥ 4/5 years positive · trades ≥ 0.9 × 436 · SPY < −1 % P&L
≥ 3 × 1,022 · ≥ +40/day on those 117 days · no day < −300 · ordinary-day (SPY ≥ −1 %) P&L within ±10 % of
+706. Tail sets as in the stress study (SPY open→close from `data/bars_iex/SPY.csv`; the 25 worst days are
exactly the 25 SPY < −2 % days). False alarm = VWAP-trade day on which SPY closed > −0.25 %.

## 2. Grid

Pass 1: control and s15 × D {0.5, 0.8} × buffer {0.3, 0.5}. Pass 2: the best D/buffer (0.8 / 0.5 — best
control cell by +2,800, s15 cells identical within noise) at s10 / s20, plus s00 / s05 to map the trigger.
Follow-ups: VWAP stop off (`_nostop`, buffer 5 %: only the 2.5 % hard stop, breakeven, 3 h, 15:55 remain),
re-entries allowed (`_multi`), and `_only` (window without the morning book) to count pre-emption.
`subset` = the VWAP window's own trades (P&L / n / PF, win %, mean hold).

| cell | 5y P&L / n / PF | +yrs | VWAP subset | win / hold | SPY<−1 % (117 d) | SPY<−2 % (25 d) | ordinary Δ | corr | worst day | false alarms | gate |
|---|---|---|---|---|---|---|---|---|---|---|---|
| iex_v18 | +1728 / 436 / 1.37 | 3 | — | — | +1022 | +198 | — | −0.20 | −123 | — | — |
| c_d05_b03 | −2964 / 2398 / 0.88 | 1 | −4732 / 1968 / 0.76 | 29 % / 85 m | +4704 (233 v) | +977 (36 v) | −1187 % | −0.39 | −141 | 585 d −9766 | xx....x |
| c_d05_b05 | −2300 / 2376 / 0.91 | 2 | −4077 / 1947 / 0.81 | 35 % / 107 m | +5524 (231 v) | +1012 (36 v) | −1209 % | −0.42 | −176 | 583 d −11172 | xx....x |
| c_d08_b03 | −1099 / 1517 / 0.93 | 2 | −2877 / 1085 / 0.74 | 28 % / 73 m | +3331 (162 v) | +565 (22 v) | −728 % | −0.32 | −134 | 369 d −5319 | xx..x.x |
| c_d08_b05 | −123 / 1508 / 0.99 | 2 | −1915 / 1077 / 0.84 | 34 % / 96 m | +3809 (162 v) | +586 (22 v) | −657 % | −0.34 | −137 | 369 d −5937 | xx..x.x |
| s00_d08_b05 | +734 / 1147 / 1.06 | 3 | −1002 / 712 / 0.87 | 36 % / 97 m | +3185 (152 v) | +583 (22 v) | −447 % | −0.28 | −137 | 182 d −3412 | xx..x.x |
| s05_d08_b05 | +1504 / 668 / 1.21 | 3 | −224 / 232 / 0.91 | 37 % / 91 m | +1999 (99 v) | +547 (18 v) | −170 % | −0.23 | −123 | 21 d −736 | xx.xx.x |
| s10_d08_b05 | +1588 / 492 / 1.30 | 3 | −140 / 56 / 0.78 | 30 % / 83 m | +1184 (40 v) | +195 (10 v) | −43 % | −0.21 | −123 | 5 d −145 | xx.xx.x |
| s15_d05_b03 | +1522 / 451 / 1.31 | 3 | −203 / 15 / 0.01 | 7 % / 37 m | +871 (12 v) | +157 (3 v) | −8 % | −0.19 | −123 | 0 | .x.xx.. |
| s15_d05_b05 | +1494 / 451 / 1.30 | 3 | −234 / 15 / 0.05 | 13 % / 52 m | +874 (12 v) | +159 (3 v) | −12 % | −0.19 | −123 | 0 | .x.xx.x |
| s15_d08_b03 | +1540 / 449 / 1.31 | 3 | −185 / 13 / 0.01 | 8 % / 40 m | +874 (11 v) | +160 (2 v) | −6 % | −0.19 | −123 | 0 | .x.xx.. |
| s15_d08_b05 | +1522 / 449 / 1.31 | 3 | −206 / 13 / 0.06 | 15 % / 49 m | +877 (11 v) | +162 (2 v) | −9 % | −0.19 | −123 | 0 | .x.xx.. |
| s20_d08_b05 | +1723 / 437 / 1.36 | 3 | −5 / 1 / 0.00 | 0 % / 35 m | +1017 (1 v) | +193 (1 v) | 0 % | −0.20 | −123 | 0 | .x.xx.. |
| s05_d08_nostop | +1999 / 666 / 1.28 | 4 | +272 / 230 / 1.11 | 45 % / 122 m | +2527 (98 v) | +654 (18 v) | −175 % | −0.26 | −177 | 21 d −1076 | x..xx.x |
| s10_d08_b05_multi | +1158 / 627 / 1.16 | 4 | −571 / 191 / 0.77 | 30 % / 68 m | +1710 (118 v) | +621 (28 v) | −178 % | −0.23 | −186 | 10 d −632 | x..xx.x |

Exit mix of the subset (share of trades): the VWAP stop is 33–61 % of exits in every cell and loses
everywhere (control d08_b05: `FilterAlignment` −10,039 on 400; `MaxHoldTimeout` +6,418 on 256, `SessionClose`
+2,565 on 216, breakeven −665 on 190). s15 cells: VWAP stop 53–61 %, breakeven 30–33 %, one max-hold winner.
Pre-emption: no baseline entry lost in any s05–s20 cell; control 5 (−79), s00 1 (−19). The other way,
`s10_d08_b05_only` has the same 56 VWAP trades as `s10_d08_b05` — 0 VWAP entries lost to a morning position.

What the grid says:
- **the day filter kills the setup rather than carrying it.** With SPY ≤ −1.5 % *at the retest bar* there are
  13–15 trades in five years (s20: one), and they lose (win 7–15 %). By the time SPY is down 1.5 % the names
  are far below VWAP and do not come back to it the same day; the retests that do happen are names showing
  relative strength, which then reclaim VWAP (VWAP stop) or stall (breakeven).
- **the control is where the crash-day capture is** (+3,331 to +5,524 on SPY < −1 % days, 3.3–5.4× the
  baseline; the two D 0.5 control cells are the only ones meeting both tail clauses), because the first VWAP
  retest usually comes early, while SPY is still near flat (median SPY at a first retest on days closing
  < −1.5 %: −0.43 %, D 0.5). But the same retest also fires on the ~87 % of traded days that do not close
  < −1 % (control d08_b05: 81 of 626), and 369–585 false-alarm days cost −5,300 to −11,200.
- **loosening the trigger trades capture for false alarms monotonically** (c → s00 → s05 → s10 → s15:
  SPY < −1 % P&L +3,809 → +3,185 → +1,999 → +1,184 → +877; false-alarm cost −5,937 → −3,412 → −736 → −145 → 0)
  and the subset stays negative at every point (PF 0.84 → 0.87 → 0.91 → 0.78 → 0.06).
- **the tight VWAP stop is the loss centre.** Turning it off (`s05_d08_nostop`) flips the subset to +272 / 230 /
  PF 1.11 (win 45 %, hold 122 m), combined +1,999 (best in the study, +271 over baseline) and SPY < −1 % days
  +2,527 (2.5×) — but 2024 goes −465 on 26 trades (7 hard stops −656 across the five years, trailing −623)
  and ordinary days go from +706 to −528. In the control, buffer 0.5 beats 0.3 (+660 / +960) and D 0.8 beats 0.5; at s15 the pairs differ by ≤ 30.
- **re-entries do not help** (`_multi` at s10): +135 trades, subset −571, worst day −186.

## 3. Tail days: baseline vs the literal hypothesis (s15_d08_b05) vs the best cell (s05_d08_nostop)

| day set | iex_v18 P&L / per day / trades / worst | s15_d08_b05 | s05_d08_nostop |
|---|---|---|---|
| SPY < −1 % (117 d) | +1022 / +8.7 / 38 / −71 | +877 / +7.5 / 49 (11 v) / −81 | +2527 / +21.6 / 136 (98 v) / −85 |
| SPY < −2 % = 25 worst (25 d) | +198 / +7.9 / 6 / −23 | +162 / +6.5 / 8 (2 v) / −23 | +654 / +26.2 / 24 (18 v) / −23 |
| ordinary, SPY ≥ −1 % (1058 d) | +706 / +0.7 / 398 / −123 | +645 / +0.6 / 400 (2 v) / −123 | −528 / −0.5 / 530 (132 v) / −177 |

Correlation of daily P&L with SPY: −0.20 / −0.19 / −0.26. Per year (s05_d08_nostop, subset in brackets):
2022 +1,595 (+603) / 2023 +3 (+8) / 2024 −483 (−465) / 2025 +404 (+75) / 2026 +480 (+51). False alarms: 21
days −1,076; SPY closed ≤ −1 %: 50 days +1,505 (PF 4.5); in between: 57 days −157.

Gate: `s15_d08_b05` fails years (3/5) and both tail clauses; `s05_d08_nostop` fails PF (1.28), both tail
clauses (+2,527 < 3,066 and +21.6 < +40/day) and ordinary ±10 % (−175 %). **No cell passes; none is a clean
additive improvement either** (every cell with more than one VWAP trade has a negative year or moves the
ordinary days by > 10 %).

## 4. What a −5 % day looks like (the honest reading)

Engine, best cell `s05_d08_nostop`, the 25 worst days: VWAP trades on 9 of them (18 trades, +654 vs the
baseline's +198). Nothing at all on 2025-04-08 (−4.9 %), 2022-08-26 (−3.4 %), 2025-11-20 (−3.1 %),
2025-10-10 (−2.8 %), 2022-04-22, 04-21, 06-28, 2024-04-15, 2023-03-09, 2022-09-02, 12-13, 2024-04-04.
The rest, trade by trade (ET, V = VWAP window, M = morning book):
2025-04-04 (−3.4) M NVDA 09:33→11:04 max-hold +101; V AMZN 10:54→11:13 **hard stop −98** (AMZN closed +2.7 %
on a −3.4 % tape); V AAPL 11:05→11:12 breakeven −1: day +2 vs baseline +101. 2022-10-14 V NVDA 11:04→14:05 +52.
2022-03-07 V AAPL +38, MSFT −4, NVDA +72. 2022-04-29 M AMZN −7, V AMZN −6. 2022-02-23 V AAPL +62, NVDA +153,
AMZN +66 (+281, the best day). 2022-05-05 M AMZN +97, V NVDA −5. 2022-04-26 V NVDA −11, MSFT −3, AMZN +10,
AAPL +47. 2022-09-21 V NVDA −5. 2022-09-13 M +39 / −9, V AMZN +38, MSFT +52. 2024-12-18 M MSFT −23.
Every V winner exits on the 3 h max-hold, not at the close; the 2025 crash days contribute −99.

Raw bars (`bar_check.py days`, no stops or costs, first retest after ≥ 0.8 % below VWAP, short to 15:55):
- **2025-04-08 (−4.9 %)**: SPY opened up, first ≤ −1 % at 11:12, ≤ −2 % at 12:36. Names spent 86–91 % of the
  session below VWAP and ended 3.6–7.0 % under it. AMZN retested at 10:01 and NVDA at 10:31 — both with SPY
  still *green* (+0.1 / +0.2 %) — and would have made +7.1 / +9.1 % to the close; AAPL and MSFT never came
  back. The engine took neither: AMZN's first touch was 09:57, before the 10:00 window (the first-retest rule
  then blocks), and in every triggered cell SPY was nowhere near the trigger.
- **2025-04-04 (−3.4 %)**: retests at 10:00–11:07 with SPY −0.2 to −1.7 %; three would have made +2.4–3.9 %,
  AMZN (the one the engine took) +0.5 % with a > 2.5 % adverse excursion on the way.
- **2022-08-26 (−3.4 %)**: 97–98 % of bars below VWAP; only NVDA retested (10:00, SPY −0.3 %, +8.4 % to the
  close). No name came back to VWAP after SPY reached −1 % (10:27).
- **2024-08-05**: SPY −3 % vs the prior close but +1.2 % open→close; no retest, not a tail day by this metric.

`bar_check.py frontier` — every first retest in five years, by SPY at entry (no stop, no costs):
| SPY at entry | name-days | short→15:55 mean | win | on days SPY then closed < −1 % |
|---|---|---|---|---|
| any | 1450 | −0.01 % | 51 % | 205: +1.53 % |
| ≤ −0.5 % | 443 | +0.04 % | 58 % | 147: +0.97 % |
| ≤ −1.0 % | 131 | −0.11 % | 53 % | 73: +0.66 % |
| ≤ −1.5 % | 27 | −0.07 % | 48 % | 22: +0.31 % |

This is the whole story. A VWAP-retest short on the days that *end* red is worth +1–1.5 % a trade, but with
only what is known at the retest it is a coin flip before costs (≈ 0.12 % a round trip). The later the
trigger, the fewer the retests and the less is left to capture.

## 5. Verdict

**The hypothesis fails as specified; no `vwap_short_v1.patch.json` is written.** On a genuine down day the
names do sit below VWAP (85–98 % of bars on the crash days above), but they rarely return to it once the
market is visibly down: at SPY ≤ −1.5 % there are 13–15 retests in five years and they lose, and the tight
stop above VWAP is the single largest loss line in every cell. The retests that do pay come early, while SPY
is still flat, and an intraday rule cannot tell them from the ~86 % of first retests (1,245 of 1,450) on days that do not close < −1 %.
The best variant (`s05_d08_nostop`: SPY ≤ −0.5 %, no VWAP stop) is +271 over the baseline with 2.5× the
SPY < −1 % capture, but loses −465 in 2024 and costs −1,234 on ordinary days — a different, noisier book, not
an add-on. Compared with the stress study's `s15_core` (+274 on 23 trades, PF 3.3, ordinary +0.7 %) nothing
here is better. If crash-day capture stays a goal, the next thing to test is holding what the morning book
already has on red days (e.g. a max-hold / 11:55 extension when SPY ≤ −1 %), not a new afternoon entry.

The two code additions are harmless (scaffold reproduces 436/436) and can stay; `vwap_stop` with
`scope_min_max_hold_ms` is a general window-scoped exit.

## 6. Exact commands

```
cargo test -p actions -p indicators && cargo clippy -p actions -p indicators -- -D warnings
flock logs/.research.lock cargo build --release -p backtest          # once, 13:44, before any sweep
python3 research/vwap_short/make_patches.py        # writes every cell's patch (regenerable, pruned after)
research/vwap_short/run_queue.sh am_only c_d05_b03 s15_d05_b03 c_d05_b05 s15_d05_b05 c_d08_b03 s15_d08_b03 c_d08_b05 s15_d08_b05
research/vwap_short/run_queue.sh s10_d08_b05 s20_d08_b05 s05_d08_b05 s00_d08_b05
research/vwap_short/run_queue.sh s05_d08_nostop s10_d08_b05_multi s10_d08_b05_only
#   each = BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh vw_<cell> --sizing-fraction 0.36 --max-position-pct 0.36 \
#          --cross-index SPY --no-new-entries-after 15:30 --force-exit-by 15:55 --patch-json research/vwap_short/<cell>.json
#          under logs/.research.lock, then research/stress/fixup_skipped.sh (0 days re-run in this study)
python3 research/vwap_short/analyze.py --grid vw_c_d05_b03 ... vw_s10_d08_b05_multi      # §2
python3 research/vwap_short/analyze.py --only vw_s10_d08_b05_only vw_s10_d08_b05         # pre-emption
python3 research/vwap_short/analyze.py vw_s05_d08_nostop ; python3 research/vwap_short/analyze.py --walk vw_s05_d08_nostop   # §3, §4
python3 research/vwap_short/bar_check.py frontier ; python3 research/vwap_short/bar_check.py days 2025-04-08,2025-04-04,2022-08-26,2024-08-05
```
