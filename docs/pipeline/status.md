# research pipeline — status

generated 2026-10-08 05:12 UTC by `scripts/pipeline/pipeline.py report`. do not edit: regenerated nightly. lane: proposed → backtesting → backtest_passed → shadow → shadow_passed → promotion_proposed → promoted (human). failures: backtest_failed / shadow_failed (180 d cooldown), withdrawn (human).

- gate sweeps run at the research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, IEX cache, 3 bps + $0.005); the per-year floor (no year < −300) is stated at 36 %. live and shadow books size at the blob's fraction; parity replays use no sizing override.
- shadow-book creation: **enabled** (`PIPELINE_SHADOW_BOOKS=1`; while disabled `advance` only prints the books it would create).
- baseline `iex_v18` (promoted row on its own tickers): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429

## promotion proposals (open)

none.

## flags (plumbing, not verdicts)

none.

## candidates by stage

### proposed (1)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 47 | `qqq-noise-pm-vol` | config | quality-config | base | 31 m | — |

### backtesting (0)

none.

### backtest_passed (0)

none.

### shadow (6)

| id | name | kind | gate | book | since | trial | backtest |
|---:|---|---|---|---|---|---|---|
| 4 | `vpin-0.26` | config | quality-config | shadow:vpin-0.26 | 7 d | 6 sessions / 0 trades / +0 · needs 14 more sessions and 15 more trades (or 54 sessions to the time limit) | +1646 / 324 / PF 1.48 · 22:+820 23:+42 24:+105 25:+210 26:+469 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |
| 6 | `stress-s15-core` | config | additive-config | shadow:stress-s15-core | 7 d | 6 sessions / 0 trades / +0 · needs 14 more sessions and 15 more trades (or 54 sessions to the time limit) | +2003 / 459 / PF 1.41 · 22:+1226 23:-3 24:-24 25:+376 26:+429 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 · added +274 / 23 / PF 3.32 (worst yr -4, worst day -44) |
| 28 | `size-vpin26-x1.25` | config | sizing-config | shadow:size-vpin26-x1.25 | 5 d | 3 sessions / 0 trades / +0 · needs 17 more sessions and 15 more trades (or 57 sessions to the time limit) | +2155 / 436 / PF 1.40 · 22:+1194 23:+46 24:+10 25:+410 26:+496 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 · added +0 / 0 / PF 0.00 (worst yr +0, worst day +0) |
| 29 | `thrust-1h15` | config | quality-config | shadow:thrust-1h15 | 5 d | 3 sessions / 0 trades / +0 · needs 17 more sessions and 15 more trades (or 57 sessions to the time limit) | +1714 / 278 / PF 1.63 · 22:+644 23:+55 24:-56 25:+396 26:+676 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |
| 44 | `spy-sqrt-band` | config | volume-config | shadow:spy-sqrt-band | 0 m | 0 sessions / 0 trades / +0 · needs 20 more sessions and 15 more trades (or 60 sessions to the time limit) | +2195 / 460 / PF 1.46 · 22:+1144 23:+77 24:+116 25:+371 26:+486 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |
| 45 | `spy-sqrt-band-1h15` | config | quality-config | shadow:spy-sqrt-band-1h15 | 0 m | 0 sessions / 0 trades / +0 · needs 20 more sessions and 15 more trades (or 60 sessions to the time limit) | +2044 / 309 / PF 1.70 · 22:+708 23:+125 24:+81 25:+428 26:+703 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |

### shadow_passed (0)

none.

### promotion_proposed (0)

none.

