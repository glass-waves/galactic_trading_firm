# missed short setups — what the v15 engine did not take, and why (2022–2026)

question: which highly profitable short setups did the engine NOT take, what is the single biggest
reason they were filtered out, and would admitting them (loosening one condition) be net positive
across years?

short answer: **no.** the great shorts the engine misses are not near-misses of any window condition —
86–90 % of them need two or more conditions changed and their 5-minute score is *bullish* (median s5m
+0.07 to +0.22). the one condition that, on its own, keeps out the most "very successful" bars is
`strong core short: s1h <= -0.30`, but the bars it excludes have the same expectancy as a random
09:30–11:29 bar (mean opp_pnl -0.066 %, hit-rate 39 %, pooled 12,279 bars / 3,369 episodes). every
loosening simulated adds volume, not edge: the best (+$470–490 over five years at the sweep's cost
model) is positive in only 2–3 of 5 years, adds ~$3 per added trade, and turns negative in 4–5 of 5
years once slippage is charged against the short. no tightening is robust either.

a material side-finding: the backtest cost model is direction-blind and therefore pays the SHORT
book ~$4.5/trade of favourable slippage (entry fill = open + 3 bps + $0.005, exit fill = open - 3 bps
- $0.005). re-running the same trades with the cost charged against the short gives
**2022 +1,860 / 2023 -442 / 2024 -474 / 2025 -478 / 2026 -291 = +175 over five years**. every sweep
number the v15 promotion rests on carries this tailwind.

---

## headline findings (each with the counter-evidence checked)

### 1. there is no pool of missed edge in the window conditions — the missed winners are not near-misses

very successful = top 5 % of `opp_pnl_pct` among all 09:30–11:29 bars in that year (threshold
+1.69 % in 2022, +1.11–1.16 % in 2023–2026), and separately >= +1.0 %.

| year | top5 bars | TAKEN | IN_POSITION (already short) | GATED (reject noise / cooldown) | FREE-not-taken | of FREE: need >= 2 condition changes |
|---|---|---|---|---|---|---|
| 2022 | 6,015 | 50 (0.8 %) | 1,732 (28.8 %) | 1,734 (28.8 %) | 2,499 (41.5 %) | 2,157 (86 %) |
| 2023 | 6,010 | 24 (0.4 %) | 1,131 (18.8 %) | 2,064 (34.3 %) | 2,791 (46.4 %) | 2,611 (94 %) |
| 2024 | 6,053 | 32 (0.5 %) | 1,054 (17.4 %) | 2,051 (33.9 %) | 2,916 (48.2 %) | 2,693 (92 %) |
| 2025 | 6,001 | 31 (0.5 %) | 1,455 (24.2 %) | 1,838 (30.6 %) | 2,677 (44.6 %) | 2,518 (94 %) |
| 2026 | 4,161 | 30 (0.7 %) | 1,003 (24.1 %) | 1,338 (32.2 %) | 1,790 (43.0 %) | 1,610 (90 %) |

(>= +1.0 % definition gives the same shares within 2 points; see analysis.md section C.)

what the FREE missed winners look like (median scores; TAKEN and ALL FREE for reference):

| year | set | n | median comp | median s5m | median s1h | s5m <= -0.40 | s5m > 0 |
|---|---|---|---|---|---|---|---|
| 2022 | FREE top5 winners | 2,499 | +0.20 | +0.22 | +0.13 | 21 % | 62 % |
| 2022 | ALL FREE | 51,518 | +0.15 | +0.08 | +0.26 | 16 % | 56 % |
| 2023 | FREE top5 winners | 2,791 | +0.18 | +0.13 | +0.32 | 16 % | 57 % |
| 2023 | ALL FREE | 57,599 | +0.14 | +0.06 | +0.31 | 11 % | 55 % |
| 2024 | FREE top5 winners | 2,916 | +0.14 | +0.07 | +0.26 | 18 % | 54 % |
| 2024 | ALL FREE | 57,842 | +0.15 | +0.07 | +0.30 | 10 % | 56 % |
| 2025 | FREE top5 winners | 2,677 | +0.17 | +0.08 | +0.34 | 17 % | 55 % |
| 2025 | ALL FREE | 55,290 | +0.14 | +0.07 | +0.28 | 9 % | 57 % |
| 2026 | FREE top5 winners | 1,790 | +0.14 | +0.12 | +0.24 | 16 % | 56 % |
| 2026 | ALL FREE | 37,424 | +0.15 | +0.09 | +0.30 | 12 % | 58 % |
| all | TAKEN | 1,286 | -0.45..-0.47 | -0.54..-0.58 | -0.25..-0.30 | 100 % | 0 % |

