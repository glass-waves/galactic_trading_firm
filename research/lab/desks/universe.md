# universe desk — the short-morning engine on other names (lab, 2026-11-01)

desk: universe · `LAB_AGENT=universe` · **58 ledger lines** (37 single-name screens, 15 single-name train extensions,
6 group/engine cells; one of them, `nt10_u37`, screens 37 names in one process, so name-level tests on the new engine add 37
more looks the ledger does not show). No holdout dates, no engine edits, no DB/pipeline/trader changes, no commits.

## bottom line

1. **The short-morning edge does not belong to any name, and it does not belong to the universe either: it belongs to the
   2024h2–2025h1 tape.** Pooled over the 37 non-baseline names, v18 makes +2352 / 594 / PF 1.31 on 2024h2_25h1, but
   −1520 / 1461 / PF 0.92 on 2023_25h1 (23h1 0.72, 23h2 0.81, 24h1 0.57, 24h2 1.17, 25h1 1.45). Nearly every name "works"
   on the screen window. That is a regime, not a stock-selection signal.
2. **Name-level results do not persist.** Spearman correlation of per-name P&L between adjacent periods is −0.04 to −0.32
   in every split I tried, on all three engines. Names picked because they were positive in one period did *worse* than the
   rest in the next period in 9 of the 11 splits (table C1). Picking names in-sample here is fitting noise.
3. **No basket meets the nomination bar** (2023_25h1 PF ≥ 1.3, ≥ 100 trades, no name > 40 % of P&L). The closest is
   SELQ (LLY MSFT NFLX NVDA PLTR, v18): 2023_25h1 +1273 / 356 / PF 1.29, NVDA 29 %. But those five names were picked on
   that same window, and on 2022, a year the selection never saw, the names picked on 2023_25h1 make PF 1.02 against
   1.56 for the names left out.
4. **Engine changes that help the four do not carry over to other names.** thrust-1h15 lifts the four (2023_25h1 PF
   1.03 → 1.28) but drags the 37 others down (0.92 → 0.78). The indicators desk's best engine (`r21_v020_noadx_t10`) makes
   the four PF 1.41 on 2023_25h1, but on the 37 others it is PF 0.79 there and 0.87 over train (−4214 / 2080).
5. **ETFs are dead**: ETF6 (SMH XLK XLF XLE IWM QQQ) is −229 / 169 / PF 0.85 on train (capped). Only 2024h2 is positive.
6. **NVDA alone** has the best and steadiest single-name record (train +984 / 147 / PF 1.52, positive in every year:
   22 +626, 23 +164, 24 +133, 25h1 +62). It carries 84 % of the four-name book's train P&L, and more than 100 % of it
   on 2023_25h1. On SPY down-tail days (open→close ≤ −1 %, 101 days) NVDA makes +464 on 13 trades and AAPL+AMZN+MSFT
   make +379 on 19. On flat days (|SPY| < 0.5 %, 441 days) NVDA makes +165 / 86 / PF 1.15 and the other three
   −165 / 89 / PF 0.84. So "NVDA leads tail days" holds in the trades, but NVDA's real advantage over the other three
   is on ordinary days. Even so, NVDA alone is too thin to stand on its own (~11 trades in 2025h1).
7. Requested by the coordinator (meta / indicators desks): the AAPL drop and the LLY swap. Dropping AAPL improves the book
   in-sample on every engine. AAPL is negative on 2023_25h1 on all four engines: v18 −51/27, 1h15 −114/15, r21 −188/19,
   nt10 −191/25. LLY as a replacement adds trades but lowers PF on r21 and nt10. Both are best-of-few, post-hoc picks of
   the kind finding 2 says do not persist. I give one low-confidence nomination (`nt10_b3`) with a written expectation,
   because the coordinator asked for this exact book.

## method

- Engine: config row 12 unchanged (`u_<name>.json`, `"tickers": "<NAME>"`), lab defaults (IEX cache, house costs, 0.36/0.36,
  `--cross-index SPY`, lookback 8). Every cached name starts 2022-01-03.
- Compute was the limit: the box was shared and ran at 4–10 min per single-name train run. So 6 names ran on `train`, the
  other 31 on `2023_25h1` (a superset of the `2024h2_25h1` screen), and the 15 that the screen or a basket needed were
  extended to `train`. All screen-window numbers are slices of these runs. The four baseline names come from
  `base/v18.train`.
- **Tickers replay independently.** `v18_lly` (AMZN NVDA MSFT LLY in one process) reproduces the single-name trades
  exactly for all four names. So a basket's trades are exactly the union of its single-name cells. The session caps
  (≤ 3 concurrent, ≤ 95 % deployed, a third position trimmed to what is left, as live does) are **not** simulated by the
  replay. I emulate them after the fact on the merged trade list, and every basket number below is **capped**. Capping
  matters little for 4–5 names and a lot for 14+.
- **thrust-1h15 per name** is emulated by dropping thrust-window trades with entry 1h > −0.15. I checked this against the
  real cell `b4_1h15`: the real cell has the same 173 trades plus 34 re-entries the emulation cannot see, worth +156
  (real +1013 / 207 / 1.46 vs emulated +858 / 173 / 1.47). So the emulation is slightly conservative on P&L and right on
  PF. Basket 1h15 rows are marked `~`.

## A. per-name results

