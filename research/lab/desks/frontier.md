# frontier desk: fitting the levers that each pass on their own into one book (2026-11-01)

**Agent** `frontier`. **Cells** 39 distinct specs, 44 ledger runs (16 legacy screens + 11 noADX screens + 2 reproduction runs + 15 train runs).
No holdout date was run. No engine, DB or pipeline change was made, and nothing was committed.

**Answer, short.** The best cell that meets the brief's bar is **`frontier/na_bundle`**: train **+2,610 / 320 / PF 1.75**, ex-2022 PF 1.47,
sign-flip p 0.0007, every year positive, 92 trades a year, and on **2023_25h1 +1,167 / 201 / PF 1.47** (v18: +90 / 237 / 1.03).
It is the indicators desk's noADX row-21 cell (sqrt SPY band 10:00–11:00, thrust 1h ≤ −0.10, VPIN 0.20, `adx_14_1hr` off) plus two
additive levers: the stress-s15-core window (+258 / 33 on train) and the ×1.25 size tier at VPIN ≥ 0.26 (+357 on the same trades).
Against v18 the paired day bootstrap gives P(diff ≤ 0) = **0.006** on train and **0.005** on 2023_25h1. Two caveats bind:
(1) it reaches 90 trades a year only because the stress window trades the 2022 crash days. After 2022 it trades ~82 a year (71 in 2023, 89 in 2024, 41 in 2025h1).
(2) NVDA carries 70 % of the P&L (+1,837 / 145), and AAPL loses (−300 / 44 / PF 0.52).
The volume twin **`frontier/nb_v15`** (VPIN 0.15) trades 114 a year at PF 1.61. Its extra 138 trades are +871 in 2022 and about zero after 2022.

## setup and scaffold

- Generator `research/lab/patches/frontier/make.py NAME key=value…` writes `patches/frontier/NAME.json` and `specs/frontier/NAME.json`
  (keys: am_end, k, pm_end, t1h_am/t1h_pm, vpin, c5/c1, core_am/core_pm, band, tier/tier_min, stress, pdl, se, noadx; `_params` in each spec).
  Every cell is a single self-contained patch on config row 12.
- **Scaffold proven.** `frontier/scaffold` (v18 rebuilt as `_am` windows, 11:30 clock) matches `base/v18` on 2025h1 **trade for trade**
  (45 / 45, P&L diff 0, correlation 1.00). `frontier/sq05` is semantically identical to `research/timefilter/spy_sqrt_band.patch.json`.
  `frontier/sq05_1h15` reproduces cand_45 / row 21 on 2024h2_25h1 (+392 / 65 / PF 1.54). `frontier/na` matches
  `indicators/r21_v020_noadx_t10` on 2025h1 trade for trade (+271 / 34, diff 0). Its train numbers below are therefore the indicators desk's run.
- The stress window opens the session to 15:30 / 15:55. In every diff the AM/PM windows' trades are unchanged by it (`kept identical`).
  Only 1–2 trades change P&L, through cooldown or position overlap.
- Sizing tier: `indicator_tiered` on `vpin_1m.raw_vpin`, `base_fraction 0.36`, ×1.25 at ≥ 0.26, with spec arg `--max-position-pct 0.45`.
  A patch is applied *after* the CLI overrides (`main.rs` apply order), so the patch carries 0.36 itself and has no `session.max_position_pct`.
  At live 0.30 sizing this is the `research/sizing/size_vpin26_v1.patch.json` form (0.30 base, cap 0.40).
- Pitfall check (indicators desk): none of my cells relied on `disable` for an indicator. `noadx` re-declares `adx_14_1hr` with `enabled: false`.

## stage 1: one factor at a time on 2024h2_25h1, legacy (ADX on), around sq05