### backtest_failed (39)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 1 | `bear-bounce-sma50` | config | volume-config | base | 10 d | +2179 / 828 / PF 1.23 · 22:+1422 23:+371 24:-436 25:+61 26:+762 · fail: pf_5y, min_year_pnl · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-26 |
| 2 | `ticker:AMD` | ticker | additive-ticker | AMD | 1 h | -18 / 158 / PF 0.99 · 22:+476 23:+132 24:-335 25:+171 26:-463 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -18 / 158 / PF 0.99 (worst yr -463, worst day -99) · cooldown to 2027-03-26 |
| 3 | `ticker:META` | ticker | additive-ticker | META | 1 h | -19 / 160 / PF 0.99 · 22:+132 23:-223 24:-76 25:+73 26:+76 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -19 / 160 / PF 0.99 (worst yr -223, worst day -102) · cooldown to 2027-03-26 |
| 5 | `rs-long-x0.3` | config | volume-config | base | 9 d | +1997 / 1586 / PF 1.11 · 22:+1490 23:+238 24:-197 25:-213 26:+679 · fail: pf_5y, years_positive · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-28 |
| 7 | `trend-day-ride` | config | additive-config | base | 7 d | +2142 / 803 / PF 1.23 · 22:+2003 23:-260 24:-307 25:+461 26:+246 · fail: added_pf, added_worst_year, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +395 / 367 / PF 1.08 (worst yr -287, worst day -273) · cooldown to 2027-03-29 |
| 8 | `ticker:JPM` | ticker | additive-ticker | JPM | 1 h | +23 / 51 / PF 1.03 · 22:-100 23:+4 24:+45 25:+292 26:-217 · fail: added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added +23 / 51 / PF 1.03 (worst yr -217, worst day -75) · cooldown to 2027-04-04 |
| 9 | `ticker:V` | ticker | additive-ticker | V | 1 h | -245 / 67 / PF 0.64 · 22:-107 23:+6 24:-62 25:+44 26:-128 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -245 / 67 / PF 0.64 (worst yr -128, worst day -69) · cooldown to 2027-04-04 |
| 10 | `ticker:MA` | ticker | additive-ticker | MA | 1 h | -234 / 85 / PF 0.80 · 22:-203 23:+1 24:+8 25:+14 26:-53 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -234 / 85 / PF 0.80 (worst yr -203, worst day -79) · cooldown to 2027-04-05 |
| 11 | `ticker:UNH` | ticker | additive-ticker | UNH | 1 h | -130 / 119 / PF 0.90 · 22:-68 23:-144 24:-12 25:+118 26:-25 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -130 / 119 / PF 0.90 (worst yr -144, worst day -64) · cooldown to 2027-04-05 |
| 12 | `ticker:LLY` | ticker | additive-ticker | LLY | 1 h | +218 / 147 / PF 1.14 · 22:-105 23:+237 24:+69 25:-90 26:+107 · fail: added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +218 / 147 / PF 1.14 (worst yr -105, worst day -74) · cooldown to 2027-04-05 |
| 13 | `ticker:COST` | ticker | additive-ticker | COST | 1 h | -56 / 68 / PF 0.92 · 22:+150 23:+6 24:-123 25:-112 26:+24 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -56 / 68 / PF 0.92 (worst yr -123, worst day -40) · cooldown to 2027-04-05 |
| 14 | `ticker:HD` | ticker | additive-ticker | HD | 1 h | -568 / 92 / PF 0.52 · 22:-71 23:-64 24:-77 25:-186 26:-170 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -568 / 92 / PF 0.52 (worst yr -186, worst day -68) · cooldown to 2027-04-06 |
| 15 | `ticker:XOM` | ticker | additive-ticker | XOM | 1 h | +131 / 51 / PF 1.28 · 22:+164 23:+64 24:-10 25:+25 26:-112 · fail: combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +131 / 51 / PF 1.28 (worst yr -112, worst day -64) · cooldown to 2027-04-06 |
| 16 | `ticker:CVX` | ticker | additive-ticker | CVX | 1 h | +213 / 56 / PF 1.43 · 22:+29 23:+44 24:-67 25:+68 26:+139 · fail: combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +213 / 56 / PF 1.43 (worst yr -67, worst day -29) · cooldown to 2027-04-06 |
| 17 | `ticker:ADBE` | ticker | additive-ticker | ADBE | 1 h | -70 / 122 / PF 0.96 · 22:-31 23:-204 24:+126 25:-54 26:+93 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -70 / 122 / PF 0.96 (worst yr -204, worst day -117) · cooldown to 2027-04-06 |
| 18 | `ticker:CRM` | ticker | additive-ticker | CRM | 1 h | -182 / 106 / PF 0.87 · 22:+88 23:-97 24:-83 25:+7 26:-98 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -182 / 106 / PF 0.87 (worst yr -98, worst day -96) · cooldown to 2027-04-06 |
| 19 | `ticker:ORCL` | ticker | additive-ticker | ORCL | 1 h | +44 / 77 / PF 1.04 · 22:+74 23:+11 24:-39 25:+184 26:-187 · fail: added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added +44 / 77 / PF 1.04 (worst yr -187, worst day -110) · cooldown to 2027-04-06 |
| 20 | `ticker:QCOM` | ticker | additive-ticker | QCOM | 1 h | -534 / 98 / PF 0.66 · 22:+192 23:-26 24:-329 25:-62 26:-310 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -534 / 98 / PF 0.66 (worst yr -329, worst day -96) · cooldown to 2027-04-06 |
| 21 | `ticker:CAT` | ticker | additive-ticker | CAT | 1 h | +267 / 92 / PF 1.26 · 22:+120 23:+35 24:-129 25:+203 26:+37 · fail: combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +267 / 92 / PF 1.26 (worst yr -129, worst day -74) · cooldown to 2027-04-06 |
| 22 | `ticker:GS` | ticker | additive-ticker | GS | 1 h | -564 / 93 / PF 0.57 · 22:-96 23:+45 24:-155 25:+32 26:-390 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -564 / 93 / PF 0.57 (worst yr -390, worst day -81) · cooldown to 2027-04-06 |
| 23 | `ticker:PG` | ticker | additive-ticker | PG | 1 h | -151 / 21 / PF 0.40 · 22:-21 23:+11 24:-32 25:+17 26:-126 · fail: added_trades, added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -151 / 21 / PF 0.40 (worst yr -126, worst day -40) · cooldown to 2027-04-06 |
| 24 | `ticker:ABBV` | ticker | additive-ticker | ABBV | 1 h | -44 / 41 / PF 0.90 · 22:+55 23:-49 24:+17 25:-27 26:-41 · fail: added_pnl, added_pf, combined_pf · base iex_v18: +1728 / 436 / PF 1.37 · added -44 / 41 / PF 0.90 (worst yr -49, worst day -59) · cooldown to 2027-04-06 |
| 25 | `ticker:MRK` | ticker | additive-ticker | MRK | 1 h | +90 / 25 / PF 1.26 · 22:+90 23:+0 24:+202 25:-189 26:-14 · fail: added_trades, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added +90 / 25 / PF 1.26 (worst yr -189, worst day -115) · cooldown to 2027-04-06 |
| 26 | `ticker:TXN` | ticker | additive-ticker | TXN | 1 h | -95 / 66 / PF 0.87 · 22:-79 23:-19 24:-45 25:+162 26:-114 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -95 / 66 / PF 0.87 (worst yr -114, worst day -75) · cooldown to 2027-04-06 |
| 27 | `ticker:AMAT` | ticker | additive-ticker | AMAT | 1 h | -326 / 112 / PF 0.82 · 22:-47 23:-253 24:-125 25:-109 26:+208 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -326 / 112 / PF 0.82 (worst yr -253, worst day -94) · cooldown to 2027-04-06 |
| 30 | `ticker:QQQ` | ticker | additive-ticker | QQQ | 1 h | +53 / 67 / PF 1.10 · 22:+95 23:-5 24:+5 25:+60 26:-103 · fail: added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +53 / 67 / PF 1.10 (worst yr -103, worst day -43) · cooldown to 2027-04-06 |
| 31 | `ticker:SMH` | ticker | additive-ticker | SMH | 1 h | -141 / 90 / PF 0.86 · 22:-75 23:-129 24:+132 25:+165 26:-234 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -141 / 90 / PF 0.86 (worst yr -234, worst day -89) · cooldown to 2027-04-06 |
| 32 | `ticker:XLK` | ticker | additive-ticker | XLK | 1 h | -178 / 38 / PF 0.65 · 22:+41 23:-3 24:-40 25:+16 26:-193 · fail: added_trades, added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -178 / 38 / PF 0.65 (worst yr -193, worst day -79) · cooldown to 2027-04-06 |
| 33 | `ticker:XLF` | ticker | additive-ticker | XLF | 1 h | -46 / 2 / PF 0.26 · 22:-61 23:+0 24:+0 25:+16 26:+0 · fail: added_trades, added_pnl, added_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -46 / 2 / PF 0.26 (worst yr -61, worst day -61) · cooldown to 2027-04-06 |
| 34 | `ticker:IWM` | ticker | additive-ticker | IWM | 1 h | -241 / 27 / PF 0.40 · 22:-65 23:-57 24:+51 25:-109 26:-62 · fail: added_trades, added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -241 / 27 / PF 0.40 (worst yr -109, worst day -59) · cooldown to 2027-04-06 |
| 35 | `ticker:XLE` | ticker | additive-ticker | XLE | 1 h | -204 / 43 / PF 0.57 · 22:+44 23:-46 24:-63 25:-41 26:-97 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added -204 / 43 / PF 0.57 (worst yr -97, worst day -51) · cooldown to 2027-04-06 |
| 36 | `ticker:TSLA` | ticker | additive-ticker | TSLA | 1 h | -569 / 223 / PF 0.85 · 22:-379 23:-201 24:-213 25:+424 26:-200 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -569 / 223 / PF 0.85 (worst yr -379, worst day -110) · cooldown to 2027-04-06 |
| 37 | `ticker:AVGO` | ticker | additive-ticker | AVGO | 1 h | -455 / 143 / PF 0.73 · 22:+20 23:-99 24:+185 25:-210 26:-351 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -455 / 143 / PF 0.73 (worst yr -351, worst day -94) · cooldown to 2027-04-06 |
| 38 | `ticker:NFLX` | ticker | additive-ticker | NFLX | 1 h | +24 / 128 / PF 1.01 · 22:-37 23:+75 24:-70 25:+76 26:-20 · fail: added_pf, combined_pf, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +24 / 128 / PF 1.01 (worst yr -70, worst day -91) · cooldown to 2027-04-06 |
| 39 | `ticker:COIN` | ticker | additive-ticker | COIN | 1 h | -1307 / 217 / PF 0.74 · 22:-193 23:-504 24:-142 25:-335 26:-133 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -1307 / 217 / PF 0.74 (worst yr -504, worst day -129) · cooldown to 2027-04-06 |
| 40 | `ticker:PLTR` | ticker | additive-ticker | PLTR | 1 h | +204 / 160 / PF 1.06 · 22:-92 23:+365 24:-223 25:+390 26:-235 · fail: added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added +204 / 160 / PF 1.06 (worst yr -235, worst day -111) · cooldown to 2027-04-06 |
| 41 | `ticker:MU` | ticker | additive-ticker | MU | 1 h | -317 / 139 / PF 0.87 · 22:+7 23:-98 24:-173 25:+232 26:-284 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -317 / 139 / PF 0.87 (worst yr -284, worst day -150) · cooldown to 2027-04-06 |
| 42 | `ticker:SHOP` | ticker | additive-ticker | SHOP | 1 h | +622 / 113 / PF 1.30 · 22:+708 23:-445 24:+4 25:+43 26:+312 · fail: combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added +622 / 113 / PF 1.30 (worst yr -445, worst day -103) · cooldown to 2027-04-06 |
| 43 | `ticker:UBER` | ticker | additive-ticker | UBER | 1 h | -305 / 84 / PF 0.79 · 22:-166 23:-164 24:+270 25:-33 26:-211 · fail: added_pnl, added_pf, combined_pf, combined_years_not_worse, added_worst_year · base iex_v18: +1728 / 436 / PF 1.37 · added -305 / 84 / PF 0.79 (worst yr -211, worst day -103) · cooldown to 2027-04-06 |