the missed winners are a random sample of the morning: more than half have a bullish 5-minute score
at the bar before the drop. the timescale scores do not see these moves; no threshold on them can.

counter-evidence checked: the windows do enrich — TAKEN bars are top-5 % 12–15 % of the time vs the
5 % base rate, and ">= +1 %" 14–29 % vs 6–12 %. the enrichment is real but it is a ~2.5x lift on a
5 % base rate, and (finding 4) it is worth less than the round-trip cost in four of five years.

### 2. the single biggest single-condition filter is `core s1h <= -0.30`, and it filters noise, not winners

FREE missed winners that exactly ONE condition (on either window) keeps out — bars (episodes = runs of
consecutive bars on one ticker-day):

| year | def | FREE winners | composite -0.35 | thrust s5m -0.50 | thrust lag 0.10 | thrust s1h 0.0 | core s5m -0.40 | core s1h -0.30 | need >= 2 |
|---|---|---|---|---|---|---|---|---|---|
| 2022 | top5 | 2,499 | 1 | 5 (5 ep) | 161 (55 ep) | 83 (13 ep) | 14 (8 ep) | **327 (89 ep)** | 2,157 |
| 2023 | top5 | 2,791 | 0 | 6 (6 ep) | 61 (29 ep) | 28 (18 ep) | 5 (4 ep) | **175 (70 ep)** | 2,611 |
| 2024 | top5 | 2,916 | 1 | 7 (7 ep) | 52 (24 ep) | 46 (25 ep) | 4 (4 ep) | **218 (68 ep)** | 2,693 |
| 2025 | top5 | 2,677 | 0 | 5 (4 ep) | 71 (33 ep) | 15 (8 ep) | 5 (2 ep) | **154 (69 ep)** | 2,518 |
| 2026 | top5 | 1,790 | 0 | 5 (4 ep) | 80 (33 ep) | 6 (3 ep) | 2 (2 ep) | **178 (58 ep)** | 1,610 |

ranking (both definitions, every year): core s1h > thrust lag > thrust s1h > core s5m > thrust s5m >
composite. composite is never binding on its own (0–1 bars/yr): it is 0.1·s1m + 0.6·s5m + 0.3·s1h, so
once the s5m and s1h conditions pass it passes too. that is why `cmax -0.30` and `cmax -0.25` give
identical simulations, and why `t_s5 -0.40` and `t_s5 -0.30` are identical (the composite, plus the
long-side noise gate, caps s5m at about -0.38 for any bar that can pass).

base rate — what each condition EXCLUDES (FREE bars where every other condition of that window passes
and only this one fails; opp_pnl = simplified exit stack, 3 bps adverse each way):

| year | condition | bars | episodes | days | mean opp | median | hit | top5 share | TAKEN mean opp (ref) | ALL FREE mean (ref) |
|---|---|---|---|---|---|---|---|---|---|---|
| 2022 | core: only s1h fails | 3,471 | 900 | 432 | +0.013 % | -0.100 % | 42.2 % | 9.4 % | +0.129 % | -0.092 % |
| 2023 | core: only s1h fails | 2,405 | 672 | 301 | -0.078 % | -0.080 % | 41.5 % | 7.3 % | -0.083 % | -0.093 % |
| 2024 | core: only s1h fails | 2,516 | 697 | 329 | -0.156 % | -0.170 % | 35.1 % | 8.7 % | -0.085 % | -0.052 % |
| 2025 | core: only s1h fails | 2,062 | 585 | 323 | -0.085 % | -0.095 % | 39.1 % | 7.5 % | -0.065 % | -0.042 % |
| 2026 | core: only s1h fails | 1,825 | 515 | 266 | -0.053 % | -0.088 % | 36.8 % | 9.8 % | -0.064 % | -0.123 % |
| 2022 | thrust: only lag fails | 1,089 | 348 | 254 | +0.146 % | -0.048 % | 47.3 % | 14.8 % | +0.129 % | -0.092 % |
| 2023 | thrust: only lag fails | 694 | 214 | 158 | -0.077 % | -0.089 % | 41.6 % | 8.8 % | -0.083 % | -0.093 % |
| 2024 | thrust: only lag fails | 702 | 218 | 165 | -0.206 % | -0.167 % | 35.9 % | 7.4 % | -0.085 % | -0.052 % |
| 2025 | thrust: only lag fails | 765 | 235 | 182 | -0.090 % | -0.100 % | 39.6 % | 9.3 % | -0.065 % | -0.042 % |
| 2026 | thrust: only lag fails | 624 | 202 | 158 | -0.186 % | -0.245 % | 32.2 % | 12.8 % | -0.064 % | -0.123 % |