### T1 screen, v18 engine (all 41 names; sorted by 2024h2_25h1 P&L)

| name | 24h2_25h1 | 23_25h1 | 2022 | 2023 | 2024 | 2025h1 | 22h1 | 22h2 | 23h1 | 23h2 | 24h1 | 2024h2 | train |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| TSLA | +605/45/1.99 | +191/123/1.09 | -414/47/0.64 | -201/48/0.80 | -213/50/0.73 | +604/25/3.11 | +59/23/1.13 | -473/24/0.30 | +32/25/1.07 | -233/23/0.56 | -214/30/0.54 | +1/20/1.00 | -223/170/0.93 |
| MU | +347/32/1.82 | -18/54/0.98 | +7/23/1.02 | -98/14/0.48 | -173/27/0.67 | +253/13/2.52 | +228/8/7.38 | -222/15/0.23 | -37/7/0.54 | -61/7/0.44 | -268/8/0.02 | +94/19/1.37 | -11/77/0.99 |
| JPM | +283/11/7.61 | +321/18/4.10 | -100/13/0.59 | +4/3/1.25 | +45/6/1.96 | +272/9/7.50 | -15/8/0.89 | -85/5/0.23 | -15/2/0.00 | +18/1/99.00 | +35/4/1.75 | +11/2/12.93 | +221/31/1.64 |
| LLY | +235/29/1.81 | +335/66/1.49 | -105/19/0.67 | +237/23/2.57 | +69/25/1.21 | +29/18/1.14 | -167/13/0.34 | +62/6/1.97 | +4/10/1.06 | +233/13/3.83 | -137/14/0.42 | +206/11/3.36 | +230/85/1.23 |
| AMD | +197/15/4.39 | -231/73/0.79 | +476/22/6.56 | +132/28/1.36 | -335/43/0.51 | -28/2/0.00 | +218/10/7.26 | +258/12/6.08 | -31/12/0.79 | +163/16/1.74 | -560/30/0.14 | +226/13/8.56 | +246/95/1.21 |
| NFLX | +178/21/2.01 | +154/61/1.24 | -37/31/0.94 | +75/24/1.26 | -70/29/0.77 | +149/8/3.30 | +66/23/1.16 | -102/8/0.45 | -42/14/0.81 | +117/10/2.76 | -99/16/0.47 | +29/13/1.26 | +117/92/1.09 |
| UBER | +176/16/1.76 | -81/38/0.88 | -166/21/0.63 | -164/13/0.30 | +270/20/2.13 | -186/5/0.09 | -14/11/0.94 | -152/10/0.23 | -50/5/0.43 | -115/8/0.22 | -92/9/0.57 | +362/11/14.02 | -246/59/0.78 |
| NVDA | +169/39/1.29 | +359/112/1.24 | +626/35/2.67 | +164/45/1.29 | +133/56/1.20 | +62/11/1.25 | +489/22/5.06 | +136/13/1.54 | -4/19/0.99 | +168/26/1.59 | +25/28/1.08 | +108/28/1.32 | +984/147/1.52 |
| UNH | +164/32/1.48 | +12/57/1.02 | -68/29/0.74 | -144/15/0.37 | -12/29/0.95 | +167/13/2.22 | -60/14/0.64 | -8/15/0.92 | -53/8/0.55 | -91/7/0.17 | -9/10/0.82 | -3/19/0.98 | -56/86/0.94 |
| ADBE | +125/27/1.37 | +43/65/1.06 | -31/21/0.91 | -204/21/0.32 | +126/28/1.41 | +121/16/1.77 | -18/11/0.90 | -13/10/0.92 | -77/9/0.46 | -127/12/0.20 | +122/17/1.94 | +4/11/1.02 | +12/86/1.01 |
| META | +122/35/1.35 | -152/71/0.82 | +132/30/1.29 | -223/20/0.22 | -76/37/0.81 | +147/14/2.03 | +129/13/1.62 | +3/17/1.01 | -193/10/0.00 | -30/10/0.67 | -51/16/0.74 | -25/21/0.88 | -21/101/0.98 |
| MRK | +114/8/1.96 | +145/9/2.22 | – | +0/0/0.00 | +202/3/54.58 | -57/6/0.50 | – | – | +0/0/0.00 | +0/0/0.00 | +31/1/99.00 | +171/2/46.38 | –
| SPY | +98/11/4.18 | +67/17/1.89 | – | -29/3/0.00 | +78/6/6.08 | +17/8/1.55 | – | – | -11/1/0.00 | -18/2/0.00 | -2/3/0.86 | +81/3/99.00 | –
| QQQ | +94/14/2.17 | +45/30/1.25 | – | -5/7/0.90 | +5/13/1.06 | +44/10/1.85 | – | – | -33/3/0.00 | +28/4/3.23 | -44/9/0.12 | +49/4/2.78 | –
| GS | +89/13/1.54 | -1/28/1.00 | – | +45/9/1.39 | -155/11/0.27 | +109/8/2.03 | – | – | +47/6/1.52 | -1/3/0.95 | -135/6/0.13 | -20/5/0.65 | –
| SMH | +87/8/5.18 | -4/32/0.98 | – | -129/13/0.18 | +132/17/3.21 | -7/2/0.34 | – | – | -87/9/0.13 | -42/4/0.28 | +39/11/1.79 | +94/6/9.66 | –
| MA | +84/11/2.03 | +56/21/1.31 | -203/24/0.56 | +1/8/1.01 | +8/6/1.20 | +47/7/1.65 | -121/17/0.61 | -83/7/0.46 | +10/7/1.16 | -9/1/0.00 | -30/2/0.00 | +38/4/4.50 | -148/45/0.77 |
| TXN | +84/17/1.76 | +46/28/1.23 | -79/11/0.61 | -19/5/0.58 | -45/14/0.61 | +110/9/4.28 | -43/4/0.54 | -35/7/0.67 | -46/3/0.00 | +27/2/99.00 | -19/6/0.52 | -27/8/0.65 | -33/39/0.92 |
| PLTR | +73/28/1.11 | +318/69/1.23 | -92/37/0.89 | +365/32/1.60 | -223/21/0.41 | +177/16/1.47 | -206/21/0.61 | +113/16/1.38 | +234/9/2.50 | +131/23/1.29 | -119/9/0.00 | -104/12/0.60 | +226/106/1.10 |
| CVX | +66/4/3.43 | +51/28/1.18 | +29/19/1.17 | +44/19/1.24 | -67/6/0.21 | +74/3/4.95 | +66/10/1.69 | -37/9/0.50 | +102/9/2.72 | -58/10/0.53 | -58/5/0.24 | -8/1/0.00 | +79/47/1.17 |
| ORCL | +41/12/1.33 | -6/19/0.98 | – | +11/2/1.17 | -39/7/0.37 | +22/10/1.19 | – | – | +0/0/0.00 | +11/2/1.17 | -58/5/0.00 | +19/2/5.74 | –
| SHOP | +19/20/1.05 | -346/58/0.72 | +714/20/4.63 | -445/23/0.35 | +4/20/1.01 | +95/15/1.42 | +540/15/3.83 | +174/5/28.84 | -449/12/0.01 | +4/11/1.02 | +81/15/1.52 | -77/5/0.51 | +368/78/1.26 |
| AVGO | +16/23/1.05 | -106/65/0.85 | +20/22/1.13 | -99/27/0.57 | +185/29/1.93 | -191/9/0.29 | +35/11/1.36 | -15/11/0.76 | -60/13/0.39 | -39/14/0.71 | -22/15/0.81 | +207/14/3.50 | -85/87/0.90 |
| XLF | +16/1/99.00 | +16/1/99.00 | – | +0/0/0.00 | +0/0/0.00 | +16/1/99.00 | – | – | +0/0/0.00 | +0/0/0.00 | +0/0/0.00 | +0/0/0.00 | –
| XLK | -7/5/0.87 | -50/13/0.59 | – | -3/2/0.87 | -40/6/0.10 | -7/5/0.87 | – | – | +0/0/0.00 | -3/2/0.87 | -40/6/0.10 | +0/0/0.00 | –
| AMZN | -10/19/0.96 | -405/46/0.49 | +124/24/1.49 | -385/23/0.16 | -48/10/0.71 | +28/13/1.16 | +208/15/2.98 | -84/9/0.44 | -145/11/0.22 | -239/12/0.11 | -10/4/0.77 | -38/6/0.68 | -280/70/0.73 |
| PG | -14/4/0.58 | -3/7/0.93 | – | +11/3/2.01 | -32/2/0.02 | +18/2/99.00 | – | – | -11/2/0.00 | +22/1/99.00 | +0/0/0.00 | -32/2/0.02 | –
| MSFT | -14/25/0.93 | +187/52/1.59 | +289/17/4.84 | +169/18/3.58 | -22/21/0.87 | +41/13/1.47 | +158/7/4.33 | +131/10/5.72 | +63/10/2.25 | +105/8/8.07 | +33/9/1.67 | -55/12/0.54 | +476/69/2.21 |
| CAT | -22/13/0.85 | +13/35/1.04 | +120/10/2.14 | +35/18/1.22 | -129/10/0.14 | +106/7/7.49 | +194/9/7.19 | -74/1/0.00 | +18/12/1.14 | +17/6/1.49 | -1/4/0.97 | -128/6/0.02 | +133/45/1.30 |
| XOM | -23/6/0.51 | +64/20/1.63 | +164/18/2.05 | +64/11/2.34 | -10/6/0.75 | +11/3/1.80 | +212/8/6.82 | -48/10/0.60 | +11/6/1.65 | +53/5/2.71 | +24/3/4.27 | -34/3/0.00 | +228/38/1.88 |
| IWM | -24/8/0.80 | -114/17/0.49 | – | -57/4/0.00 | +51/6/2.11 | -109/7/0.10 | – | – | -49/3/0.00 | -8/1/0.00 | -33/5/0.28 | +85/1/99.00 | –
| V | -24/11/0.71 | -30/23/0.81 | – | +6/9/1.10 | -62/8/0.00 | +25/6/1.76 | – | – | +23/5/1.77 | -16/4/0.51 | -12/3/0.00 | -49/5/0.00 | –
| XLE | -31/4/0.07 | -107/18/0.31 | – | -46/12/0.50 | -63/5/0.00 | +2/1/99.00 | – | – | -9/7/0.75 | -36/5/0.34 | -30/2/0.00 | -33/3/0.00 | –
| COIN | -43/33/0.93 | -633/100/0.74 | – | -504/37/0.58 | -142/50/0.86 | +13/13/1.05 | – | – | +28/19/1.06 | -531/18/0.29 | -87/30/0.87 | -55/20/0.84 | –
| ABBV | -45/10/0.40 | -19/14/0.85 | +55/10/1.68 | -49/3/0.00 | +17/6/1.28 | +13/5/1.91 | -29/4/0.00 | +84/6/2.61 | -49/3/0.00 | +0/0/0.00 | +75/1/99.00 | -58/5/0.05 | +36/24/1.18 |
| CRM | -52/16/0.73 | -244/42/0.51 | – | -97/14/0.19 | -83/18/0.62 | -65/10/0.60 | – | – | -39/7/0.26 | -57/7/0.14 | -96/12/0.51 | +13/6/1.47 | –
| COST | -63/7/0.61 | -139/21/0.54 | – | +6/9/1.10 | -123/11/0.45 | -22/1/0.00 | – | – | -34/6/0.43 | +40/3/265.47 | -82/5/0.00 | -41/6/0.71 | –
| AAPL | -86/14/0.49 | -51/27/0.83 | +50/11/1.51 | +49/5/2.97 | -80/14/0.64 | -20/8/0.60 | +116/5/10.42 | -66/6/0.24 | +11/2/3.41 | +37/3/2.87 | -14/8/0.87 | -66/6/0.45 | -1/38/1.00 |
| HD | -146/11/0.32 | -207/27/0.44 | – | -64/8/0.44 | -77/11/0.36 | -66/8/0.51 | – | – | -40/7/0.56 | -24/1/0.00 | +3/8/1.06 | -80/3/0.00 | –
| QCOM | -202/15/0.31 | -440/43/0.37 | – | -26/15/0.85 | -329/22/0.26 | -85/6/0.00 | – | – | -20/9/0.81 | -6/6/0.91 | -212/13/0.09 | -117/9/0.44 | –
| AMAT | -263/18/0.37 | -465/50/0.47 | -47/21/0.86 | -253/17/0.28 | -125/25/0.71 | -87/8/0.15 | -4/12/0.98 | -42/9/0.59 | -146/8/0.02 | -107/9/0.47 | +50/15/1.44 | -176/10/0.45 | -512/71/0.58 |

