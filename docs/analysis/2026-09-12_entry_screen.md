# entry-feature screen, round two — 28 candidate features vs five years of short-entry labels

date 2026-09-12. offline, python stdlib. scripts and outputs in `scratchpad/screen/` (see §9).
data: `data/scr16_<year>_ticks.csv` (564,474 bars 09:30–11:29 ET, AMZN/AAPL/NVDA/MSFT, 2022–2026-09-10)
joined to `data/labels/bars_<year>.csv` on (date, ticker, ts). **join rate 100.00 % in every year** (120,474 / 120,000 / 120,960 / 120,000 / 83,040 rows).

## 0. the answer in four lines

1. **no feature works on its own.** every one of the 28 features (and every bucket of them) has negative mean `opp_pnl` pooled, and every one loses money simulated as a standalone short window under v16's exit stack. the v16 window is the only positive standalone trigger in the set.
2. **two conditions on the v16 windows pass the bar (keep ≥ 40 % of trades, raise the per-trade mean in ≥ 4 of 5 years):** *require SPY session return inside ±0.2 %* (PF 1.12 → 1.33, keeps 64 %) and *exclude bars where price is already > 1 % below the prior-day low* (PF 1.12 → 1.31, keeps 66 %). both have smooth threshold sensitivity, i.e. they are not knife-edge picks.
3. **one pair beats both parents in 5/5 years:** SPY flat ±0.2 % **and** VPIN top quintile (raw ≥ 0.217): +2,769 over 468 trades, PF 1.64, every year positive (2023 +31). robust to the SPY band (±0.3 % → PF 1.50) and the VPIN cut (0.18 → 1.53, 0.26 → 1.63).
4. **the bearish candle patterns are confirmations, not early signals.** when they fire while the 5-minute score is still bullish (where the best shorts live) they are at or below the base rate; when they fire with s5m ≤ −0.40 they merely match the window. none survives as a condition.

caveat on (2)/(3): the `cross_1m` indicator was **null on every bar** of the dump, so the SPY features were recomputed offline from `data/bars/SPY.csv` (§6). the live binary has no cross feed yet; a real-replay test needs `--cross-index SPY` to actually populate `MarketState.cross`.

## 1. ranked: features worth a real-replay test (≤ 8; slot 8 deliberately empty)

per-year cells are **mean `opp_pnl` % on the v16-window bars that satisfy the condition / episodes** (episode = run of consecutive firing bars in one ticker-day). the v16 window itself: **+0.11 / 1,655 · −0.04 / 552 · −0.13 / 691 · −0.06 / 762 · −0.06 / 657** (bar-level, label exit stack). "sim" = calibrated simulation with v16's exit stack, $3,600 per position, honest costs (v16 itself: +1,390 / −158 / +241 / +101 / +149 = **+1,723, 1,179 trades, PF 1.12**; real replay +1,380 / −177 / +229 / +99 / +153, 1,179 — within 2 %).