### withdrawn (1)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 46 | `qqq-noise-pm-vol` | config | quality-config | base | 31 m | — |

## last 20 events

| when (UTC) | candidate | transition | actor | detail |
|---|---|---|---|---|
| 2026-10-08 05:12 | `spy-sqrt-band-1h15` | backtest_passed → shadow | pipeline | shadow_start book=shadow:spy-sqrt-band-1h15 |
| 2026-10-08 05:12 | `spy-sqrt-band` | backtest_passed → shadow | pipeline | shadow_start book=shadow:spy-sqrt-band |
| 2026-10-08 04:41 | `qqq-noise-pm-vol` | ∅ → proposed | human | proposed |
| 2026-10-08 04:41 | `qqq-noise-pm-vol` | proposed → withdrawn | human | withdrawn |
| 2026-10-08 04:22 | `qqq-noise-pm-vol` | ∅ → proposed | human | proposed |
| 2026-10-08 03:40 | `spy-sqrt-band-1h15` | backtesting → backtest_passed | pipeline | gate tag=cand_45 pass=True |
| 2026-10-08 03:40 | `spy-sqrt-band-1h15` | backtesting → backtesting | pipeline | sweep_done tag=cand_45 |
| 2026-10-08 03:35 | `spy-sqrt-band-1h15` | backtesting → backtesting | pipeline | materialized config_version_id=21 |
| 2026-10-08 03:35 | `spy-sqrt-band-1h15` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 03:35 | `spy-sqrt-band` | backtesting → backtest_passed | pipeline | gate tag=cand_44 pass=True |
| 2026-10-08 03:35 | `spy-sqrt-band` | backtesting → backtesting | pipeline | sweep_done tag=cand_44 |
| 2026-10-08 03:30 | `spy-sqrt-band` | backtesting → backtesting | pipeline | materialized config_version_id=20 |
| 2026-10-08 03:30 | `spy-sqrt-band` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 03:19 | `ticker:UBER` | backtest_failed → backtest_failed | pipeline | regate pass=False |
| 2026-10-08 03:19 | `ticker:SHOP` | backtest_failed → backtest_failed | pipeline | regate pass=False |
| 2026-10-08 03:19 | `ticker:MU` | backtest_failed → backtest_failed | pipeline | regate pass=False |
| 2026-10-08 03:19 | `ticker:PLTR` | backtest_failed → backtest_failed | pipeline | regate pass=False |
| 2026-10-08 03:19 | `ticker:COIN` | backtest_failed → backtest_failed | pipeline | regate pass=False |
| 2026-10-08 03:19 | `ticker:NFLX` | backtest_failed → backtest_failed | pipeline | regate pass=False |
| 2026-10-08 03:19 | `ticker:AVGO` | backtest_failed → backtest_failed | pipeline | regate pass=False |