### T2 screen, thrust-1h15 (emulated by dropping thrust trades with entry 1h > −0.15; see caveat)

| name | 24h2_25h1 | 23_25h1 | 2022 | 2023 | 2024 | 2025h1 |
|---|---|---|---|---|---|---|
| MU | +344/23/2.05 | +219/36/1.40 | -29/17/0.90 | -44/10/0.67 | +1/17/1.01 | +261/9/2.64 |
| TSLA | +304/27/1.93 | +347/75/1.30 | -512/32/0.42 | +92/28/1.17 | -161/33/0.70 | +416/14/7.04 |
| LLY | +279/16/3.06 | +86/39/1.18 | -253/13/0.09 | +22/15/1.17 | -23/12/0.89 | +87/12/1.66 |
| UBER | +229/8/17.76 | -92/25/0.78 | -79/16/0.78 | -166/10/0.17 | +56/13/1.25 | +18/2/39.89 |
| AMZN | +170/11/3.05 | +51/28/1.17 | -102/10/0.47 | -149/15/0.30 | +80/6/3.34 | +120/7/3.33 |
| AMD | +153/8/68.66 | -41/40/0.92 | +277/15/5.16 | +147/13/1.95 | -188/27/0.49 | +0/0/0.00 |
| META | +134/20/1.65 | +3/45/1.01 | +80/18/1.26 | -158/13/0.14 | -45/23/0.83 | +207/9/5.26 |
| NVDA | +127/21/1.44 | +343/55/1.48 | +318/24/2.29 | +191/20/1.89 | +59/28/1.16 | +93/7/1.66 |
| TXN | +108/7/4.42 | +80/10/2.35 | +39/4/2.34 | -3/1/0.00 | -12/4/0.70 | +96/5/7.59 |
| SMH | +84/5/8.78 | +0/20/1.00 | – | -103/8/0.02 | +103/12/4.12 | +0/0/0.00 |
| JPM | +66/5/2.57 | +37/8/1.52 | +49/4/7.91 | -15/2/0.00 | -14/1/0.00 | +66/5/2.57 |
| SPY | +35/1/99.00 | +32/2/9.81 | – | -4/1/0.00 | +35/1/99.00 | +0/0/0.00 |
| GS | +33/6/1.25 | -44/17/0.88 | – | +52/7/1.49 | -144/8/0.21 | +49/2/1.60 |
| NFLX | +29/12/1.21 | +19/33/1.05 | -157/24/0.69 | -35/13/0.77 | -14/15/0.90 | +68/5/2.10 |
| UNH | +19/15/1.09 | +11/27/1.03 | +37/13/1.37 | -16/9/0.84 | +103/12/1.99 | -77/6/0.34 |
| ABBV | +8/6/1.43 | -35/8/0.44 | -15/4/0.59 | -43/2/0.00 | -5/1/0.00 | +13/5/1.91 |
| COIN | +7/20/1.02 | -933/64/0.48 | – | -675/28/0.33 | -341/30/0.49 | +82/6/1.73 |
| XOM | +5/4/1.23 | +33/12/1.52 | +2/10/1.03 | +1/6/1.03 | +17/4/2.27 | +15/2/2.53 |
| MA | +3/9/1.04 | -40/14/0.74 | -35/11/0.86 | -23/4/0.54 | -22/4/0.27 | +6/6/1.08 |
| XLF | +0/0/0.00 | +0/0/0.00 | – | +0/0/0.00 | +0/0/0.00 | +0/0/0.00 |
| V | -3/6/0.93 | -14/14/0.87 | – | -11/8/0.83 | -20/2/0.00 | +16/4/1.60 |
| HD | -4/5/0.95 | -11/13/0.93 | – | -37/4/0.43 | +30/4/3.15 | -4/5/0.95 |
| XLK | -8/1/0.00 | -8/3/0.35 | – | +0/0/0.00 | -0/2/0.96 | -8/1/0.00 |
| CRM | -9/8/0.90 | -88/20/0.68 | – | -36/6/0.20 | -12/8/0.91 | -39/6/0.56 |
| PG | -14/4/0.58 | +8/5/1.24 | – | +22/1/99.00 | -32/2/0.02 | +18/2/99.00 |
| QQQ | -15/1/0.00 | -21/2/0.00 | – | -6/1/0.00 | +0/0/0.00 | -15/1/0.00 |
| MSFT | -17/13/0.83 | +82/24/1.68 | +297/11/57.12 | +67/7/6.64 | +47/11/1.96 | -31/6/0.47 |
| XLE | -23/3/0.09 | -60/10/0.32 | – | -8/5/0.77 | -55/4/0.00 | +2/1/99.00 |
| CVX | -27/3/0.00 | -112/12/0.22 | +35/9/1.39 | -55/5/0.20 | -38/5/0.32 | -19/2/0.00 |
| IWM | -31/3/0.27 | -88/7/0.18 | – | -31/2/0.00 | -26/2/0.22 | -31/3/0.27 |
| SHOP | -40/7/0.63 | -405/36/0.44 | +578/11/6.44 | -394/17/0.20 | -28/13/0.84 | +17/6/1.32 |
| ORCL | -56/4/0.04 | -76/6/0.03 | – | +0/0/0.00 | -24/3/0.00 | -52/3/0.04 |
| ADBE | -80/17/0.64 | -196/38/0.63 | +54/12/1.33 | -176/11/0.17 | +26/17/1.12 | -46/10/0.57 |
| COST | -82/3/0.00 | -94/9/0.25 | – | -11/5/0.74 | -83/4/0.00 | +0/0/0.00 |
| QCOM | -90/9/0.50 | -155/19/0.44 | – | -38/6/0.43 | -81/11/0.53 | -35/2/0.00 |
| MRK | -93/5/0.08 | -62/6/0.39 | – | +0/0/0.00 | +27/2/8.20 | -89/4/0.09 |
| AMAT | -96/8/0.51 | -226/25/0.55 | -145/14/0.46 | -194/9/0.25 | +8/14/1.04 | -40/2/0.00 |
| AAPL | -111/9/0.21 | -114/15/0.40 | -16/6/0.77 | -8/4/0.66 | -111/5/0.21 | +5/6/1.22 |
| AVGO | -111/15/0.56 | -223/31/0.49 | +73/13/2.27 | -84/10/0.27 | +38/14/1.26 | -176/7/0.00 |
| CAT | -116/7/0.04 | -108/21/0.53 | +92/9/1.87 | +24/12/1.26 | -131/7/0.02 | -2/2/0.58 |
| PLTR | -304/15/0.20 | -420/43/0.58 | +14/30/1.02 | -51/23/0.91 | -227/13/0.00 | -142/7/0.35 |