| # | feature (as condition on v16 windows) | 2022 | 2023 | 2024 | 2025 | 2026 | yrs mean ↑ | sim P&L per year (trades) | 5y sim | keep | PF | what to test |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | **SPY session return in (−0.2, +0.2) % — require** | +0.43 / 537 | −0.01 / 388 | −0.06 / 398 | +0.10 / 347 | −0.14 / 356 | 4 / 5 | +1,676 (221) · −148 (137) · +202 (135) · +581 (131) · +430 (128) | **+2,741** | 64 % | **1.33** | added condition; test band 0.2 % and 0.3 % (±0.3 %: +3,027 / 876 trades / PF 1.31) |
| 2 | **price > 1 % below prior-day low (`pdl_5m.dist_low_pct` < −1) — exclude** | +0.19 / 971 | +0.06 / 317 | −0.04 / 353 | +0.10 / 377 | −0.08 / 310 | 4 / 5 | +1,529 (284) · +266 (113) · +286 (127) · +687 (136) · −269 (118) | **+2,498** | 66 % | **1.31** | added exclusion; test −1 % and −1.5 % (−1.5 %: +2,380 / 954 / 1.23; −0.5 % is too tight, 2026 −576) |
| 3 | **VPIN top quintile (`vpin_1m.raw_vpin` ≥ 0.217) — require** | +0.18 / 614 | +0.08 / 263 | −0.18 / 315 | −0.08 / 377 | +0.06 / 301 | 3 / 5 | +1,314 (272) · −66 (121) · +148 (145) · +44 (171) · +576 (128) | +2,016 | 71 % | 1.22 | **only as the pair with #1** (§2) — alone it misses the 4/5 bar |
| 4 | gap ≤ −1 % (`gap_5m.gap_pct`) — exclude | +0.01 / 823 | +0.12 / 293 | −0.02 / 359 | −0.13 / 311 | +0.03 / 386 | 4 / 5 (sim) | +583 (212) · +48 (95) · +587 (112) · +240 (102) · +189 (109) | +1,647 | 53 % | 1.27 | alternative to #2, not alongside it (pair is redundant, 0/5). gives back most of 2022. related: require gap in (−1, +0.3) % → +1,497 / 444 / PF 1.35, 5/5 positive |
| 5 | relative volume top quintile (`rvol_1m.rvol` ≥ 1.13) — exclude | +0.13 / 1,842 | −0.04 / 603 | −0.12 / 740 | −0.04 / 849 | −0.06 / 721 | 4 / 5 (sim), 3 / 5 (bars) | +1,387 (402) · +4 (161) · +136 (185) · +181 (217) · +389 (174) | +2,097 | 97 % | 1.16 | cheap add-on: removes 40 trades for +374; flips 2023 to flat. expect small |
| 6 | SPY session ≤ −0.5 % — exclude (weaker form of #1) | +0.24 / 1,095 | −0.05 / 516 | −0.06 / 603 | +0.09 / 555 | −0.04 / 578 | 4 / 5 (bars), 3 / 5 (sim) | +1,655 (336) · −229 (162) · +290 (176) · +225 (191) · +45 (169) | +1,986 | 88 % | 1.16 | only if #1 is rejected on replay |
| 7 | prior-low distance in [0, 0.5) % — exclude | +0.10 / 1,552 | −0.04 / 529 | −0.12 / 657 | −0.06 / 730 | −0.06 / 617 | 5 / 5 (sim), 2 / 5 (bars) | +1,510 (400) · −76 (162) · +258 (184) · +108 (216) · +215 (171) | +2,015 | 96 % | 1.15 | probably noise (+292 from 46 trades); listed because it is 5/5 in sim — do not test alone |

feature-alone context (screen 1) for the same features, mean `opp_pnl` % on **all** bars where they fire vs base (base −0.072 / −0.098 / −0.068 / −0.050 / −0.110):
#1 −0.03 / −0.08 / −0.07 / −0.06 / −0.12 (5,008 → 2,736 episodes/yr) · #2's excluded set −0.08 / −0.10 / −0.17 / −0.13 / −0.08 (the worst bucket to short into) · #3 −0.04 / −0.08 / −0.07 / −0.03 / −0.09 (beats base 5/5 yet pooled −0.061) · #4's excluded set −0.02 / −0.14 / −0.19 / −0.07 / −0.16. none is a trigger; they are filters.

reading: the two strong conditions say the same thing from two sides — **short idiosyncratic weakness, don't chase a market-wide or already-extended move.** the window's 2022 profit came largely from big gap-down / SPY-down mornings (hence #4 gives back 2022); in 2023–2026 those mornings are where the window loses.

## 2. pairs (top-7 conditions, all 21 pairs as joint conditions on the windows; rule = per-trade mean beats *both* parents in ≥ 4 of 5 years)

| pair | beats both | yrs > 0 | sim per year (trades) | 5y | trades | PF |
|---|---|---|---|---|---|---|
| **SPY flat ±0.2 % + VPIN Q5 (require both)** | **5 / 5** | 5 / 5 | +1,294 (113) · +31 (93) · +180 (91) · +597 (88) · +668 (83) | **+2,769** | 468 | **1.64** |
| SPY flat + exclude pdl [0,0.5) | 4 / 5 | 4 / 5 | +1,615 (207) · −9 (131) · +227 (128) · +537 (125) · +431 (123) | +2,801 | 714 | 1.36 |
| exclude pdl < −1 % + exclude pdl [0,0.5) | 4 / 5 | 4 / 5 | +1,602 (270) · +371 (107) · +302 (118) · +694 (128) · −204 (109) | +2,765 | 732 | 1.38 |
| exclude gap ≤ −1 % + exclude or15 lower half | 4 / 5 | 5 / 5 | +448 (206) · +73 (91) · +604 (108) · +281 (99) · +193 (102) | +1,598 | 606 | 1.28 |

the second and third rows lean on the pdl [0,0.5) exclusion, which is noise-level on its own (§1 #7); treat them as SPY-flat and pdl < −1 % respectively. pairs that fail the rule but are worth knowing: SPY flat + exclude pdl < −1 %: 3/5, but positive every year, +2,312 / 482 / PF 1.47 (2026 only +30); SPY flat + exclude gap ≤ −1 %: 3/5, PF 1.53, 350 trades; exclude pdl < −1 % + exclude gap ≤ −1 %: 0/5 (they remove the same mornings).

robustness of the winning pair (not requested, cheap): SPY band ±0.3 % → +2,788 / 574 / PF 1.50; VPIN cut 0.18 → +2,762 / 531 / 1.53; cut 0.26 → +2,278 / 392 / 1.63. adding exclusion #2 (triple) → +2,342 / 278 trades / **PF 2.01**, every year positive, but ~1 trade a week.

standalone "both firing" without the window (SPY flat ∧ VPIN Q5): −27,416 over 13,590 trades, PF 0.65. the features filter; they do not trigger.

## 3. screen 1 — every feature alone (summary; full table in `tables.md`)

criterion: mean `opp_pnl` above the base rate in ≥ 4 of 5 years **and** pooled mean > 0 **and** ≥ 30 episodes every year. **zero features pass** — the "pooled mean > 0" leg fails for all 90 feature/bucket definitions (the v16 window itself is −0.002 % pooled under the label exit stack). closest:

| feature | yrs > base | episodes/yr (min–max) | pooled mean % | pooled episode-mean % | standalone sim (v16 exits) |
|---|---|---|---|---|---|
| cs_three_5m bear (three black crows, 5m) | 4 | 53–85 | −0.005 | +0.022 | −191 / 353 trades, 2 of 5 yrs positive |
| cs_cloud_1m bear (dark cloud, 1m) | 5 | 178–299 | −0.031 | −0.031 | −1,845 / 1,170 |
| gap up ≥ 0.3 % and fading below the open | 5 | 585–1,073 | −0.049 | −0.088 | −18,011 / 9,113 |
| pdl dist_low in [−1, −0.5) % | 5 | 801–1,477 | −0.052 | −0.087 | −2,637 / 1,847 |
| VPIN Q5 | 5 | 3,036–4,886 | −0.061 | −0.069 | −46,710 / 22,130 |
| v16 window (reference) | 3 | 552–1,655 | −0.002 | −0.021 | **+1,723 / 1,179** |

"beats base in 5 years" is a weak bar because the base rate is negative every year (−0.05 to −0.11 %); it says "less bad", not "good". the top-5 % share is 5.0–6.9 % for these features vs 11.2 % for the window — no feature concentrates the great shorts.

## 4. screen 5 — what the bearish candle patterns actually catch

mean `opp_pnl` % / episodes, pooled 2022–26, when the pattern fires **with s5m ≤ −0.40** vs **with s5m > 0** (per-year cells in `tables.md`):

| pattern | s5m ≤ −0.40 | s5m > 0 | s1m > 0 | on a v16 window bar |
|---|---|---|---|---|
| engulfing 1m / 5m | −0.044 / 4,704 · −0.018 / 1,316 | −0.080 / 17,371 · −0.085 / 3,917 | −0.084 · −0.084 | +0.066 / 1,090 · +0.005 / 400 |
| star 1m / 5m | −0.090 / 650 · −0.119 / 185 | −0.066 / 2,818 · −0.094 / 626 | −0.062 · −0.076 | +0.022 / 161 · −0.265 / 54 |
| three crows 1m / 5m | −0.023 / 506 · −0.001 / 221 | −0.080 / 544 · +0.123 / 40 | −0.224 · −0.012 | −0.076 / 87 · −0.304 / 89 |
| pin bar 1m / 5m | −0.023 / 1,067 · +0.010 / 174 | −0.076 / 7,440 · −0.099 / 2,012 | −0.069 · −0.110 | +0.038 / 192 · −0.090 / 50 |
| dark cloud 1m / 5m | −0.034 / 122 · −0.005 / 8 | −0.045 / 689 · −0.046 / 109 | −0.030 · −0.128 | +0.488 / 16 · −0.281 / 2 |
| harami 1m / 5m | −0.054 / 1,687 · **−0.347 / 235** | −0.066 / 10,466 · −0.117 / 2,822 | −0.074 · −0.115 | −0.040 / 343 · **−0.531 / 70** |
| inside-break 1m / 5m | −0.037 / 3,589 · −0.039 / 1,148 | −0.093 / 9,557 · −0.102 / 1,646 | −0.099 · −0.096 | +0.034 / 836 · +0.040 / 381 |
| three-bar reversal 1m / 5m | −0.052 / 7,212 · −0.038 / 2,152 | −0.081 / 23,591 · −0.064 / 5,170 | −0.084 · −0.075 | −0.020 / 1,705 · +0.028 / 625 |
| first reversal 1m / 5m | −0.047 / 1,985 · **+0.029 / 404** | −0.096 / 14,101 · −0.074 / 3,767 | −0.093 · −0.078 | +0.014 / 437 · −0.068 / 108 |

- in 16 of 18 cases the pattern is better when the 5m score is already ≤ −0.40 than when it is > 0, and in every case the "s5m > 0" cell is at or below the base rate. **the patterns confirm the score; they do not see the best shorts before it does** (those have a bullish 5m score — where every pattern is −0.05 to −0.12 %).
- the only positive cells are `cs_first_5m` bear with s5m ≤ −0.40 (+0.029 %, 404 episodes, but 3 of 5 years positive: +0.08 / +0.09 / −0.18 / +0.25 / −0.10) and `cs_pin_5m` (+0.010 %, 174 episodes). neither survives as a window condition (§5).
- `cs_harami_5m` bear after a 5m sell-off is the strongest *negative* signal in the whole set (−0.35 %, 235 episodes, negative in all five years; −0.53 % on window bars). excluding it from the window did not help in simulation (+1,620 vs +1,723): the entry simply shifts to the next window bar.

## 5. negative results (look good pooled or in one lens, fail per-year or on replay)

- **every feature as a standalone trigger** (§3): all negative. the worst standalone sets are the "obvious" ones — SPY ≥ +0.5 % (PF 0.44), gap up and holding (0.42), FOMC days (0.52), peers all green (0.50).
- **candle patterns as window conditions:** none reaches 4/5 with ≥ 40 % retention. cs_pin_1m require: PF 1.43 pooled but 169 trades and 3/5. cs_first_5m require: 4/5 but 85 trades. all pattern exclusions move 5-year P&L by < ±120 (noise).
- **opening range:** or15 below the range require: +1,222 / 736 / PF 1.17 pooled but 2/5 (2022-driven: +1,283 then −206 / −89 / −19 / +252). or30 below: +78. as exclusions 1/5.
- **OFI quintiles (1m and 5m):** nothing. best require ofi_1m Q4: 3/5, +927, PF 1.09. exclusions are noise.
- **peers red 3/3 require:** +1,042 / 650 / PF 1.14 pooled; 3/5 (2025 −153).
- **SPY 5-minute return < 0 as an exclusion:** bar-level screen says raised 4/5 (keep 59 %, pooled +0.049 vs −0.002) but the simulation is only +1,578 with 2/5 years positive — the entry shifts to a later bar. a screen-2 pass that fails screen 3.
- **below today's open (gap holding/fading) require:** +1,505 / 1,066, PF 1.12 pooled; 2/5 (2023 −485).
- **thin 5/5 buckets:** gap in (−0.3, +0.3) % require (172 trades, PF 1.51) and pdl distance [1, 2) % require (106 trades, PF 1.89, 9 trades/yr in 2025–26). too thin to test alone; the gap union (−1, +0.3) % gets 444 trades / PF 1.35 / 5-year +1,497 (below v16's total).
- **calendar:** v16 takes 32 window trades on FOMC days in five years (−33 total) and 45 on earnings-reaction days (−40): excluding them changes nothing. as standalone triggers FOMC and earnings days are among the worst sets (§3).
- **time-of-day** (reference): no 10-minute bucket beats base consistently (best 10:20–10:29, pooled −0.072 ≈ base).
- **2023** stays negative or flat in every single-condition variant; only the SPY-flat + VPIN pair (+31) and the pdl < −1 % exclusion (+266) turn it positive.
- **multiple comparisons:** ~90 feature definitions × require/exclude × 5 years; a 4/5 rule will be passed by chance by several. the 96–99 %-retention exclusions (#5, #7, or15 lower half, cs_engulfing_5m) are the most likely to be luck (deltas of +100–400 from 40–50 trades). #1 and #2 have the largest per-year effects and monotone threshold sensitivity, which is why they lead.

## 6. what could not be determined

1. **`cross_1m` (index session return, 5m/15m return, peers-red fraction) is null on all 564,474 bars** in the dump, despite `--cross-index SPY` in `scr16.args`. the four cross features were recomputed offline from `data/bars/` (SPY session return = close / today's 09:30 open − 1; 5m/15m = close vs close 5/15 bars back; peers red = fraction of the other three tickers below their own 09:30 open). whether the engine's `cross_context` computes `index_session_ret` identically, and why the dump had none, must be checked before a replay test; live has no cross feed at all (`cross_context.rs` header), so #1 cannot run live until it is wired.
2. **label exit stack ≠ v16.** the bar-level means (screens 1, 2, 5) use the label's v15 stack (75-min losing limit, no breakeven, no score exit); the simulation (screen 3, pairs) uses v16's (40 / 90 min, breakeven 0.5 %, score exit at composite ≥ 0.30) and matches the real replay within 2 %. the score exit could only be evaluated on bars ≤ 11:29 (no composite in the dump after that); the calibration says this is negligible.
3. **quintile edges are pooled over 2022–26** (VPIN raw 0.0402 / 0.0847 / 0.138 / 0.217; rvol 0.592 / 0.736 / 0.891 / 1.134; OFI 1m −0.524 / −0.172 / 0.186 / 0.546) — mild look-ahead in the bucket boundaries; a replay must use fixed thresholds (e.g. `vpin_1m.raw_vpin ≥ 0.217`, `rvol_1m.rvol ≥ 1.13`). the `rvol_1m` *score* is 0 on ~90 % of bars (thresholded), so the raw metadata was used.
4. three ticker-days were nulled for the gap/pdl features by an |gap| > 15 % rule: AMZN 2022-06-06 and NVDA 2024-06-10 (splits) and NVDA 2023-05-25 (a real +24 % earnings gap).
5. the screen-2 "keep" column counts episodes and can exceed 100 % for exclusions (removing bars splits a window run); trade retention in §1 is taken from the simulation instead.
6. episodes vs bars: e.g. VPIN Q5 fires on ~22,000 bars/yr but 3,036–4,886 episodes; the v16 window fires on 1,655 episodes in 2022 for 413 trades. every count above that matters for a decision is a trade count from the simulation or an episode count, never a bar count.

## 7. suggested replay order (one variant at a time, five-year cached replay, honest costs)

1. v16 + `cross_1m.index_session_ret` in (−0.2, +0.2) % — after confirming the indicator populates in the replay. also ±0.3 %.
2. v16 + `pdl_5m.dist_low_pct` ≥ −1.0 (exclusion). also −1.5.
3. v16 + (1) + `vpin_1m.raw_vpin` ≥ 0.217 — the 5/5 pair. expect ~470 trades / 5 y.
4. optionally (1) + (2), and (1) + (2) + VPIN (278 trades, PF 2.01 in sim — the frequency may be too low for a week-1 read).
5. skip: everything in §5; candle patterns in any role; rvol / pdl [0,0.5) exclusions unless bundled and free.

## 8. method

- `build_table.py`: one streaming pass per year over the 480 MB tick file; keeps date/ticker/ts/hm/open/scores/event/position + 39 feature columns from the `indicators` JSON; joins the label row (opp_pnl_pct, opp_reason, opp_full_pct, ret_1155, mfe90, mae90) on (date, ticker, epoch ts). output `feat_<year>.csv` (~45 MB/yr).
- `screen.py`: per year — features → boolean masks (90 definitions incl. references), v16 windows recomputed from composite/s1m/s5m/s1h (thrust: comp ≤ −0.35, s5m ≤ −0.50, s5m ≤ s1m − 0.10, s5m ≤ s1h − 0.10, s1h ≤ 0; core: comp ≤ −0.35, s5m ≤ −0.40, s1h ≤ −0.40; reject gate applied, `event` not used), screen 1/2/5 stats (bars, episodes, mean, median, hit, top-5 % share, lift, episode-first-bar mean), screen 3 simulation (one position per ticker, first firing bar, fill next open, nothing until the exit-fill bar, exit model from `scripts/analysis/missed/common.py::simulate_short` with v16 parameters and, secondarily, the label's).
- `summarize.py` → `tables.md` (every row of every screen); `rank_conditions.py` → retention / per-year lift ranking; `pairs.py` → 21 pairs + threshold sensitivity; `extras.py` → gap-band union, band/threshold robustness of the winning pair, the triple.

## 9. files

`scratchpad/screen/`: `build_table.py`, `screen.py`, `summarize.py`, `rank_conditions.py`, `pairs.py`, `extras.py`; `feat_2022..2026.csv`; `s1_feature_alone.csv`, `s2_conditional.csv`, `s3_sim.csv`, `s5_patterns.csv`, `s4_pairs.csv`, `s4_sens.csv`, `s4_extras.csv`; `tables.md` (full per-year tables), `rank_conditions.json`, `quintile_edges.json`, `meta.json`, `build_<year>.log`, `screen.log`. re-run: `python3 build_table.py <year>` ×5, then `screen.py`, `summarize.py`, `rank_conditions.py`, `pairs.py`, `extras.py`.
