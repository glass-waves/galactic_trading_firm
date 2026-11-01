# meta desk — re-scoring every existing sweep (2026-10-31)

No new searches. Only the five reproduction runs (§4) plus one port cell (`meta/v18_pdl10`) were added to the ledger.
Scripts and raw numbers live in my scratchpad and were not committed: `scoreall.py` (all tags × 4 windows → scores.json), `feedfp.py`
(fingerprints each tag's feed and cost model from its entry prices), `brk.py`/`yr.py` (per-trade breakdowns).
Nothing here reads after 2025-06-30.

## 0. Comparability: what can be ranked against what

There are 404 tag sets in `data/*_trades.csv` (373 have a `.args` file). The `.args` file does **not** record `BARS_DIR`
(`run_cached_sweep.sh` defaults to the SIP cache `data/bars`) or `COST_ARGS`. So I fingerprinted every tag from its own trades. For 40 random
train-window trades I recomputed entry = bar open × (1 ± 3 bps) ± $0.005 from each of the four caches (`bars_iex`, `bars_iex_raw`,
`bars`, `bars_raw`).

| class | tags | treatment |
|---|---|---|
| IEX, point-in-time/raw, house costs, 0.36/0.36, `--cross-index SPY`, after the VWAP fix (09-26 23:16) | 217 → **194 distinct records** (identical trade sets merged, e.g. `cand_29 = qv_v217_b04 = tr_b_1h15(raw)`; `iex_v18 = st2_am_only = rq9_iex = iex_v18_pitcheck`) | **ranked** |
| SIP cache (`e_*` entry screen, `h_*` holds, `sip_*`, `v12/v13*`, `sig_orb*`, `scr16`, `rq9_sip`) | 64 | not ranked: different feed; VPIN 0.217 means something else on SIP |
| IEX but **before the VWAP fix** (`vp_*`, `tier*`, `rl_*`, `rs_*`, `e_iex_*` grid, `iex_v18_prevwap`) | 60 | not ranked: multi-day VWAP restatement |
| IEX split-**adjusted** replays (`*_adj`, `cand_35–38`, `cand_42`, `iex_v18_adj`, `rq8_k01_adj`) | 13 | not ranked: dollar MACD is wrong on adjusted bars (the "+1318" restatement) |
| entry price matches no cache under house costs (`earn*` ORB fills, `e_v17_cost5/10`, `*_c2` cost-x2, `v15*` pre-cost-fix) | 22 | not ranked |
| no train-window trades (2026-only or empty) | 22 | — |

Caveat inside the ranked set: tags swept before 10-13 (`tf_*`, `tr_*`, `qv_*`, `st*`, `vw_*`, `*_raw`) ran on the raw IEX cache and not the
point-in-time loader. That moves **2022 only** by about ±100 (`iex_v18` +1089 vs raw +991) and leaves 2023 onward the same to within a trade or two.
Compare families on `2023_25h1`, where this does not matter. Single-ticker tags (`cand_2…43`, `uq_*`) run one name alone and are ranked
as ticker evidence, not as books.

## 1. Leaderboards

Ranking rule: PF on the window, among records with ≥ 150 trades on `train` (118 of 194). p = sign-flip p on daily P&L; ex-best = PF
without the best calendar year. The baseline `iex_v18` row is appended to each table.

#### window `train` (ranked by window PF; eligible = ≥ 150 train trades)

| tag | what it is | P&L | n | PF | dd | p | ex-best PF | per year |
|---|---|---|---|---|---|---|---|---|
| rq6_r21_v0.20 | RQ-6: row 21 (sqrt band + thrust 1h≤−0.15), VPIN 0.20 | +1530 | 250 | 1.58 | -451 | 0.0057 | 1.313 | 22:+936/77/2.28 23:+136/67/1.21 24:+166/74/1.18 25:+292/32/1.88 |
| rq6_r21_v0.18 | RQ-6: row 21, VPIN 0.18 | +1681 | 283 | 1.55 | -414 | 0.008 | 1.284 | 22:+1070/88/2.19 23:+136/74/1.19 24:+185/83/1.19 25:+290/38/1.60 |
| cand_45 | #45 spy-sqrt-band-1h15 (row 21, VPIN 0.217) | +1317 | 233 | 1.54 | -344 | 0.015 | 1.285 | 22:+806/71/2.29 23:+125/63/1.21 24:+81/68/1.09 25:+306/31/1.97 |
| rq6_r21_v0.15 | RQ-6: row 21, VPIN 0.15 | +1840 | 313 | 1.54 | -389 | 0.0073 | 1.25 | 22:+1242/99/2.24 23:+147/84/1.19 24:+215/89/1.20 25:+236/41/1.43 |
| cand_48_adj | #48 QQQ noise-area long, pm, high-vol days (row 23, lb 22) — standalone | +982 | 229 | 1.51 | -182 | 0.0287 | 1.423 | 22:+600/118/1.59 23:+168/57/1.53 24:-22/24/0.89 25:+236/30/1.60 |
| tf_sq05_1h15 | timefilter: sqrt k0.05 to 11:30 + thrust 1h≤−0.15 | +1188 | 248 | 1.47 | -441 | 0.0253 | 1.338 | 22:+593/76/1.75 23:+83/66/1.13 24:+180/74/1.22 25:+333/32/2.09 |
| cand_49 | #49 spy-sqrt-band-v020 (row 20, VPIN 0.20) | +1869 | 370 | 1.47 | -614 | 0.013 | 1.193 | 22:+1275/106/2.34 23:+60/99/1.05 24:+475/113/1.39 25:+58/52/1.08 |
| cand_29 | #29 thrust-1h15 (v18 + thrust 1h≤−0.15) | +1013 | 207 | 1.46 | -348 | 0.039 | 1.162 | 22:+741/59/2.37 23:+55/58/1.09 24:-56/62/0.93 25:+273/28/2.00 |
| tf_sq05_1100 | timefilter: sqrt k0.05 band, entries to 11:00 | +1591 | 336 | 1.45 | -611 | 0.0127 | 1.2 | 22:+1054/93/2.21 23:-1/92/1.00 24:+270/107/1.23 25:+268/44/1.61 |
| qv_v150_b04 | qualvol: 1h15 + VPIN 0.15, band ±0.2 | +1478 | 293 | 1.44 | -383 | 0.0227 | 1.202 | 22:+1012/89/1.97 23:+131/79/1.17 24:+167/85/1.17 25:+169/40/1.31 |
| tr_b_1h20 | trigger: thrust 1h≤−0.20 | +746 | 156 | 1.43 | -258 | 0.068 | 1.279 | 22:+402/48/1.78 23:+104/42/1.22 24:-4/46/0.99 25:+244/20/2.24 |
| tf_sq04_1100 | timefilter: sqrt k0.04 to 11:00 | +1382 | 304 | 1.42 | -412 | 0.0263 | 1.108 | 22:+1100/79/2.65 23:+3/86/1.00 24:+115/98/1.10 25:+165/41/1.36 |
| rq6_r20_v0.18 | RQ-6: row 20 (sqrt band), VPIN 0.18 | +1930 | 407 | 1.42 | -587 | 0.0107 | 1.14 | 22:+1451/120/2.25 23:-101/108/0.93 24:+502/121/1.40 25:+78/58/1.10 |
| qv_v180_b04 | qualvol: 1h15 + VPIN 0.18, band ±0.2 | +1234 | 259 | 1.42 | -371 | 0.0343 | 1.176 | 22:+868/75/1.97 23:+92/70/1.13 24:+55/78/1.06 25:+219/36/1.46 |
| cand_44 | #44 spy-sqrt-band (flat open + sqrt 10:00+, to 11:00) | +1581 | 348 | 1.41 | -481 | 0.0203 | 1.113 | 22:+1242/98/2.47 23:+77/94/1.07 24:+120/108/1.09 25:+142/48/1.24 |
| iex_v18 | BASELINE v18 (row 12) | +1179 | 324 | 1.32 | -469 | 0.061 | 1.031 | 22:+1089/87/2.36 23:-3/91/1.00 24:-17/101/0.99 25:+110/45/1.20 |

#### window `2023_25h1` (ranked by window PF; eligible = ≥ 150 train trades)

| tag | what it is | P&L | n | PF | dd | p | ex-best PF | per year |
|---|---|---|---|---|---|---|---|---|
| cand_48_adj | #48 QQQ noise-area long, pm, high-vol days (row 23, lb 22) — standalone | +382 | 111 | 1.42 | -125 | 0.2193 | 1.287 | 23:+168/57/1.53 24:-22/24/0.89 25:+236/30/1.60 |
| tf_sq05_1h15 | timefilter: sqrt k0.05 to 11:30 + thrust 1h≤−0.15 | +596 | 172 | 1.34 | -289 | 0.1103 | 1.18 | 23:+83/66/1.13 24:+180/74/1.22 25:+333/32/2.09 |
| im_na_qqq_dm07 | index mom: QQQ noise-area, high-vol days (full day) | +465 | 205 | 1.32 | -129 | 0.1443 | 1.256 | 23:+38/115/1.05 24:+195/40/1.93 25:+233/50/1.42 |
| rq6_r21_v0.20 | RQ-6: row 21 (sqrt band + thrust 1h≤−0.15), VPIN 0.20 | +594 | 173 | 1.31 | -241 | 0.1213 | 1.193 | 23:+136/67/1.21 24:+166/74/1.18 25:+292/32/1.88 |
| cand_45 | #45 spy-sqrt-band-1h15 (row 21, VPIN 0.217) | +512 | 162 | 1.28 | -286 | 0.1393 | 1.139 | 23:+125/63/1.21 24:+81/68/1.09 25:+306/31/1.97 |
| rq6_r21_v0.18 | RQ-6: row 21, VPIN 0.18 | +611 | 195 | 1.28 | -274 | 0.1263 | 1.192 | 23:+136/74/1.19 24:+185/83/1.19 25:+290/38/1.60 |
| qv_v150_b06_thrust1h_m20 | qualvol: v150 b06, thrust 1h −0.20 | +536 | 185 | 1.28 | -283 | 0.123 | 1.219 | 23:+254/72/1.42 24:+144/76/1.17 25:+138/37/1.32 |
| tr_b_1h20 | trigger: thrust 1h≤−0.20 | +343 | 108 | 1.28 | -258 | 0.192 | 1.096 | 23:+104/42/1.22 24:-4/46/0.99 25:+244/20/2.24 |
| rq6_r21_v0.15 | RQ-6: row 21, VPIN 0.15 | +598 | 214 | 1.25 | -260 | 0.1487 | 1.196 | 23:+147/84/1.19 24:+215/89/1.20 25:+236/41/1.43 |
| qv_v180_b06 | qualvol: 1h15 + VPIN 0.18, band ±0.3 | +513 | 228 | 1.22 | -303 | 0.1717 | 1.195 | 23:+200/87/1.25 24:+125/93/1.12 25:+189/48/1.32 |
| qv_v180_b06_thrust1h_m20 | qualvol: v180 b06, thrust 1h −0.20 | +369 | 170 | 1.21 | -284 | 0.2063 | 1.135 | 23:+209/67/1.35 24:+71/68/1.09 25:+89/35/1.21 |
| qv_v150_b04 | qualvol: 1h15 + VPIN 0.15, band ±0.2 | +466 | 204 | 1.20 | -273 | 0.199 | 1.169 | 23:+131/79/1.17 24:+167/85/1.17 25:+169/40/1.31 |
| tf_sq05_1100 | timefilter: sqrt k0.05 band, entries to 11:00 | +537 | 243 | 1.20 | -280 | 0.176 | 1.175 | 23:-1/92/1.00 24:+270/107/1.23 25:+268/44/1.61 |
| tf_sq05_1h05 | timefilter: sqrt k0.05 + thrust 1h≤−0.05 | +551 | 247 | 1.20 | -228 | 0.1857 | 1.129 | 23:-9/92/0.99 24:+346/112/1.29 25:+214/43/1.45 |
| cand_49 | #49 spy-sqrt-band-v020 (row 20, VPIN 0.20) | +593 | 264 | 1.19 | -315 | 0.1683 | 1.064 | 23:+60/99/1.05 24:+475/113/1.39 25:+58/52/1.08 |
| iex_v18 | BASELINE v18 (row 12) | +90 | 237 | 1.03 | -343 | 0.4247 | 0.991 | 23:-3/91/1.00 24:-17/101/0.99 25:+110/45/1.20 |

#### window `2024h2_25h1` (ranked by window PF; eligible = ≥ 150 train trades)

| tag | what it is | P&L | n | PF | dd | p | ex-best PF | per year |
|---|---|---|---|---|---|---|---|---|
| cand_36_pit | #36 TSLA alone under v18 | +605 | 45 | 1.99 | -175 | 0.0623 | 1.003 | 24:+1/20/1.00 25:+604/25/3.11 |
| tf_sq05_1h15 | timefilter: sqrt k0.05 to 11:30 + thrust 1h≤−0.15 | +435 | 70 | 1.60 | -289 | 0.1163 | 1.242 | 24:+102/38/1.24 25:+333/32/2.09 |
| rq6_r21_v0.20 | RQ-6: row 21 (sqrt band + thrust 1h≤−0.15), VPIN 0.20 | +431 | 69 | 1.58 | -241 | 0.1167 | 1.337 | 24:+139/37/1.34 25:+292/32/1.88 |
| cand_45 | #45 spy-sqrt-band-1h15 (row 21, VPIN 0.217) | +392 | 65 | 1.54 | -286 | 0.1293 | 1.212 | 24:+86/34/1.21 25:+306/31/1.97 |
| qv_v180_b06_core1h_m20 | qualvol: v180 b06, core 1h −0.20 | +932 | 174 | 1.53 | -264 | 0.0373 | 1.474 | 24:+416/93/1.47 25:+516/81/1.59 |
| tf_sq05_1h05 | timefilter: sqrt k0.05 + thrust 1h≤−0.05 | +533 | 99 | 1.52 | -220 | 0.116 | 1.447 | 24:+319/56/1.58 25:+214/43/1.45 |
| tf_sq05 | timefilter: sqrt k0.05 to 11:30 | +535 | 106 | 1.50 | -220 | 0.0983 | 1.558 | 24:+274/59/1.45 25:+261/47/1.56 |
| tf_sq05_1100 | timefilter: sqrt k0.05 band, entries to 11:00 | +511 | 98 | 1.50 | -215 | 0.1147 | 1.411 | 24:+242/54/1.41 25:+268/44/1.61 |
| im_na_qqq_dm07 | index mom: QQQ noise-area, high-vol days (full day) | +371 | 85 | 1.49 | -126 | 0.201 | 1.667 | 24:+139/35/1.67 25:+233/50/1.42 |
| rq6_r21_v0.18 | RQ-6: row 21, VPIN 0.18 | +426 | 80 | 1.46 | -274 | 0.136 | 1.305 | 24:+137/42/1.30 25:+290/38/1.60 |
| tf_sq05_1h10 | timefilter: sqrt k0.05 + thrust 1h≤−0.10 | +432 | 85 | 1.46 | -205 | 0.1377 | 1.455 | 24:+232/47/1.46 25:+200/38/1.46 |
| qv_v150_b06_core1h_m30 | qualvol: v150 b06, core 1h −0.30 | +596 | 132 | 1.42 | -260 | 0.1083 | 1.258 | 24:+417/70/1.56 25:+179/62/1.26 |
| iex_v18 | BASELINE v18 (row 12) | +59 | 97 | 1.05 | -343 | 0.449 | 0.927 | 24:-51/52/0.93 25:+110/45/1.20 |

#### window `2025h1` (ranked by window PF; eligible = ≥ 150 train trades)

| tag | what it is | P&L | n | PF | dd | p | ex-best PF | per year |
|---|---|---|---|---|---|---|---|---|
| cand_36_pit | #36 TSLA alone under v18 | +604 | 25 | 3.11 | -94 | 0.04 | - | 25:+604/25/3.11 |
| tr_b_1h20 | trigger: thrust 1h≤−0.20 | +244 | 20 | 2.24 | -143 | 0.119 | - | 25:+244/20/2.24 |
| tf_sq05_1h15 | timefilter: sqrt k0.05 to 11:30 + thrust 1h≤−0.15 | +333 | 32 | 2.09 | -168 | 0.1027 | - | 25:+333/32/2.09 |
| cand_29 | #29 thrust-1h15 (v18 + thrust 1h≤−0.15) | +273 | 28 | 2.00 | -194 | 0.13 | - | 25:+273/28/2.00 |
| tr_b_1h15 | = cand_29 on the raw-cache replay | +273 | 28 | 2.00 | -194 | 0.13 | - | 25:+273/28/2.00 |
| cand_45 | #45 spy-sqrt-band-1h15 (row 21, VPIN 0.217) | +306 | 31 | 1.97 | -194 | 0.1157 | - | 25:+306/31/1.97 |
| rq6_r21_v0.20 | RQ-6: row 21 (sqrt band + thrust 1h≤−0.15), VPIN 0.20 | +292 | 32 | 1.88 | -194 | 0.1327 | - | 25:+292/32/1.88 |
| im_na_spy_nostop | index mom: SPY noise-area, no stop | +408 | 64 | 1.82 | -170 | 0.1433 | - | 25:+408/64/1.82 |
| tf_sq05_1100 | timefilter: sqrt k0.05 band, entries to 11:00 | +268 | 44 | 1.61 | -125 | 0.1777 | - | 25:+268/44/1.61 |
| rq6_r21_v0.18 | RQ-6: row 21, VPIN 0.18 | +290 | 38 | 1.60 | -228 | 0.1613 | - | 25:+290/38/1.60 |
| cand_48_adj | #48 QQQ noise-area long, pm, high-vol days (row 23, lb 22) — standalone | +236 | 30 | 1.60 | -125 | 0.371 | - | 25:+236/30/1.60 |
| qv_v180_b06_core1h_m20 | qualvol: v180 b06, core 1h −0.20 | +516 | 81 | 1.59 | -264 | 0.0743 | - | 25:+516/81/1.59 |
| iex_v18 | BASELINE v18 (row 12) | +110 | 45 | 1.20 | -179 | 0.3577 | - | 25:+110/45/1.20 |

#### high PF, low count (train n < 150, PF ≥ 1.25: not ranked)

| tag | what | train P&L/n/PF | p | 2023_25h1 P&L/n/PF | per year (train) |
|---|---|---|---|---|---|
| cand_15 | XOM alone, v18 | +228/38/1.88 | 0.11 | +64/20/1.63 | 22:+164 23:+64 24:−10 25h1:+11 |
| cand_8 | JPM alone | +221/31/1.64 | 0.17 | +321/18/4.10 | 22:−100 23:+4 24:+45 25h1:+272 |
| cand_30 | QQQ alone | +139/43/1.53 | 0.14 | +45/30/1.25 | 22:+95 … 25h1:+44 |
| uq_AMD | AMD alone, thrust-1h15 | +243/68/1.35 | 0.23 | −34/53/0.95 | 24:−244 |
| tr_b_lag1m | thrust with 1m most bearish | +566/145/1.32 | 0.15 | +110/94/1.10 | 25h1:−92 |
| cand_21 | CAT alone | +133/45/1.30 | 0.27 | +13/35/1.04 | 24:−129 |
| uq_META | META alone, thrust-1h15 | +236/73/1.28 | 0.23 | +60/50/1.12 | 23:−168 |
| cand_42_pit | SHOP alone | +368/78/1.26 | 0.24 | −346/58/0.72 | 22:+714 23:−445 |

None of these has p < 0.10. LLY (`cand_12`, 85 train trades, PF 1.23) has the best recent single-name record: 2023_25h1 +335/66/PF 1.49, with all three periods positive.

#### not comparable, but the standout of the whole scan

`e_spy02_pdl10_vpin_cond` is the 2026-09-12 entry-screen "triple": v16 + SPY ±0.2 % + VPIN ≥ 0.217 + **exclude entries > 1 % below the
prior-day low** (`pdl_5m.dist_low_pct ≥ −1.0`). It is on SIP, before the VWAP fix, on the v16 exits. Train: **+1903 / 212 / PF 2.10**; 2023_25h1:
**+1113 / 141 / PF 2.03**; 23:+337 24:+236 25h1:+541. Its twin without the pdl clause (`e_spy02_vpin_cond`, the sweep v17 was promoted from):
train PF 1.58, 2023_25h1 PF 1.28. Old plan §11.3 rejected the triple only for frequency ("~1 trade/week, three filters stacked"). It was
never re-tested after the move to IEX and v18, and HISTORY.md does not list it. What the clause removes (trade diff of the two SIP
tags): 2022 **+515/43/PF 2.52** (the crash-day shorts already far below yesterday's low), 2023 **−286/35/PF 0.37**, 2024 +3/42/PF 1.01,
2025h1 −31/29/PF 0.92. It is a recent-regime lever by construction. Ported to v18 on IEX as `meta/v18_pdl10` (§4).

Other notable non-comparable records: `sip_sqrt_band` (SIP, train +2243/405/PF 1.56, 2023_25h1 PF 1.34) and `sip_v18_vwapfix` (SIP v18, train
PF 1.51, 2023_25h1 PF 1.25). On SIP the v18 book holds up after 2022 better than on IEX (2023_25h1 PF 1.03).
`e_pdl10_cond` (pdl alone on v16, SIP) has 2023_25h1 +1201/309/PF 1.40.

## 2. Structural read: which levers carry the post-2022 edge

Each row is a one-dial family. Columns are 2022 and `2023_25h1` (2023-01 → 2025-06), as P&L / n / PF.

| lever | cells (dial value → 2022 PF ; 2023_25h1 P&L/n/PF) | read |
|---|---|---|
| **thrust 1h ceiling** (v18 base, `tr_b_*`) | none 1.87 ; +64/261/1.02 · ≤−0.05 2.08 ; +120/224/1.04 · ≤−0.10 1.84 ; +24/186/1.01 · **≤−0.15 2.00 ; +272/148/1.16** · **≤−0.20 1.78 ; +343/108/1.28** · v18 (≤0) 2.10 ; +86/238/1.03 | the main recent-regime lever. A tighter 1h ceiling raises post-2022 PF and lowers 2022 PF. 1h10 is a dip in the curve, so do not read the curve as smooth |
| thrust 1h on the sqrt band (`tf_sq05_*`) | none 1.98 ; +524/261/1.18 · 1h05 2.00 ; 1.20 · 1h10 1.69 ; 1.18 · **1h15 1.75 ; +596/172/1.34** | same direction on a second base |
| loosening 1h back from −0.15 (`qv_*_thrust1h_m10`) | v150: 1h15 1.18 → **−0.10: 0.97** (−118/296) · v180: 1.22 → 1.07 | the loosening direction gives back all of the recent edge |
| core 1h loosened (−0.2/−0.3) | 2023_25h1 PF 1.01–1.07 on 284–478 trades | dead (HISTORY B1 already) |
| **SPY band shape** (`tf_*`) | flat ±0.2 → 11:30: 2.10 ; 1.03 · sqrt k0.04 2.35 ; 1.09 · **k0.05 1.98 ; +524/261/1.18** · k0.06 2.03 ; 1.08 · flat-open + sqrt 10:00+ (`c_sq05`) 1.99 ; 1.10 · rolling 15m 1.85 ; **0.88** · flat + rolling 1.82 ; 1.00 | the sqrt band helps after 2022 (+0.15 PF). k0.05 is a sharp peak (k0.04 and k0.06 both ≈ 1.08), so treat the size of the gain as partly fitted. Rolling bands only work in 2022 |
| what the sqrt band adds | 10:00–11:00 entries in the r21 cell: 22 +306/29 · 23 +159/22 · 24 +144/23 · 25h1 +184/12 (PF 1.45–3.8 every period) | the 10:00–11:00 extension is the one block of entries that is positive in every period |
| **VPIN floor** on row 21 | 0.15: 2.24 ; +598/214/1.25 · 0.18: 2.19 ; 1.28 · 0.20: 2.28 ; 1.31 · 0.217: 2.29 ; +512/162/1.28 | flat after 2022 between 0.15 and 0.217 (PF 1.25–1.31). Recent edge does not depend on a high floor; a lower floor buys trades almost free (+52 trades for −0.06 PF) |
| VPIN floor on row 20 / v18 | r20: 0.15 1.13 · 0.18 1.14 · 0.20 1.19 · 0.217 1.11 ; v18: 0.217 1.03 → 0.26 1.11 | small effect without the 1h dial |
| entry clock | both windows 09:45–11:00: 2023_25h1 **0.89** · to 13:00 0.96 · to 15:30 0.95 · sqrt to 13:00 1.05 / to 15:30 1.06 | the open is still the edge. Entries after 11:30 dilute it in every period |
| trigger strength (composite ≤ −0.40/−0.45, 5m ≤ −0.6) | 2022 PF 2.28–2.56 ; 2023_25h1 0.97–0.99 | a stronger trigger is a 2022 lever only |
| stress add-ons (s10/s15/s20/n15/c10n15 core, VWAP short) | 2022 +1017…+1595 ; 2023_25h1 −438…+133 (PF 0.89–1.05) | 2022 only, as expected |
| **ticker** (r21_v0.20) | AAPL 22 +15/8 · 23 −3/6 · 24 −147/8 · 25h1 −68/6 (2023_25h1 −218/20/PF 0.25) · AMZN 2023 −147/17 then +155, +127 · MSFT +36/+99/−14 · NVDA +249/+58/+247 | AAPL adds nothing in any period of any 1h15 cell. Ex-AAPL r21_v0.20 is train +1733/222/PF 1.76 and 2023_25h1 +812/153/PF 1.51, but that is a post-hoc, in-sample cut. On v18 AAPL is +49 in 2023, so the AAPL effect is specific to the 1h15 cells. AMZN-2023 is v18's single biggest hole (−385/23/PF 0.16); the 1h dial halves it |
| entry hour (09:45–10:00) | v18: 23 +27 · 24 +71 · 25h1 −162 ; 1h15 cells: 23 −93/−102 · 24 −162/−196 · 25h1 +17 | not stable across cells or years. No clock cut is justified |
| **exits** | ScoreExit trades lose in every year of every cell: v18 train −781/33, 2023_25h1 **−661/23** (−$29/trade, more than v18's whole post-2022 P&L of +90); r21_v0.20 −349/16 post-2022. BreakevenStop −234/50 (by design). MaxHoldTimeout carries the book (+803/148) | the score exit fires after a reversal is already in the price. The counterfactual is unknown without a run, so this goes to the exits desk (§6) |
| strong-core window | v18 2023_25h1 −109/11/PF 0.49 ; r21 −100/8 | negative after 2022 in every cell, but the count is small |
| sizing | `cand_28` (×1.25 at VPIN ≥ 0.26) exists only as adjusted/raw variants (2023_25h1 PF ≈ v18) | leverage, not edge |
| standalone QQQ noise-area (`cand_48`) | 2022 1.59 ; **+382/111/1.42**; daily correlation with v18 −0.04 | the only non-v18 mechanism with post-2022 PF > 1.3 |

**Summary of the read.** The post-2022 edge sits in (a) the thrust window's 1h ceiling at −0.15/−0.20, (b) the sqrt SPY band and its
10:00–11:00 entries, and (c) NVDA/MSFT/(AMZN after 2023). The 2022 edge sat in a loose 1h, strong triggers, stress shorts and crash-day shorts far
below the prior-day low. Every lever that loosens toward 2022 costs PF after 2022.

## 3. The 2022 dependence problem: the top 20 train records on `2023_25h1`

| # | tag | train P&L/n/PF | ex-best PF | 2022 | 2023_25h1 P&L/n/PF | p (23–25h1) | 23 | 24 | 25h1 | post-2022 edge* |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | rq6_r21_v0.20 | +1530/250/1.58 | 1.31 | +936 | +594/173/**1.31** | 0.12 | +136 | +166 | +292 | YES |
| 2 | rq6_r21_v0.18 | +1681/283/1.55 | 1.28 | +1070 | +611/195/**1.28** | 0.13 | +136 | +185 | +290 | YES |
| 3 | cand_45 | +1317/233/1.54 | 1.29 | +806 | +512/162/**1.28** | 0.14 | +125 | +81 | +306 | YES |
| 4 | rq6_r21_v0.15 | +1840/313/1.54 | 1.25 | +1242 | +598/214/**1.25** | 0.15 | +147 | +215 | +236 | YES |
| 5 | cand_48 (QQQ) | +982/229/1.51 | 1.42 | +600 | +382/111/**1.42** | 0.22 | +168 | −22 | +236 | weak (n) |
| 6 | tf_sq05_1h15 | +1188/248/1.47 | 1.34 | +593 | +596/172/**1.34** | 0.11 | +83 | +180 | +333 | YES |
| 7 | cand_49 | +1869/370/1.47 | 1.19 | +1275 | +593/264/1.19 | 0.17 | +60 | +475 | +58 | weak |
| 8 | cand_29 | +1013/207/1.46 | 1.16 | +741 | +272/148/1.16 | 0.26 | +55 | −56 | +273 | weak |
| 9 | tf_sq05_1100 | +1591/336/1.45 | 1.20 | +1054 | +537/243/1.20 | 0.18 | −1 | +270 | +268 | weak |
| 10 | qv_v150_b04 | +1478/293/1.44 | 1.20 | +1012 | +466/204/1.20 | 0.20 | +131 | +167 | +169 | weak |
| 11 | tr_b_1h20 | +746/156/1.43 | 1.28 | +402 | +343/108/1.28 | 0.19 | +104 | −4 | +244 | weak (n) |
| 12 | tf_sq04_1100 | +1382/304/1.42 | 1.11 | +1100 | +282/225/1.11 | 0.31 | +3 | +115 | +165 | no |
| 13 | rq6_r20_v0.18 | +1930/407/1.42 | 1.14 | +1451 | +480/287/1.14 | 0.25 | −101 | +502 | +78 | no |
| 14 | qv_v180_b04 | +1234/259/1.42 | 1.18 | +868 | +366/184/1.18 | 0.22 | +92 | +55 | +219 | weak |
| 15 | cand_44 | +1581/348/1.41 | 1.11 | +1242 | +339/250/1.11 | 0.30 | +77 | +120 | +142 | no |
| 16 | tf_sq05_1h05 | +1507/347/1.41 | 1.20 | +956 | +551/247/1.20 | 0.19 | −9 | +346 | +214 | weak |
| 17 | rq6_r20_v0.15 | +2065/457/1.41 | 1.13 | +1601 | +465/313/1.13 | 0.24 | +30 | +361 | +73 | no |
| 18 | qv_v150_b06 | +1574/358/1.40 | 1.18 | +1096 | +478/250/1.18 | 0.20 | +141 | +138 | +200 | weak |
| 19 | tr_b_1h15 | (= cand_29, raw cache) | | | | | | | | |
| 20 | tf_sq05 | +1510/366/1.39 | 1.18 | +986 | +524/261/1.18 | 0.20 | −88 | +350 | +261 | weak |
| ref | iex_v18 | +1179/324/1.32 | 1.03 | +1089 | +90/237/1.03 | — | −3 | −17 | +110 | no |

\* YES = 2023_25h1 PF ≥ 1.25 and 2023, 2024 and 2025h1 each positive.

**Reading.** Only one mechanism has real post-2022 edge: **sqrt SPY band + thrust 1h ≤ −0.15**. That is row 21 and its VPIN variants, plus
`tf_sq05_1h15`, which is the same idea with entries to 11:30. Five of the six "YES" rows are this one mechanism, so they are one finding, not five.
Even for it, post-2022 alone is not significant (sign-flip p 0.11–0.15 on ~170 trades). Paired against v18 on 2023_25h1:
r21_v0.20 +504, P(diff ≤ 0) 0.10; tf_sq05_1h15 +506, P 0.11; cand_45 +422, P 0.15 (daily correlation with v18 0.75).
On all of train, r21_v0.20 vs v18 is only +351, P 0.24, because it gives back 2022.
Everything that wins train by extra trades or a looser filter (`cand_49`, `rq6_r20_*`, `cand_44`, `tf_sq04/06`) is a 2022 record with
post-2022 PF 1.11–1.19.

**Two-book note.** r21_v0.20 + cand_48 (QQQ afternoon, correlation −0.04) as two books on train: **+2512 / 479 / PF 1.55, dd −277,
p 0.0013, ex-best 1.35**, every year positive (22 +1536, 23 +304, 24 +144, 25h1 +528); 2023_25h1 +976/284/PF 1.35. The live blocker is
cand_48's 22-day warm-up (the trader warms 8).

## 4. Lab reproductions (`train`, one run each, `LAB_AGENT=meta`)

Scaffold check first: `meta/v18_pdl_scaffold` (v18 + weight-0 `pdl_5m`) vs `base/v18` on 2025h1 gave P&L diff 0, identical counts, correlation 1.00.

| spec | reproduces | recorded train | lab train | verdict |
|---|---|---|---|---|
| `research/lab/specs/meta/cand45_r21.json` (row 21) | cand_45 spy-sqrt-band-1h15 | +1317 / 233 / 1.54 | **+1317 / 233 / 1.54**, every year identical | exact |
| `research/lab/specs/meta/r21_v020.json` (row 21 + `--set-action-param` VPIN 0.20 ×4) | rq6_r21_v0.20 | +1530 / 250 / 1.58 | **+1530 / 250 / 1.58** | exact |
| `research/lab/specs/meta/r21_v018.json` | rq6_r21_v0.18 | +1681 / 283 / 1.55 | **+1681 / 283 / 1.55** | exact |
| `research/lab/specs/meta/qqq_noise_pm_dm07.json` (row 23, lookback 22) | cand_48 qqq-noise-pm-vol | +982 / 229 / 1.51 | **+982 / 229 / 1.51**, every year identical | exact |
| `research/lab/specs/meta/v18_pdl10.json` (v18 + `pdl_5m.dist_low_pct ≥ −1.0` on both windows; patch `research/lab/patches/meta/v18_pdl10.json`) | **port** of the SIP triple, not a reproduction | (SIP/v16: +1903 / 212 / 2.10) | **+1098 / 200 / 1.59**, dd −277, p 0.019, ex-2022 PF 1.29 | new cell, see below |

The binary and cache have not drifted: the lab reproduces the pipeline sweeps trade for trade. `tf_sq05_1h15` was not respecified. Its patch
was pruned (regenerate it with `research/timefilter/make_patches.py sq05_1h15`), and it is the same mechanism as row 21 (sqrt band to 11:30 instead of
flat-open + sqrt 10:00+ to 11:00).

**`meta/v18_pdl10` in detail.** Per year: 22 +678/51/2.63 · 23 +205/61/1.33 · 24 −26/61/0.96 · 25h1 +241/27/2.10. On 2023_25h1 it makes **+420/149/PF 1.29**
against v18's +90/237/1.03 (paired +331, P(diff ≤ 0) 0.19). On all of train it is −80 vs v18 but PF +0.27 (it removes 124 trades, many of them 2022
crash shorts). By ticker: NVDA +884/98, MSFT +201, AMZN +12, AAPL +1. It shares only **77 of its 200 trades** with r21_v0.20 (daily
correlation on 2023_25h1 0.58), so the pdl clause and the sqrt-band/1h dial select different trades. The combination is untested (§6).

## 5. Nominations (best of 194 scored records + 1 port; all are re-expressions of existing work)

**Holdout caveat that applies to every meta nomination.** Each underlying tag was a five-year sweep (2022 → 2026-09) whose 2025h2–2026
numbers its authors saw when they chose it (HISTORY §C quotes them). The holdout is therefore *not* clean for these cells the way it is for
fresh desk cells. The coordinator should discount any holdout pass from this list.

1. **`research/lab/specs/meta/r21_v018.json`**: sqrt SPY band + thrust 1h ≤ −0.15 + VPIN 0.18. Train +1681/283/PF 1.55, p 0.008, ex-best
   1.28. 2023_25h1 +611/195/PF 1.28, with every year positive (+136/+185/+290). Why VPIN 0.18 over 0.20: it sits in the middle of a flat VPIN plateau
   (0.15–0.217 all PF 1.25–1.31 post-2022) and adds 33 trades. Trade rate is about 6.5/month, so the target window gives ≈ 140 trades,
   slightly under the 150 bar. **Expectation on the holdout: PF 1.2–1.35, P&L +150…+400, ≈ 100 trades. PF ≥ 1.3 about even odds.**
2. **`research/lab/specs/meta/qqq_noise_pm_dm07.json`**: standalone QQQ afternoon long book (cand_48). Train +982/229/PF 1.51, ex-best 1.42,
   2023_25h1 PF 1.42, daily correlation with v18 −0.04. As a second book next to #1: train **+2512/479/PF 1.55, dd −277, p 0.001**. 2024 was −22/24.
   It needs a 22-day warm-up (a live blocker) and fails at 2× costs (HISTORY B13). **Expectation: PF 1.1–1.4 on ~60 trades. Its value is
   diversification, not a standalone pass.**
3. **`research/lab/specs/meta/v18_pdl10.json`**: v18 + the prior-day-low exclusion. Train PF 1.59 on 200 trades, 2023_25h1 PF 1.29. It is the
   one lever in the archive that was dropped for frequency, not for failing. It runs at ~4.8 trades/month, so on its own it will miss the 150-trade bar.
   **Expectation: PF 1.2–1.4 on ~75 trades. Nominated mainly as the clause to add to #1 (see frontier, §6).**

## 6. Suggestions to the other desks

- **frontier**: run `r21_v018` + the pdl clause (`pdl_5m.dist_low_pct ≥ −1.0` on all four row-21 windows). The two levers share only 77/200 trades and each lifts post-2022 PF from 1.03 to about 1.3, so the combination is the highest-expected-value untested cell in the archive.
- **frontier**: test ScoreExit off, or its threshold moved, on the row-21 windows. ScoreExit trades are −661/23 (−$29/trade) in v18's 2023_25h1, more than v18's entire post-2022 P&L (+90), and they lose in every year of every cell.
- **indicators**: the thrust 1h ceiling is the only dial whose post-2022 PF rises monotonically from ≤ 0 → −0.15 → −0.20 (1.03 → 1.16 → 1.28), with the 1h10 dip as the exception. A directional 1h (RQ-13: signed ADX / standard supertrend) should be judged on whether it beats the −0.20 cell's 2023_25h1 PF 1.28, not on train.
- **universe**: AAPL is −218/20/PF 0.25 on 2023_25h1 in r21_v0.20 and adds ≤ +15 in every period of every 1h15 cell (ex-AAPL r21_v0.20 train PF 1.76). Test the 3-name book and LLY (alone 2023_25h1 +335/66/PF 1.49, all three periods positive) as a replacement, and state that the AAPL cut is in-sample.
- **tom-event**: the only non-v18 mechanism with post-2022 PF > 1.3 in the archive is QQQ-specific and afternoon/high-vol (cand_48 2023_25h1 PF 1.42, im_na_qqq_dm07 1.32). SPY versions of the same rule are 0.84–0.91, so look for index event cells on QQQ before SPY.
- **long-2026**: every long-side tag scored is PF ≤ 1.04 on 2023_25h1 (cand_1 bear-bounce 0.96/429, cand_5 RS-long 1.00/901, rs_solo_x0.5 1.04/1097, rl_sma20 1.04, e_long_filters 0.90; the rl_/rs_ tags are also pre-VWAP-fix). The one long that works after 2022 is cand_48 (QQQ, pm, high-vol days), so a long desk should start from that gate (day-move filter `dm07`) and not from morning longs.
- **signals-new**: stronger triggers (composite ≤ −0.40/−0.45, 5m ≤ −0.6) earn 2022 PF 2.3–2.6 and post-2022 PF 0.97–0.99, and pullback entries are post-2022 0.86. Any new trigger should be shown on `2023_25h1` first, because the archive says 2022 rewards almost anything short.
- **all desks**: compare on `2023_25h1`, not `train`. The baseline's train PF 1.32 is 92 % 2022 dollars (+1089 of +1179), and the top-20 train list reorders completely post-2022: only 6 of 20 clear PF 1.25, all one mechanism.