Screen-window PF distribution (v18 engine, 37 non-baseline names): on `2024h2_25h1`, 17 names have ≥ 15 trades, with PF
min 0.31 / q25 1.05 / median 1.37 / q75 1.81 / max 4.39, and 23 of 37 are positive. On `2023_25h1`, 32 names have
≥ 15 trades, with PF min 0.31 / q25 0.72 / **median 0.98** / q75 1.23 / max 4.10, and 16 of 37 are positive. On thrust-1h15
the median is 1.09 and 0.78.
The names positive with ≥ 15 trades on the screen were TSLA MU LLY AMD NFLX UBER NVDA UNH ADBE META TXN PLTR SHOP AVGO
(14). Of those, NVDA (+984), SHOP (+368, all of it 2022), AMD (+246, all of it 2022), LLY (+230), PLTR (+226), NFLX (+117)
and ADBE (+12) are positive over `train`. TSLA, MU, UBER, UNH, META, TXN and AVGO are flat or negative over `train`: they
are regime fits.

## B. baskets (capped: ≤ 3 concurrent, 95 % deployed)

Definitions, fixed before looking at basket results: **B4** = the baseline four. **SEL** = every stock positive on
2023_25h1 with ≥ 15 trades (14 names: ADBE CAT CVX JPM LLY MA MSFT NFLX NVDA PLTR TSLA TXN UNH XOM). **SELQ** = the subset
with ≥ 30 trades and PF ≥ 1.2 (LLY MSFT NFLX NVDA PLTR). **U8** ("eight-name union") = B4 + the top four new names by
2023_25h1 P&L (LLY JPM PLTR TSLA). **SEL15** = SEL rebuilt from the 1h15 screen (AMZN LLY META MSFT MU NFLX NVDA TSLA UNH).