| cell | change | P&L | n | PF | dd | p | ex-best PF | tr/yr | per year (P&L/n/PF) |
|---|---|---|---|---|---|---|---|---|---|
| scaffold | v18 as _am windows | +59 | 97 | 1.05 | -343 | 0.449 | 0.927 | 97 | 24:-51/52/0.93 · 25:+110/45/1.20 |
| sq05 | sqrt band k0.05 10:00–11:00 (#44) | +313 | 103 | 1.24 | -309 | 0.244 | 1.239 | 103 | 24:+170/55/1.25 · 25:+142/48/1.24 |
| sq04 | k 0.04 | +51 | 98 | 1.04 | -408 | 0.447 | 0.995 | 98 | 24:-3/51/0.99 · 25:+54/47/1.09 |
| sq06 | k 0.06 | +243 | 107 | 1.19 | -332 | 0.2923 | 1.064 | 107 | 24:+204/58/1.30 · 25:+40/49/1.06 |
| sq05_1h10 | + thrust 1h ≤ −0.10 | +355 | 81 | 1.36 | -204 | 0.1847 | 1.245 | 81 | 24:+233/43/1.49 · 25:+122/38/1.25 |
| sq05_1h15 | + thrust 1h ≤ −0.15 (= row 21) | +392 | 65 | 1.54 | -286 | 0.1293 | 1.212 | 65 | 24:+86/34/1.21 · 25:+306/31/1.97 |
| sq05_1h20 | + thrust 1h ≤ −0.20 | +417 | 51 | 1.76 | -208 | 0.0887 | 1.505 | 51 | 24:+153/26/1.50 · 25:+265/25/2.07 |
| sq05_1h15am | 1h −0.15 am windows only | +376 | 72 | 1.45 | -323 | 0.16 | 1.133 | 72 | 24:+70/41/1.13 · 25:+306/31/1.97 |
| sq05_1h15pm | 1h −0.15 pm windows only | +327 | 97 | 1.28 | -254 | 0.2127 | 1.239 | 97 | 24:+185/49/1.33 · 25:+142/48/1.24 |
| sq05_v20 | VPIN 0.20 | +284 | 109 | 1.22 | -255 | 0.265 | 1.085 | 109 | 24:+226/57/1.36 · 25:+58/52/1.08 |
| sq05_v26 | VPIN 0.26 | +278 | 84 | 1.27 | -295 | 0.2353 | 1.191 | 84 | 24:+106/44/1.19 · 25:+172/40/1.38 |
| sq05_tier | size ×1.25 at VPIN ≥ 0.26 | +392 | 103 | 1.27 | -365 | 0.2323 | 1.236 | 103 | 24:+188/55/1.24 · 25:+204/48/1.30 |
| sq05_s15 | + stress-s15-core window | +353 | 108 | 1.27 | -313 | 0.2157 | 1.242 | 108 | 24:+166/57/1.24 · 25:+187/51/1.31 |
| sq05_nocoream | no 09:30–10:00 core window | +336 | 100 | 1.29 | -309 | 0.207 | 1.224 | 100 | 24:+156/55/1.22 · 25:+180/45/1.40 |
| sq05_c5_05 | core 5m ≤ −0.5 | +279 | 102 | 1.21 | -309 | 0.2737 | 1.233 | 102 | 24:+139/55/1.20 · 25:+139/47/1.23 |
| sq05_c1_05 | core 1h ≤ −0.5 | +342 | 99 | 1.30 | -309 | 0.2143 | 1.229 | 99 | 24:+160/55/1.23 · 25:+183/44/1.41 |

Reading (trade diffs vs sq05, `fdiff`):
- **sqrt band vs v18:** it adds 15 pm trades (+391, PF 9.7) and drops 9 late fixed-band thrusts (+137). On this window it is the biggest single lever (+254).
- **1h dial** is monotone in PF (1.24 → 1.36 → 1.54 → 1.76) and in volume (103 → 81 → 65 → 51 a year). At −0.10 the removed trades are −128 / 32 / PF 0.75.
  Most of the dial's work is in the AM thrust: pm-only −0.15 removes 6 trades for −14.
- **VPIN** 0.20 adds trades at PF ≈ 1.0, and 0.26 removes 34 trades at PF 0.89. It is a pure volume dial.
- **Tier** leaves the trades unchanged. The 69 tier trades run at PF 1.50 against PF 0.89 for the 34 below the tier, so +79 in the window.
- **Stress window** adds 5 trades (+39). The AM core window removed: −74 / 7. The core 5m and 1h dials change little.

## stage 2: the same levers on the noADX row-21 cell (`na`), 2024h2_25h1

The coordinator redirected me to build on `indicators/r21_v020_noadx_t10`, which is already train-confirmed. I did not run the legacy
r21 + pdl / score-exit cells. Their specs were deleted before running and none is in the ledger.

| cell | change | P&L | n | PF | dd | p | ex-best PF | tr/yr | per year (P&L/n/PF) |
|---|---|---|---|---|---|---|---|---|---|
| na | sq05 + noADX + 1h −0.10 + VPIN 0.20 (= indicators/r21_v020_noadx_t10) | +704 | 81 | 1.85 | -177 | 0.0463 | 1.804 | 81 | 24:+433/47/1.87 · 25:+271/34/1.80 |
| na_v18 | VPIN 0.18 | +687 | 92 | 1.67 | -211 | 0.0547 | 1.641 | 92 | 24:+394/53/1.70 · 25:+293/39/1.64 |
| na_v15 | VPIN 0.15 | +639 | 97 | 1.57 | -211 | 0.079 | 1.503 | 97 | 24:+388/54/1.63 · 25:+251/43/1.50 |
| na_k06 | k 0.06 | +576 | 87 | 1.63 | -190 | 0.091 | 1.465 | 87 | 24:+408/52/1.74 · 25:+167/35/1.47 |
| na_t05 | 1h −0.05 | +461 | 94 | 1.41 | -236 | 0.1423 | 1.055 | 94 | 24:+428/52/1.79 · 25:+33/42/1.05 |
| na_s15 | + stress window | +752 | 89 | 1.86 | -181 | 0.0463 | 1.865 | 89 | 24:+430/48/1.86 · 25:+322/41/1.86 |
| na_tier | + size tier | +809 | 81 | 1.84 | -211 | 0.0503 | 1.812 | 81 | 24:+485/47/1.86 · 25:+324/34/1.81 |
| na_pdl10 | + pdl ≥ −1.0 % | +553 | 42 | 3.08 | -145 | 0.0163 | 4.224 | 42 | 24:+337/26/2.70 · 25:+216/16/4.22 |
| na_seoff | score exit off (−1.1) | +688 | 81 | 1.82 | -177 | 0.0527 | 1.806 | 81 | 24:+417/47/1.83 · 25:+271/34/1.81 |
| na_se05 | score exit −0.5 | +688 | 81 | 1.82 | -177 | 0.0527 | 1.806 | 81 | 24:+417/47/1.83 · 25:+271/34/1.81 |
| na_nocoream | no am core window | +539 | 76 | 1.68 | -175 | 0.076 | 1.753 | 76 | 24:+321/45/1.64 · 25:+218/31/1.75 |

Reading (diffs vs na):
- **VPIN 0.18 / 0.15** add 19 / 29 trades at PF 0.59 / 0.97. This is the sub-floor population (HISTORY D5). Volume comes, edge does not.
- **k 0.06** adds 8 pm trades at PF 0.42. **1h −0.05** adds volume, but 2025h1 falls to PF 1.05.
- **Stress** adds 8 trades (+50, PF 2.4). **Tier** adds +105 on the same trades.
- **AM core window**: on the noADX cell it *pays* (+123 / 10, PF 2.4), unlike legacy, so it stays.
- **pdl ≥ −1 %** removes 43 trades at PF 1.36 (+208). It raises PF by cutting *profitable* trades, so it concentrates the book rather than improving it.
- **Score exit** −0.5 and off give identical results: the short composite never reaches +0.5 while a position is open. Off changes 5 trades from −79 to −95.

## stage 3: train (874 sessions) and 2023_25h1 (derived from the train trades; numbers match `lab.py compare`)

| cell | change | P&L | n | PF | dd | p | ex-best PF | tr/yr | per year (P&L/n/PF) | 2023_25h1 |
|---|---|---|---|---|---|---|---|---|---|---|
| base__v18 | promoted row 12 | +1179 | 324 | 1.32 | -469 | 0.061 | 1.031 | 93 | 22:+1089/87/2.36 · 23:-3/91/1.00 · 24:-17/101/0.99 · 25:+110/45/1.20 | +90/237/1.03 |
| indicators__r21_v020_noadx_t10 | na (indicators desk run; frontier/na reproduces it) | +1997 | 287 | 1.69 | -250 | 0.0017 | 1.437 | 82 | 22:+1056/94/2.40 · 23:+78/71/1.09 · 24:+592/88/1.60 · 25:+271/34/1.80 | +941/193/1.44 |
| na_s15 | na + stress | +2253 | 320 | 1.72 | -230 | 0.001 | 1.451 | 92 | 22:+1264/119/2.35 · 23:+78/71/1.09 · 24:+589/89/1.60 · 25:+322/41/1.86 | +990/201/1.45 |
| na_bundle | na + stress + tier | +2610 | 320 | 1.75 | -238 | 0.0007 | 1.468 | 92 | 22:+1443/119/2.43 · 23:+136/71/1.15 · 24:+650/89/1.58 · 25:+380/41/1.87 | +1167/201/1.47 |
| na_bundle_pm0 | bundle, pm thrust 1h 0 | +2477 | 329 | 1.67 | -274 | 0.0 | 1.388 | 94 | 22:+1442/120/2.41 · 23:+125/72/1.13 · 24:+530/96/1.41 · 25:+380/41/1.87 | +1035/209/1.39 |
| na00_bundle | bundle, 1h 0 everywhere, VPIN 0.217 | +2044 | 364 | 1.44 | -347 | 0.0123 | 1.205 | 104 | 22:+1353/124/2.06 · 23:+334/88/1.30 · 24:+290/104/1.19 · 25:+67/48/1.09 | +691/240/1.21 |
| na_bundle_v18 | bundle, VPIN 0.18 | +2627 | 353 | 1.67 | -328 | 0.0013 | 1.405 | 101 | 22:+1512/131/2.28 · 23:+54/78/1.05 · 24:+651/98/1.55 · 25:+410/46/1.75 | +1115/222/1.40 |
| na_seoff | na, score exit off | +2014 | 287 | 1.71 | -301 | 0.0017 | 1.404 | 82 | 22:+1125/94/2.71 · 23:+68/71/1.08 · 24:+550/88/1.54 · 25:+271/34/1.81 | +889/193/1.40 |
| na_pdl10 | na + pdl ≥ −1.0 % | +1562 | 170 | 2.11 | -145 | 0.0007 | 1.894 | 49 | 22:+651/58/2.68 · 23:+237/46/1.49 · 24:+458/50/1.98 · 25:+216/16/4.22 | +911/112/1.89 |

Paired day bootstrap vs `base/v18` (A − B, P(diff ≤ 0)):

| cell | train | 2023_25h1 | daily corr |
|---|---|---|---|
| na_bundle | +1431, **0.006** | +1077, **0.005** | 0.75 |
| nb_v15 | +1557, 0.011 | +973, 0.021 | 0.69 |
| na_bundle_v18 | +1448, 0.006 | +1025, 0.012 | 0.74 |
| na_bundle_pm0 | +1299, 0.010 | +946, 0.018 | 0.75 |
| na_s15 | +1075, 0.019 | +900, 0.017 | 0.74 |
| na00_bundle | +865, 0.029 | +601, 0.033 | 0.88 |
| na_pdl10 | +383, 0.271 | +821, 0.052 | 0.54 |
| na_seoff | +836, 0.042 | +800, 0.028 | 0.77 |
| na_bundle vs na (what the bundle adds) | +613, 0.004 | +226, 0.046 | 0.96 |
| nb_v15 vs na_bundle | +126, 0.381 | −104, 0.611 | 0.89 |

Mechanism (train diffs):
- **na vs v18** (indicators' cell, decomposed): v18 loses 186 trades (−22 / PF 0.99; after 2022: 2023 −201, 2024 −180, 2025h1 −104) and gains 149 new ones
  (+794 / PF 1.50, mostly pm thrusts +438 / 55 and the stricter AM thrusts +288 / 56). Those replaced AM thrusts are where v18's 2023–24 holes were.
- **Stress window**: +258 / 33 / PF 2.16, with **25 of 33 trades and +208 in 2022**. After 2022 it adds +50 / 8. It is a 2022 crash-day lever that is harmless otherwise.
- **Tier**: the 159 trades at VPIN ≥ 0.26 run at PF 1.96 and the 161 below at PF 1.52. ×1.25 on the better half is +357.
- **pm 1h at 0** (bundle_pm0) adds 15 pm thrusts at −104 / PF 0.55. The 1h dial matters in the pm too.
- **VPIN 0.15** (nb_v15) adds 138 trades / +899 (2022 +871; 2023 −37, 2024 +79, 2025 −15) and drops 61 trades (+776).
- **Score exit off**: 27 ScoreExit trades go from −525 to −507 if held. The exit fires on trades that are already losing, and holding them does not rescue
  them (+17 on train, −52 on 2023_25h1). ScoreExit's losses show selection, not a bad exit. Dead lever (HISTORY D2 holds).
- **pdl ≥ −1 %**: removes 125 trades at +377 / PF 1.23 (2022 +404, 2023 −256, 2024 +111, 2025 +117). It is a real 2023 filter (2023 PF 1.09 → 1.49),
  but at 49 trades a year it is far under the volume bar and makes fewer dollars than `na`. As a trade-for-volume swap it does not pay
  (PF 2.1 on 49 a year ≈ +446 a year vs +571 a year for `na`). I did not run the pdl neighbourhood (−0.5 / −1.5) on noADX: the cell fails the brief's bar and
  more pdl cells would only spend the multiple-comparison budget. The legacy-r21 pdl request was superseded by the noADX redirect.

## stage 4: neighbourhood of `na_bundle` on train (one step each way per parameter)

| cell | change | P&L | n | PF | dd | p | ex-best PF | tr/yr | per year (P&L/n/PF) | 2023_25h1 |
|---|---|---|---|---|---|---|---|---|---|---|
| na_bundle | centre | +2610 | 320 | 1.75 | -238 | 0.0007 | 1.468 | 92 | 22:+1443/119/2.43 · 23:+136/71/1.15 · 24:+650/89/1.58 · 25:+380/41/1.87 | +1167/201/1.47 |
| nb_k04 | k 0.04 | +2178 | 298 | 1.65 | -247 | 0.003 | 1.362 | 85 | 22:+1308/109/2.37 · 23:+108/66/1.12 · 24:+454/85/1.41 · 25:+308/38/1.75 | +870/189/1.36 |
| nb_k06 | k 0.06 | +2434 | 336 | 1.66 | -257 | 0.003 | 1.379 | 96 | 22:+1446/126/2.34 · 23:+117/73/1.12 · 24:+623/95/1.53 · 25:+249/42/1.53 | +988/210/1.38 |
| nb_t05 | 1h −0.05 | +1900 | 368 | 1.41 | -426 | 0.0193 | 1.198 | 105 | 22:+1246/129/1.91 · 23:+65/86/1.06 · 24:+471/104/1.32 · 25:+117/49/1.16 | +653/239/1.20 |
| nb_t15 | 1h −0.15 | +2190 | 269 | 1.75 | -230 | 0.0013 | 1.465 | 77 | 22:+1211/103/2.45 · 23:+250/57/1.37 · 24:+384/72/1.38 · 25:+344/37/1.80 | +979/166/1.47 |
| nb_v15 | VPIN 0.15 | +2735 | 397 | 1.61 | -267 | 0.0017 | 1.354 | 114 | 22:+1673/154/2.13 · 23:+212/87/1.21 · 24:+528/103/1.41 · 25:+322/53/1.47 | +1063/243/1.35 |
| na_bundle_v18 | VPIN 0.18 | +2627 | 353 | 1.67 | -328 | 0.0013 | 1.405 | 101 | 22:+1512/131/2.28 · 23:+54/78/1.05 · 24:+651/98/1.55 · 25:+410/46/1.75 | +1115/222/1.40 |
| nb_v217 | VPIN 0.217 | +2201 | 296 | 1.64 | -311 | 0.0023 | 1.394 | 85 | 22:+1237/109/2.25 · 23:+152/66/1.17 · 24:+436/84/1.38 · 25:+375/37/1.92 | +964/187/1.39 |
| nb_s10 | stress SPY ≤ −1.0 % | +2053 | 356 | 1.48 | -329 | 0.0097 | 1.296 | 102 | 22:+1199/140/1.84 · 23:+155/72/1.17 · 24:+497/96/1.38 · 25:+202/48/1.32 | +854/216/1.30 |
| nb_s20 | stress SPY ≤ −2.0 % | +2377 | 303 | 1.69 | -238 | 0.001 | 1.485 | 87 | 22:+1182/105/2.23 · 23:+136/71/1.15 · 24:+652/88/1.58 · 25:+406/39/1.99 | +1195/198/1.48 |

Plateau reading:
- **k** 0.04 / 0.05 / 0.06: PF 1.65 / 1.75 / 1.66.
- **thrust 1h** −0.05 / −0.10 / −0.15: 1.41 / 1.75 / 1.75 (the indicators desk's −0.20 without the bundle: 1.74).
- **VPIN** 0.15 / 0.18 / 0.20 / 0.217: 1.61 / 1.67 / 1.75 / 1.64.
- **stress** 1.0 / 1.5 / 2.0 %: 1.48 / 1.75 / 1.69.

So there is a plateau, PF ≈ 1.64–1.75 on train with 2023_25h1 PF 1.35–1.48, along k, VPIN, stress ≥ 1.5 and the 1h dial from −0.10 to −0.20.
It has two cliffs on the *loose* side: 1h −0.05 (the dead lever "loosening the 1h confirmations", D6) and stress 1.0 % (adds 36 trades at about PF 1.0).
`na_bundle` sits at the top of its neighbourhood, so expect regression toward the median neighbour: **train PF ≈ 1.66 and 2023_25h1 PF ≈ 1.38**.
`nb_t15` (1h −0.15) has the same PF with the strongest 2023 (+250 / PF 1.37) but trades 77 a year, under the bar.

## dead ends (this desk)

AM-core removal on noADX (costs +123). VPIN below 0.20 or k 0.06 on noADX (added trades PF 0.4–1.0). pm 1h back to 0. Score exit −0.5 or off (identical,
neutral to negative). pdl as a quality lever under a volume constraint. Core 5m/1h dials (≤ ±3 trades). Every lever that adds volume after 2022 adds it at PF ≈ 1.0:
**no cell in this family reaches 90 trades a year after 2022 at PF > 1.4.** The constraint binds on the market, not on the dials.

## nominations (best of 39 frontier specs / 44 runs; programme ledger ≈ 245 lines at writing)

1. **`research/lab/specs/frontier/na_bundle.json`**: noADX row-21 (sqrt band 10:00–11:00 k 0.05, thrust 1h ≤ −0.10, VPIN 0.20) + stress-s15-core window
   + ×1.25 tier at VPIN ≥ 0.26 (`--max-position-pct 0.45`).
   - Train: +2,610 / 320 / PF 1.75, dd −238, worst day −168, p 0.0007, ex-best-year (2022) PF 1.47.
   - Halves: 22h1 +837 · 22h2 +606 · 23h1 **−84** · 23h2 +221 · 24h1 +167 · 24h2 +482 · 25h1 +380.
   - 2023_25h1: +1,167 / 201 / PF 1.47. vs v18: P 0.006 (train), 0.005 (2023_25h1).
   - Why: best train PF that clears ≥ 90 trades a year, all years positive, on a measured plateau.
   - Risks: volume after 2022 is ~82 a year. NVDA is 70 % of P&L. The tier raises the size cap to 0.45 research (0.40 live). The stress window
     needs the session opened to 15:30 / 15:55.
   - **Holdout expectation (written before any reveal):** ~100–120 trades, PF 1.2–1.45 (central 1.3), P&L +250…+700 at research sizing, both 2025h2 and 2026 positive.
     Below PF 1.1 would mean the bundle is fitted to 2022 / 2024h2. My prior is coloured by HISTORY §C, which already shows 2026 numbers for the row-21 family.
2. **`research/lab/specs/frontier/nb_v15.json`**: the same bundle with VPIN 0.15.
   - Train: +2,735 / 397 / PF 1.61, 114 trades a year, dd −267, p 0.0017, ex-2022 PF 1.35, all years positive (23 +212 / PF 1.21).
   - 2023_25h1: +1,063 / 243 / PF 1.35. vs v18: P 0.011 / 0.021.
   - Why: the only cell at ≥ 90 trades a year *after* 2022 (87 / 103 / 106 annualised) with PF > 1.3.
   - Caveat: vs na_bundle it is +126 (P 0.38) on train and −104 on 2023_25h1. Its extra trades earn their money in 2022.
   - **Holdout expectation:** ~130–150 trades, PF 1.15–1.35, P&L similar to or below #1.
   - Nominate it only if the coordinator weights trade count; otherwise #1 dominates it.
3. No third nomination. `nb_t15` (PF 1.75, 77 a year, 2023_25h1 PF 1.47) and `na_pdl10` (PF 2.11, 49 a year, 2023_25h1 PF 1.89) fail the volume bar.
   They are the quality alternatives if that bar is relaxed. `na_pdl10` is the most different book (daily correlation 0.54 with v18).

Proposal (no engine change needed). If #1 goes forward, the live patch is the generator output with `base_fraction` 0.30 and `session.max_position_pct`
0.40, as in `research/sizing/size_vpin26_v1.patch.json`. The watchdog's CRITICAL check keys on the promoted config's `force_exit_by`
(15:55 once the session is opened).