pooled 2022–2026 exclusion sets:

| condition | bars | episodes | days | mean opp | median | hit | mean full-stack | mean ret-to-11:55 |
|---|---|---|---|---|---|---|---|---|
| core: only s1h <= -0.30 fails | 12,279 | 3,369 | 1,651 | -0.066 % | -0.103 % | 39.3 % | -0.068 % | -0.028 % |
| thrust: only lag >= 0.10 fails | 3,874 | 1,217 | 917 | -0.058 % | -0.103 % | 40.3 % | -0.057 % | -0.066 % |
| thrust: only s1h <= 0.0 fails | 2,041 | 573 | 216 | -0.149 % | -0.130 % | 37.3 % | -0.152 % | -0.031 % |
| thrust: only s5m <= -0.50 fails | 535 | 418 | 247 | -0.147 % | -0.144 % | 32.5 % | -0.165 % | -0.122 % |
| core: only s5m <= -0.40 fails | 338 | 203 | 133 | +0.027 % | -0.107 % | 42.6 % | +0.037 % | +0.109 % |
| thrust: only composite fails | 19 | 17 | 16 | +0.106 % | -0.189 % | 47.4 % | -0.165 % | +0.135 % |

the excluded sets are the market's base rate. the one sub-bucket that looks good — `core s1h in
(-0.15, 0]`, 2022 +0.22 %, 2023 +0.03 % — is -0.13 % / -0.17 % / -0.18 % in 2024–2026. the
"thrust lag" set is +0.146 % in 2022 and negative in all four later years. all four outcome measures
(opp_pnl, full-stack, return-to-11:55, 90-min MFE) agree.

### 3. loosening simulations: nothing is net positive across years

simulator: one position per ticker, the exit-fill bar blocked by cooldown, both noise reject gates,
full exit stack (hard stop 2.5 %, score exit composite >= +0.30, max-hold 75/90/120, session close
11:55), next-bar-open fills, $3,600/position whole shares. calibration of the base variant against the
actual trade files: entry bars match 100 % (2022–2025) and 205/207 (2026; two one-bar offsets from
4-decimal rounding at a gate boundary), exit bars 100 % / 100 % / 100 % / 243/244 / 204/207.

| year | actual trades / pnl | sim base trades / pnl | diff |
|---|---|---|---|
| 2022 | 414 / +3,654 | 414 / +3,639 | -0.4 % |
| 2023 | 209 / +479 | 209 / +503 | +5.0 % |
| 2024 | 212 / +395 | 212 / +429 | +8.6 % |
| 2025 | 244 / +699 | 244 / +693 | -0.9 % |
| 2026 | 207 / +665 | 207 / +669 | +0.6 % |

engine cost model (the sweep's, favourable to shorts). P&L per year (delta vs base):

| variant | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | vs base | years better | trades | WR |
|---|---|---|---|---|---|---|---|---|---|---|
| base | +3,639 | +503 | +429 | +693 | +669 | +5,933 | | | 1,286 | 50.6 % |
| composite -0.35 -> -0.30 (= -0.25) | -119 | -5 | +44 | +13 | -28 | +5,838 | -95 | 2/5 | 1,293 | 50.4 % |
| thrust s5m -0.50 -> -0.40 (= -0.30) | +115 | -6 | +128 | +222 | -31 | +6,361 | +428 | 3/5 | 1,418 | 50.9 % |
| core s5m -0.40 -> -0.30 | -46 | -124 | +25 | +12 | +30 | +5,830 | -103 | 3/5 | 1,332 | 50.1 % |
| lag 0.10 -> 0.05 | +171 | -6 | -101 | +87 | -144 | +5,940 | +7 | 2/5 | 1,321 | 49.9 % |
| lag 0.10 -> 0.00 | +60 | +23 | -100 | +120 | -119 | +5,917 | -16 | 3/5 | 1,363 | 50.4 % |
| thrust s1h 0.0 -> +0.10 | +27 | -150 | -53 | -21 | +65 | +5,801 | -132 | 2/5 | 1,335 | 50.4 % |
| thrust s1h 0.0 -> +0.20 | -183 | +38 | -29 | -66 | +205 | +5,898 | -35 | 2/5 | 1,381 | 50.6 % |
| core s1h -0.30 -> -0.15 | +262 | -50 | +314 | -16 | -40 | +6,403 | +470 | 2/5 | 1,733 | 48.4 % |
| core s1h -0.30 -> 0.00 | +267 | -34 | +225 | -94 | -31 | +6,266 | +333 | 2/5 | 1,856 | 47.8 % |
| both windows, s1h dropped | -166 | +164 | +571 | -325 | +245 | +6,422 | +489 | 3/5 | 2,063 | 47.3 % |
| composite -0.30 + core s1h -0.15 | +259 | -36 | +332 | -23 | -2 | +6,463 | +530 | 2/5 | 1,744 | 48.3 % |
| reject gates removed | +46 | +1 | -111 | +6 | +30 | +5,905 | -28 | 4/5 | 1,289 | 50.5 % |

adverse cost model (3 bps + $0.005 charged against the short each way):

| variant | 2022 | 2023 | 2024 | 2025 | 2026 | 5y | vs base | years better |
|---|---|---|---|---|---|---|---|---|
| base | +1,860 | -442 | -474 | -478 | -291 | +175 | | |
| thrust s5m -> -0.40 | -14 | -175 | +5 | +96 | -126 | -39 | -214 | 2/5 |
| lag -> 0.00 | -101 | -82 | -173 | -48 | -166 | -395 | -570 | 0/5 |
| core s1h -> -0.15 | -208 | -550 | -144 | -149 | -314 | -1,190 | -1,365 | 0/5 |
| both windows, s1h dropped | -979 | -589 | -180 | -727 | -216 | -2,516 | -2,691 | 0/5 |
| composite -0.30 + core s1h -0.15 | -209 | -546 | -133 | -161 | -285 | -1,159 | -1,334 | 0/5 |
| reject gates removed | +42 | 0 | -87 | +6 | +27 | +163 | -12 | 3/5 |

the marginal trades each loosening adds (engine cost model): core s1h -0.15 adds 1,181 trades over
five years averaging **+$3.0/trade**; thrust s5m -0.40 adds 250 at +$3.0; both-s1h-dropped adds 1,721
at +$3.1; lag 0.00 adds 612 at +$4.0. the cost-model swing is $4.3–4.8/trade (engine minus adverse,
per year: 4.30 / 4.52 / 4.26 / 4.80 / 4.64). every added trade is worth less than the slippage it
is being credited with. "reject gates removed" adds 24 trades in five years (-$65): at current
thresholds the noise gates are redundant with the windows (they blocked exactly 1 window-qualifying bar
in 2025), so "GATED" in the category table is really "not signalled".

drawdown: the volume loosenings raise max drawdown — core s1h -0.15: 2023 411 -> 572, 2025 681 -> 868,
2026 535 -> 683; both-s1h-dropped 2025 681 -> 1,149.

### 4. tightening: nothing robust either, but `core s1h -0.30 -> -0.40` is the only candidate worth a follow-up

| variant | 2022 | 2023 | 2024 | 2025 | 2026 | 5y engine | years better | 5y adverse | years better | trades | WR | PF by year (base 1.57/1.18/1.14/1.20/1.26) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| composite -0.35 -> -0.40 | -88 | -264 | -159 | +80 | -102 | -533 | 1/5 | -184 | 2/5 | 1,216 | 50.6 % | 1.59/1.09/1.09/1.23/1.23 |
| composite -0.35 -> -0.45 | -445 | -140 | -836 | -149 | -252 | -1,822 | 0/5 | -632 | 3/5 | 1,011 | 50.0 % | 1.59/1.17/0.87/1.19/1.20 |
| s1h <= -0.30 on both windows (= core only) | -2,050 | -401 | -352 | -620 | +324 | -3,099 | 1/5 | -864 | 2/5 | 776 | 48.2 % | 1.38/1.06/1.04/1.03/1.95 |
| thrust s5m -0.50 -> -0.60 | -315 | -171 | -504 | -355 | +166 | -1,179 | 1/5 | -530 | 1/5 | 1,122 | 50.3 % | 1.57/1.14/0.98/1.11/1.40 |
| core s5m -0.40 -> -0.50 | -190 | +236 | -262 | +88 | -296 | -424 | 2/5 | +249 | 3/5 | 1,147 | 50.9 % | 1.60/1.32/1.06/1.24/1.16 |
| lag 0.10 -> 0.15 | -58 | -34 | -39 | -66 | +210 | +13 | 1/5 | +159 | 1/5 | 1,253 | 50.9 % | 1.57/1.18/1.13/1.18/1.38 |
| **core s1h -0.30 -> -0.40** | -319 | +244 | -20 | +181 | -191 | -105 | 2/5 | **+959** | 3/5 | 1,078 | 52.5 % | **1.63/1.37/1.16/1.31/1.22** |
| thrust window only | -781 | +271 | +102 | +151 | -285 | -542 | 3/5 | +645 | 3/5 | 1,054 | 52.0 % | 1.56/1.39/1.22/1.32/1.18 |

`core s1h -0.40` raises PF in 4 of 5 years and win rate to 52.5 %, and is the best variant under honest
costs (+959, positive delta in 2023/2024/2025), but it loses P&L in 3 of 5 years under the engine's
cost model (2022 -319, 2026 -191) and cuts trades 16 %. the "thrust only" diagnostic says the same
thing from the other side: the strong-core window is the lower-quality half (core-only PF 1.38 / 1.06
/ 1.04 / 1.03 / 1.95 vs thrust-only 1.56 / 1.39 / 1.22 / 1.32 / 1.18). this is a quality-vs-volume
trade-off with mixed signs, not a config change the evidence forces.

### 5. the backtest credits shorts with favourable slippage (~$4.5/trade); the honest short book is ~flat

`crates/backtest/src/replay.rs` applies `adjust_entry_price` (raw + 3 bps + $0.005) to every entry
fill and `adjust_exit_price` (raw - 3 bps - $0.005) to every exit fill with no direction branch
(lines 276, 289). for a short that is sell-high / buy-low. verified in the trade files: e.g.
2025-01-02 AAPL entry fill 245.8213 vs bar open 245.7426 (+3.2 bps), exit fill 244.8715 vs open
244.95 (-3.2 bps), and `pnl` = size x (entry - exit) includes it. impact on the base strategy:

| cost model | 2022 | 2023 | 2024 | 2025 | 2026 | 5y |
|---|---|---|---|---|---|---|
| engine (as swept / promoted) | +3,639 | +503 | +429 | +693 | +669 | +5,933 |
| zero cost (midpoint) | ~+2,750 | ~+30 | ~-20 | ~+110 | ~+190 | ~+3,050 |
| adverse (honest) | +1,860 | -442 | -474 | -478 | -291 | +175 |

this is outside the question asked but it changes the frame for it: the live paper account pays real
spread, so the "positive in five of five years" evidence for v15 is really "positive in 2022, roughly
break-even after costs in 2023–2026". the fix is in the replay cost model (code), not config; the
brief says to avoid code recommendations, so this is flagged, not prescribed.

---

## method

* **opportunity labelling** (`label_bars.py`): every 09:30–11:29 ET bar on every ticker-day
  (120k bars/yr, 83k in 2026) gets a hypothetical short filled at the next bar's open and managed by
  the simplified stack from the brief: hard stop 2.5 % adverse, max hold 90 min (75 if losing at the
  check, 120 if winning), forced flat at 11:55 (signal 11:55, fill 11:56), marks at bar close, 3 bps +
  $0.005 adverse each way -> `opp_pnl_pct`. also `opp_full_pct` (same + score exit at composite >= +0.30
  + breakeven), `ret_1155` (next-open to 11:55 close, no costs), `mfe90` / `mae90`. paths from
  `data/bars/<TICKER>.csv`; every tick row matched a bar row (0 mismatches).
* **categories** from the tick dump: TAKEN = `event == open`; IN_POSITION = `position == short`
  before the bar; GATED = `blocked_by` non-empty (entry_cooldown, reject_gate:1m noise filter [short]);
  FREE = none of those. the reject gates are an `entry_reject_gate` action in the config (v11/v13):
  reject when 1m leads (long variant) or lags (short variant) both other timescales by 0.15 while 5m is
  within +/-0.35. my re-evaluation of the two windows reproduces the engine's `entry_reason` on all
  1,286 entry bars and finds 0 FREE bars that pass a window (2 in 2026 at a rounding boundary).
* **failing conditions** evaluated from composite / s1m / s5m / s1h with the exact v15 thresholds;
  `near_miss` was used only as a cross-check (it is empty when composite fails). the 84 "near_miss
  with composite failing" rows in 2022 are the first trading day, which has no 1m/5m scores (no
  warm-up) — excluded as NOSCORE.
* **episodes** = runs of consecutive qualifying bars on one ticker-day.
* **simulator** (`simulate.py`): see finding 3; base reproduces the trade files bar-for-bar. the
  engine's breakeven monitor never produced an exit in five years of trade files, so it is not modelled.

## per-category expectancy (all 09:30–11:29 bars)

| year | category | n | mean opp | median | hit | mean full-stack | mean ret-11:55 |
|---|---|---|---|---|---|---|---|
| 2022 | TAKEN | 414 | +0.129 % | -0.047 % | 48.1 % | +0.146 % | +0.145 % |
| 2022 | IN_POSITION | 30,653 | -0.009 % | -0.056 % | 46.1 % | -0.005 % | +0.060 % |
| 2022 | GATED (noise) | 37,435 | -0.094 % | -0.13 % | 37.8 % | -0.084 % | -0.033 % |
| 2022 | FREE | 51,518 | -0.092 % | -0.131 % | 38.2 % | -0.089 % | -0.039 % |
| 2023 | TAKEN | 209 | -0.083 % | -0.102 % | 41.1 % | -0.056 % | -0.093 % |
| 2023 | FREE | 57,599 | -0.093 % | -0.096 % | 37.5 % | -0.074 % | -0.046 % |
| 2024 | TAKEN | 212 | -0.085 % | -0.097 % | 36.8 % | -0.043 % | -0.119 % |
| 2024 | FREE | 57,842 | -0.052 % | -0.086 % | 38.4 % | -0.047 % | +0.004 % |
| 2025 | TAKEN | 244 | -0.065 % | -0.096 % | 38.9 % | -0.036 % | -0.026 % |
| 2025 | FREE | 55,290 | -0.042 % | -0.087 % | 37.8 % | -0.063 % | +0.026 % |
| 2026 | TAKEN | 207 | -0.064 % | -0.107 % | 38.2 % | -0.045 % | -0.027 % |
| 2026 | FREE | 37,424 | -0.123 % | -0.112 % | 36.6 % | -0.074 % | -0.066 % |

under adverse costs the TAKEN bars themselves have negative mean expectancy in 2023–2026 (full table
with all gate categories in analysis.md section A). the IN_POSITION very-successful bars (17–29 % of
the top 5 %) are not missed — the engine was already short that ticker.

## time of day and ticker (FREE top-5 % missed winners)

| year | 09:30–09:39 | 09:40–09:59 | 10:00–10:29 | 10:30–10:59 | 11:00–11:29 | AMZN | AAPL | NVDA | MSFT |
|---|---|---|---|---|---|---|---|---|---|
| 2022 | 754 (11.7 %) | 721 (8.3 %) | 611 (5.4 %) | 289 (2.4 %) | 124 (0.9 %) | 711 (5.7 %) | 362 (2.7 %) | 1,205 (9.7 %) | 221 (1.7 %) |
| 2023 | 682 (9.8 %) | 919 (9.5 %) | 869 (6.8 %) | 261 (2.0 %) | 60 (0.4 %) | 852 (6.0 %) | 261 (1.8 %) | 1,211 (8.5 %) | 467 (3.2 %) |
| 2024 | 678 (9.9 %) | 783 (8.5 %) | 765 (5.8 %) | 533 (3.9 %) | 157 (1.0 %) | 504 (3.5 %) | 435 (3.0 %) | 1,652 (11.4 %) | 325 (2.3 %) |
| 2025 | 548 (8.1 %) | 703 (7.8 %) | 709 (5.7 %) | 465 (3.6 %) | 252 (1.8 %) | 757 (5.5 %) | 515 (3.7 %) | 1,129 (8.3 %) | 276 (2.0 %) |
| 2026 | 547 (11.7 %) | 529 (8.9 %) | 403 (5.0 %) | 253 (2.8 %) | 58 (0.6 %) | 388 (4.3 %) | 247 (2.5 %) | 793 (8.4 %) | 362 (4.1 %) |

(count of winners, and winners as a share of FREE bars in that bucket.)

* the first 10 minutes carry ~8–12 % winner rates vs 0.4–1.8 % after 11:00, and NVDA carries 2–4x the
  rate of MSFT/AAPL. this is the volatility profile, not a scoring artefact: a top-5 % bar needs >= 1.1–1.7 %
  in <= 120 min, which early bars and NVDA can deliver and 11:00 MSFT cannot. the engine's own entries
  have the same shape (50 % of TAKEN bars are 09:30–09:39, NVDA is the most-traded ticker) and their
  mean opp_pnl in the first 10 minutes is no better than later (2022 +0.15 %, 2023 -0.15 %, 2024
  -0.12 %, 2025 -0.02 %, 2026 -0.06 %).
* the exclusion sets show no first-10-minute edge either: "only core s1h fails" first-10 mean -0.053 % /
  hit 39.0 % vs later -0.072 % / 39.4 %; "only lag fails" -0.071 % / 40.4 % vs -0.043 % / 40.1 %. "scores
  not settled yet" is not where the missed winners come from.

## what the evidence does NOT support

* it does not support loosening any window condition. the best five-year sums (+$430–530 at the
  engine's cost model) come from 2–3 years, are smaller than the slippage credit on the added trades,
  and are negative in 4–5 years under honest costs.
* it does not support dropping the s1h conditions ("either window with s1h dropped"): +$489 engine /
  -$2,691 adverse, 2025 -325 engine, max drawdown 2025 681 -> 1,149.
* it does not support loosening composite or s5m independently of each other: composite is implied by
  the other conditions and the noise gate; `t_s5 -0.40` = `t_s5 -0.30` and `cmax -0.30` = `cmax -0.25`
  are bit-identical.
* it does not support removing the reject gates (24 trades in five years, -$65) — nor keeping them for
  edge; they are inert at current thresholds.
* it does not support a tightening either; `core s1h -0.40` is the only variant with a consistent
  quality signal (PF up 4/5 years, best under honest costs) but it is P&L-negative in 3/5 years under
  the sweep's cost model. worth a follow-up with the corrected cost model, not a promotion.
* it does not support "the engine misses great shorts because of X": the great shorts are not
  signalled by any timescale score (median s5m of the missed winners is positive).

## what I could not determine

* whether live alpaca paper fills are adverse by ~3 bps; the cost-model finding is about the replay
  and the sweep numbers, not the live account.
* intra-bar behaviour: stops and marks use bar closes (as the engine does); a 2.5 % stop that would
  have been hit intrabar is not modelled.
* whether the 2023–2026 negative honest-cost expectancy is regime or noise — five annual points, no
  significance test; the sign consistency across years is the only test used.
* the 5 % of 2022 residual P&L mismatch (sum |err| $188 across 414 matched trades) — fills match to
  the bar, so it is a sub-cent price/fee difference, not a logic difference.
* combinations beyond the listed variants (one combination tested: composite -0.30 + core s1h -0.15,
  +530 engine / -1,334 adverse, 2/5 years).

## files

all under `/tmp/claude-1000/-home-dylmet-Projects-galactic-trading-firm/48685858-bad9-4000-a57f-b038fffcc726/scratchpad/missed/`:

| file | what |
|---|---|
| `common.py` | loaders, ET conversion, window / reject-gate evaluation, exit simulator, engine mechanics notes |
| `label_bars.py` | step 1–2: `python3 label_bars.py <year>` -> `bars_<year>.csv` (one row per 09:30–11:29 bar) |
| `analyze.py` | steps 2, 3, 6 -> `analysis.md` (all tables, every year, every condition, distance buckets) |
| `score_dist.py` | score distributions of missed winners -> `score_dist.md` |
| `simulate.py` | steps 4–5: variants x years x cost models -> `sim_results.csv`, `sim_calibration.txt`, `sim_trades_base_<year>.csv` |
| `pivot_sim.py` | -> `sim_tables.md` (per-variant tables incl. marginal added/dropped trades, PF, drawdown) |

re-run: `for y in 2022 2023 2024 2025 2026; do python3 label_bars.py $y; done; python3 analyze.py; python3 score_dist.py; python3 simulate.py; python3 pivot_sim.py` (stdlib only, ~3 min on 12 cores if the label runs are backgrounded).