| basket · engine | 2022 | 2023 | 2024 | 2025h1 | 22h1 | 22h2 | 23h1 | 23h2 | 24h1 | 24h2 | **2023_25h1** | train | top name, share of 23_25h1 P&L |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| B4 · v18 | +1100/87/2.40 | +50/90/1.05 | −23/99/0.98 | +110/45/1.20 | +970/49/4.46 | +130/38/1.26 | −14/41/0.97 | +64/49/1.11 | +37/48/1.07 | −61/51/0.91 | **+136/234/1.05** | +1236/321/1.34 | NVDA 297 % |
| U8 · v18 | +337/194/1.11 | +386/186/1.14 | −345/198/0.87 | +1012/107/1.75 | +545/111/1.34 | −208/83/0.87 | +147/84/1.12 | +240/102/1.15 | −390/104/0.72 | +45/94/1.03 | **+1054/491/1.15** | +1391/685/1.14 | NVDA 43 % |
| SEL · v18 | +243/285/1.06 | +757/267/1.23 | −470/298/0.87 | +1480/145/1.91 | +673/168/1.28 | −430/117/0.77 | +401/128/1.26 | +356/139/1.20 | −453/152/0.74 | −17/146/0.99 | **+1767/710/1.21** | +2009/995/1.16 | NVDA 26 % |
| SELQ · v18 | +711/138/1.33 | +903/139/1.55 | −82/151/0.95 | +452/66/1.46 | +378/85/1.29 | +333/53/1.40 | +170/60/1.22 | +733/79/1.83 | −274/75/0.70 | +192/76/1.21 | **+1273/356/1.29** | +1984/494/1.30 | NVDA 29 % |
| NVDA · v18 | +626/35/2.67 | +164/45/1.29 | +133/56/1.20 | +62/11/1.25 | +489/22/5.06 | +136/13/1.54 | −4/19/0.99 | +168/26/1.59 | +25/28/1.08 | +108/28/1.32 | **+359/112/1.24** | +984/147/1.52 | – |
| ETF6 · v18 (real cell) | −49/58/0.93 | −232/38/0.37 | +98/47/1.35 | −46/26/0.79 | +53/25/1.16 | −101/33/0.70 | −177/22/0.19 | −56/16/0.64 | −96/33/0.53 | +194/14/3.70 | **−180/111/0.79** | −229/169/0.85 | – |
| 37 others · v18 | – | −1821/519/0.77 | −1430/633/0.83 | +1731/309/1.45 | – | – | −1071/268/0.72 | −750/251/0.81 | −2051/348/0.57 | +621/285/1.17 | **−1520/1461/0.92** (capped −657/1081/0.95) | – | – |
| B4 · 1h15 (real `b4_1h15`, uncapped) | +741/59/2.37 | +55/58/1.09 | −56/62/0.93 | +273/28/2.00 | | | | | | | | +1013/207/1.46 | |
| B4 · 1h15~ | +496/51/1.96 | +110/46/1.25 | +72/50/1.12 | +187/26/1.67 | +568/28/4.47 | −73/23/0.79 | +58/20/1.31 | +52/26/1.20 | +93/22/1.38 | −22/28/0.94 | **+369/122/1.28** | +865/173/1.47 | NVDA 96 % |
| U8 · 1h15~ | −185/128/0.92 | +192/113/1.12 | −373/108/0.76 | +545/62/1.74 | | | | | | | **+364/283/1.09** | +179/411/1.03 | TSLA 102 % |
| SEL · 1h15~ | −73/200/0.98 | +61/154/1.03 | −397/165/0.81 | +452/88/1.43 | | | | | | | **+116/407/1.02** | +44/607/1.01 | NVDA 287 % |
| SELQ · 1h15~ | +245/102/1.14 | +205/78/1.20 | −159/79/0.84 | +74/37/1.12 | | | | | | | **+120/194/1.05** | +366/296/1.08 | NVDA 289 % |
| SEL15 · 1h15~ | −331/159/0.88 | +55/126/1.04 | +17/154/1.01 | +1026/73/2.22 | +369/89/1.27 | −700/70/0.48 | +265/59/1.44 | −210/67/0.78 | −188/73/0.81 | +205/81/1.21 | **+1098/353/1.25** | +766/512/1.11 | NVDA 37 % |
| 37 others · 1h15~ | – | | | +662/156/1.33 | | | −381/151/0.83 | −1673/144/0.42 | −1260/184/0.54 | −23/160/0.99 | **−2675/795/0.78** | – | – |

How to read it: adding names buys trades and 2025h1 P&L (SEL 2025h1 +1480, PF 1.91) and gives back 2022 and 2024. The
baskets beat B4 on 2023_25h1 only because their names were chosen on 2023_25h1. SELQ is the only basket that holds up on
2022 (+711 / PF 1.33), and that is mostly NVDA + MSFT (+915 of it). LLY, NFLX and PLTR are −234 in 2022.

### B2. the coordinator's AAPL-drop / LLY-swap books (capped; slices of `r21_5n`, `nt10_5n`, `base/v18`, `v18_lly`)

| book · engine | 2022 | 2023 | 2024 | 2025h1 | 23h1 | 23h2 | 24h1 | 24h2 | **2023_25h1** | train | NVDA share 23_25h1 / train |
|---|---|---|---|---|---|---|---|---|---|---|---|
| B4 · v18 | +1100/87/2.40 | +50/90/1.05 | −23/99/0.98 | +110/45/1.20 | −14/41/0.97 | +64/49/1.11 | +37/48/1.07 | −61/51/0.91 | **+136/234/1.05** | +1236/321/1.34 | 297 % / 84 % |
| B4−AAPL · v18 | +1039/76/2.47 | −29/86/0.97 | +64/87/1.06 | +130/37/1.26 | −58/40/0.88 | +29/46/1.05 | +49/41/1.11 | +15/46/1.03 | **+165/210/1.06** | +1204/286/1.37 | 231 % |
| B4−AAPL+LLY · v18 (`v18_lly`) | +932/95/1.92 | +191/107/1.16 | +133/112/1.10 | +149/55/1.21 | −50/49/0.91 | +241/58/1.37 | −88/55/0.87 | +222/57/1.34 | **+473/274/1.15** | +1405/369/1.33 | 82 % |
| B4 · r21 | +809/71/2.30 | +136/63/1.23 | +71/68/1.08 | +285/31/1.91 | −20/25/0.92 | +156/38/1.47 | −5/34/0.99 | +76/34/1.19 | **+492/162/1.28** | +1301/233/1.54 | 99 % / 70 % |
| B4−AAPL · r21 | +791/64/2.43 | +128/57/1.22 | +216/60/1.32 | +325/26/2.24 | −45/23/0.84 | +173/34/1.56 | +22/30/1.06 | +194/30/1.67 | **+669/143/1.44** | +1460/207/1.70 | 71 % / 61 % |
| B4−AAPL+LLY · r21 | +651/79/1.80 | +97/76/1.13 | +272/78/1.30 | +348/38/1.87 | −66/32/0.81 | +163/44/1.41 | −146/41/0.76 | +418/37/2.39 | **+718/192/1.35** | +1368/271/1.48 | 66 % |
| B4 · nt10 | +1054/94/2.41 | +85/71/1.10 | +546/86/1.56 | +250/34/1.74 | −65/31/0.83 | +150/40/1.34 | +160/41/1.33 | +386/45/1.79 | **+881/191/1.41** | +1935/285/1.67 | 94 % / 75 % |
| **B4−AAPL · nt10 (`nt10_b3`)** | +1077/83/2.71 | +89/66/1.11 | +691/75/1.90 | +324/27/2.29 | −56/28/0.84 | +144/38/1.33 | +191/34/1.48 | +501/41/2.34 | **+1104/168/1.61** | +2181/251/1.89 | 74 % / 66 % |
| B4−AAPL+LLY · nt10 | +736/103/1.71 | +35/86/1.04 | +797/100/1.79 | +347/40/1.88 | −31/35/0.92 | +66/51/1.11 | +59/49/1.10 | +738/51/2.83 | **+1180/226/1.49** | +1916/329/1.56 | 69 % |

LLY alone: v18 train +230 / 85 / 1.23 (2022 −105, 2023 +237, 2024 +69, 25h1 +29), r21 −32 / 66 / 0.96, nt10 −174 / 81 / 0.83
(2022 −329). Meta's "LLY +335 / 66 / PF 1.49 post-2022" is v18's 2023_25h1 number exactly. On the engines that improve
the four, LLY is worse, and its record is 2023h2 + 2024h2. LLY is not a robust replacement.

`nt10_b3` as an uncapped lab cell (`research/lab/runs/universe__nt10_b3.train.json`): train +2212 / 251 / PF 1.90, sign-flip
p 0.0003, ex-best-year PF 1.62, dd −256, worst day −133. Years: 22 +1081/83, 23 +82/66, 24 +705/75, 25h1 +345/27.
Paired day bootstrap vs `indicators/r21_v020_noadx_t10` on 2023_25h1: +191, P(diff ≤ 0) 0.050, daily correlation 0.98.
That "significance" is what removing an in-sample loser always produces, so it says nothing about the holdout.

## C. selection-bias controls

C1. Select names positive in period S, then score them in period T (37 non-baseline names, v18 / 1h15~ / nt10 engines):

| select on → score on | engine | min n | selected | selected in T | rest in T | Spearman(P&L S, P&L T) |
|---|---|---|---|---|---|---|
| 2024h2 → 2025h1 | v18 | 5 | 10 | +678/108/1.44 | +1053/201/1.45 | −0.09 |
| 2024h2 → 2025h1 | v18 | 10 | 8 | +750/96/1.54 | +981/213/1.39 | −0.09 |
| 2023 → 2024 | v18 | 10 | 7 | −764/140/0.61 | −666/493/0.90 | −0.16 |
| 23h1–24h1 → 24h2–25h1 | v18 | 10 | 4 | +263/76/1.23 | +2089/518/1.33 | −0.04 |
| 2024 → 2025h1 | v18 | 10 | 7 | −95/75/0.92 | +1826/234/1.66 | −0.20 |
| **2023_25h1 → 2022** (24 names with 2022) | v18 | 15 | 14 | **+98/331/1.02** | **+1366/204/1.56** | **−0.32** |
| 2024h2 → 2025h1 | 1h15~ | 5 | 6 | +26/24/1.06 | +636/132/1.41 | −0.05 |
| 2023 → 2024 | 1h15~ | 10 | 4 | −503/79/0.60 | −780/265/0.78 | −0.22 |
| 2023_25h1 → 2022 | 1h15~ | 15 | 9 | −323/162/0.88 | +923/168/1.38 | −0.20 |
| 2024h2 → 2025h1 | nt10 | 10 | 6 | +279/45/1.43 | +730/224/1.24 | −0.23 |
| 2023 → 2024 | nt10 | 10 | 4 | −303/61/0.64 | −1807/554/0.77 | +0.21 |
| 2023_25h1 → 2022 | nt10 | 10 | 7 | −267/113/0.83 | +156/612/1.02 | −0.12 |

The six-month shift the brief asked for (2024h2 → 2025h1) is a wash: the selected names score PF 1.44–1.54 and the rest
1.39–1.45, because 2025h1 lifts every name. Over longer gaps, selection hurts. The one backward test, on 2022, a year no
selection touched, is the most damning: names positive on 2023_25h1 make PF 1.02 in 2022, against 1.56 for the names
left out. My estimate is that a basket's PF on the window it was selected on runs about 0.2–0.4 too high.

C2. Unselected universe (no stock picking at all). The pooled 37 others are PF 0.92 on 2023_25h1 on v18, 0.78 on 1h15~ and
0.79 on nt10. The 30 non-ETF stocks alone are no better (0.93). This matches HISTORY §B6 and §D8: the edge does not
generalise across names on IEX bars.

C3. Mechanism hint, not tested further: v18 scores MACD in dollars (`normalization_factor` $1, HISTORY §F). A $1 MACD
means something different on PLTR ($20–180) than on LLY ($300–900) or COST ($500–1000). So the name-to-name differences
are partly a units artefact, which is another reason a name's history should not predict its future here. Engine proposal
(not implemented): rerun the universe screen with `normalization_pct` (RQ-12, indicators desk) before any further
universe work. With price-invariant scores, a name screen would at least be measuring the same strategy on each name.

## D. cell list (all `--window` as stated; numbers uncapped lab metrics unless noted)

| cell | what | window | P&L / n / PF |
|---|---|---|---|
| `u_<name>` ×37 | v18 single name | train (abbv adbe amat amd avgo cat cvx jpm lly ma meta mu nflx pltr shop tsla txn uber unh xom) / 2023_25h1 (other 17) | table A |
| `b4_1h15` | four-name, thrust 1h ≤ −0.15 (real) | train | +1013 / 207 / 1.46 |
| `v18_lly` | AMZN NVDA MSFT LLY, v18 | train | +1409 / 371 / 1.33 |
| `r21_5n` | row 21 on four + LLY (sliced) | train | +1285 / 299 / 1.40 |
| `nt10_5n` | indicators-desk best engine on four + LLY (sliced) | train | +1823 / 368 / 1.47 |
| `etf6` | SMH XLK XLF XLE IWM QQQ, v18 | train | −235 / 170 / 0.85 |
| `nt10_u37` | best engine on all 37 others, one book | train | −4214 / 2080 / 0.87 |
| `nt10_b3` | best engine, AMZN NVDA MSFT | train | +2212 / 251 / 1.90 |
| `sel_q`, `sel14`, `u8` | basket specs (union of single-name cells + cap emulation; not run as cells) | – | table B |

Dead ends: per-name selection (any window), sector ETFs, LLY as an AAPL replacement, thrust-1h15 on other names, the new
engine on other names, and baskets of ≥ 8 names (the cap binds, and the extra names are regime fits).

## nominations

**Baskets: none.** No basket reaches 2023_25h1 PF ≥ 1.3 with ≥ 100 trades and no name above 40 %. The nearest, SELQ at
PF 1.29, was selected on that same window, and C1 says to expect it to lose about 0.3 of PF out of sample.

**1. `research/lab/specs/universe/nt10_b3.json` — low confidence, nominated only because the coordinator asked for this
book.** It is the indicators desk's `r21_v020_noadx_t10` without AAPL. It is the best of the 2 × 3 books in B2 and the
best of ~12 single-name drops and swaps across engines, so it is a best-of-~12 pick on top of that engine's own search.
- Train: uncapped +2212 / 251 / 1.90 (capped +2181 / 251 / 1.89). 2023_25h1 +1104 / 168 / 1.61. Every year positive, but
  2023 is +82. NVDA is 66 % of train P&L, which fails the 40 % basket rule. It is a subset of the promoted book, not a
  new basket.
- Written expectation for the holdout (2025-07-01 → 2026-10-30), set down before any reveal:
  - roughly 85–120 trades, which fails test 1's 150-trade bar on the target window as well (2025h1 ran at 27 trades per
    half);
  - holdout PF 1.15–1.6, central 1.35;
  - P&L vs the four-name `r21_v020_noadx_t10` on the holdout: within ±$150, P(better) ≈ 55 %. My own controls say a
    name's past P&L does not predict its future, and AAPL lost only about $60 a year in-sample.
  - If the holdout shows AAPL positive, that is the expected outcome, not a surprise.
  - Recommendation: give it a finalist slot only if one is free. It is a diagnostic ("does AAPL's drag persist?"), not a
    strong contender.

No second or third nomination. Every other universe cell is either selected in-sample (C1 says those do not persist) or
negative unselected (C2).
