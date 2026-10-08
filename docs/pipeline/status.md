# research pipeline — status

generated 2026-10-08 01:57 UTC by `scripts/pipeline/pipeline.py report`. do not edit: regenerated nightly. lane: proposed → backtesting → backtest_passed → shadow → shadow_passed → promotion_proposed → promoted (human). failures: backtest_failed / shadow_failed (180 d cooldown), withdrawn (human).

- gate sweeps run at the research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, IEX cache, 3 bps + $0.005); the per-year floor (no year < −300) is stated at 36 %. live and shadow books size at the blob's fraction; parity replays use no sizing override.
- shadow-book creation: **enabled** (`PIPELINE_SHADOW_BOOKS=1`; while disabled `advance` only prints the books it would create).
- baseline `iex_v18` (promoted row on its own tickers): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429

## promotion proposals (open)

none.

## flags (plumbing, not verdicts)

none.

## candidates by stage

### proposed (2)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 44 | `spy-sqrt-band` | config | volume-config | base | 1 h | — |
| 45 | `spy-sqrt-band-1h15` | config | quality-config | base | 1 h | — |

### backtesting (0)

none.

### backtest_passed (0)

none.

### shadow (4)

| id | name | kind | gate | book | since | trial | backtest |
|---:|---|---|---|---|---|---|---|
| 4 | `vpin-0.26` | config | quality-config | shadow:vpin-0.26 | 7 d | 6 sessions / 0 trades / +0 · needs 14 more sessions and 15 more trades (or 54 sessions to the time limit) | +1646 / 324 / PF 1.48 · 22:+820 23:+42 24:+105 25:+210 26:+469 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |
| 6 | `stress-s15-core` | config | additive-config | shadow:stress-s15-core | 7 d | 6 sessions / 0 trades / +0 · needs 14 more sessions and 15 more trades (or 54 sessions to the time limit) | +2003 / 459 / PF 1.41 · 22:+1226 23:-3 24:-24 25:+376 26:+429 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 · added +274 / 23 / PF 3.32 (worst yr -4, worst day -44) |
| 28 | `size-vpin26-x1.25` | config | sizing-config | shadow:size-vpin26-x1.25 | 4 d | 3 sessions / 0 trades / +0 · needs 17 more sessions and 15 more trades (or 57 sessions to the time limit) | +2155 / 436 / PF 1.40 · 22:+1194 23:+46 24:+10 25:+410 26:+496 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 · added +0 / 0 / PF 0.00 (worst yr +0, worst day +0) |
| 29 | `thrust-1h15` | config | quality-config | shadow:thrust-1h15 | 4 d | 3 sessions / 0 trades / +0 · needs 17 more sessions and 15 more trades (or 57 sessions to the time limit) | +1714 / 278 / PF 1.63 · 22:+644 23:+55 24:-56 25:+396 26:+676 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |

### shadow_passed (0)

none.

### promotion_proposed (0)

none.

### backtest_failed (39)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 1 | `bear-bounce-sma50` | config | volume-config | base | 10 d | +2179 / 828 / PF 1.23 · 22:+1422 23:+371 24:-436 25:+61 26:+762 · fail: pf_5y, min_year_pnl · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-26 |
| 2 | `ticker:AMD` | ticker | default-ticker | AMD | 10 d | -18 / 158 / PF 0.99 · 22:+476 23:+132 24:-335 25:+171 26:-463 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-03-26 |
| 3 | `ticker:META` | ticker | default-ticker | META | 10 d | -19 / 160 / PF 0.99 · 22:+132 23:-223 24:-76 25:+73 26:+76 · fail: pf_5y, years_positive · cooldown to 2027-03-26 |
| 5 | `rs-long-x0.3` | config | volume-config | base | 8 d | +1997 / 1586 / PF 1.11 · 22:+1490 23:+238 24:-197 25:-213 26:+679 · fail: pf_5y, years_positive · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-28 |
| 7 | `trend-day-ride` | config | additive-config | base | 7 d | +2142 / 803 / PF 1.23 · 22:+2003 23:-260 24:-307 25:+461 26:+246 · fail: added_pf, added_worst_year, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +395 / 367 / PF 1.08 (worst yr -287, worst day -273) · cooldown to 2027-03-29 |
| 8 | `ticker:JPM` | ticker | default-ticker | JPM | 1 d | +23 / 51 / PF 1.03 · 22:-100 23:+4 24:+45 25:+292 26:-217 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-04 |
| 9 | `ticker:V` | ticker | default-ticker | V | 1 d | -245 / 67 / PF 0.64 · 22:-107 23:+6 24:-62 25:+44 26:-128 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-04 |
| 10 | `ticker:MA` | ticker | default-ticker | MA | 22 h | -234 / 85 / PF 0.80 · 22:-203 23:+1 24:+8 25:+14 26:-53 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-05 |
| 11 | `ticker:UNH` | ticker | default-ticker | UNH | 22 h | -130 / 119 / PF 0.90 · 22:-68 23:-144 24:-12 25:+118 26:-25 · fail: pf_5y, years_positive, pnl_2026 · cooldown to 2027-04-05 |
| 12 | `ticker:LLY` | ticker | default-ticker | LLY | 2 h | +218 / 147 / PF 1.14 · 22:-105 23:+237 24:+69 25:-90 26:+107 · fail: pf_5y, years_positive · cooldown to 2027-04-05 |
| 13 | `ticker:COST` | ticker | default-ticker | COST | 2 h | -56 / 68 / PF 0.92 · 22:+150 23:+6 24:-123 25:-112 26:+24 · fail: pf_5y, years_positive, trades_5y · cooldown to 2027-04-05 |
| 14 | `ticker:HD` | ticker | default-ticker | HD | 1 h | -568 / 92 / PF 0.52 · 22:-71 23:-64 24:-77 25:-186 26:-170 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 15 | `ticker:XOM` | ticker | default-ticker | XOM | 1 h | +131 / 51 / PF 1.28 · 22:+164 23:+64 24:-10 25:+25 26:-112 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 16 | `ticker:CVX` | ticker | default-ticker | CVX | 1 h | +213 / 56 / PF 1.43 · 22:+29 23:+44 24:-67 25:+68 26:+139 · fail: trades_5y · cooldown to 2027-04-06 |
| 17 | `ticker:ADBE` | ticker | default-ticker | ADBE | 1 h | -70 / 122 / PF 0.96 · 22:-31 23:-204 24:+126 25:-54 26:+93 · fail: pf_5y, years_positive · cooldown to 2027-04-06 |
| 18 | `ticker:CRM` | ticker | default-ticker | CRM | 1 h | -182 / 106 / PF 0.87 · 22:+88 23:-97 24:-83 25:+7 26:-98 · fail: pf_5y, years_positive, pnl_2026 · cooldown to 2027-04-06 |
| 19 | `ticker:ORCL` | ticker | default-ticker | ORCL | 1 h | +44 / 77 / PF 1.04 · 22:+74 23:+11 24:-39 25:+184 26:-187 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 20 | `ticker:QCOM` | ticker | default-ticker | QCOM | 1 h | -534 / 98 / PF 0.66 · 22:+192 23:-26 24:-329 25:-62 26:-310 · fail: pf_5y, years_positive, min_year_pnl, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 21 | `ticker:CAT` | ticker | default-ticker | CAT | 1 h | +267 / 92 / PF 1.26 · 22:+120 23:+35 24:-129 25:+203 26:+37 · fail: pf_5y, trades_5y · cooldown to 2027-04-06 |
| 22 | `ticker:GS` | ticker | default-ticker | GS | 1 h | -564 / 93 / PF 0.57 · 22:-96 23:+45 24:-155 25:+32 26:-390 · fail: pf_5y, years_positive, min_year_pnl, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 23 | `ticker:PG` | ticker | default-ticker | PG | 1 h | -151 / 21 / PF 0.40 · 22:-21 23:+11 24:-32 25:+17 26:-126 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 24 | `ticker:ABBV` | ticker | default-ticker | ABBV | 1 h | -44 / 41 / PF 0.90 · 22:+55 23:-49 24:+17 25:-27 26:-41 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 25 | `ticker:MRK` | ticker | default-ticker | MRK | 58 m | +90 / 25 / PF 1.26 · 22:+90 23:+0 24:+202 25:-189 26:-14 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 26 | `ticker:TXN` | ticker | default-ticker | TXN | 55 m | -95 / 66 / PF 0.87 · 22:-79 23:-19 24:-45 25:+162 26:-114 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 27 | `ticker:AMAT` | ticker | default-ticker | AMAT | 51 m | -326 / 112 / PF 0.82 · 22:-47 23:-253 24:-125 25:-109 26:+208 · fail: pf_5y, years_positive · cooldown to 2027-04-06 |
| 30 | `ticker:QQQ` | ticker | default-ticker | QQQ | 48 m | +53 / 67 / PF 1.10 · 22:+95 23:-5 24:+5 25:+60 26:-103 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 31 | `ticker:SMH` | ticker | default-ticker | SMH | 46 m | -141 / 90 / PF 0.86 · 22:-75 23:-129 24:+132 25:+165 26:-234 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 32 | `ticker:XLK` | ticker | default-ticker | XLK | 42 m | -178 / 38 / PF 0.65 · 22:+41 23:-3 24:-40 25:+16 26:-193 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 33 | `ticker:XLF` | ticker | default-ticker | XLF | 39 m | -46 / 2 / PF 0.26 · 22:-61 23:+0 24:+0 25:+16 26:+0 · fail: pf_5y, years_positive, trades_5y · cooldown to 2027-04-06 |
| 34 | `ticker:IWM` | ticker | default-ticker | IWM | 36 m | -241 / 27 / PF 0.40 · 22:-65 23:-57 24:+51 25:-109 26:-62 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 35 | `ticker:XLE` | ticker | default-ticker | XLE | 32 m | -204 / 43 / PF 0.57 · 22:+44 23:-46 24:-63 25:-41 26:-97 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 36 | `ticker:TSLA` | ticker | default-ticker | TSLA | 28 m | -569 / 223 / PF 0.85 · 22:-379 23:-201 24:-213 25:+424 26:-200 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-04-06 |
| 37 | `ticker:AVGO` | ticker | default-ticker | AVGO | 26 m | -455 / 143 / PF 0.73 · 22:+20 23:-99 24:+185 25:-210 26:-351 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-04-06 |
| 38 | `ticker:NFLX` | ticker | default-ticker | NFLX | 23 m | +24 / 128 / PF 1.01 · 22:-37 23:+75 24:-70 25:+76 26:-20 · fail: pf_5y, years_positive, pnl_2026 · cooldown to 2027-04-06 |
| 39 | `ticker:COIN` | ticker | default-ticker | COIN | 19 m | -1307 / 217 / PF 0.74 · 22:-193 23:-504 24:-142 25:-335 26:-133 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-04-06 |
| 40 | `ticker:PLTR` | ticker | default-ticker | PLTR | 16 m | +204 / 160 / PF 1.06 · 22:-92 23:+365 24:-223 25:+390 26:-235 · fail: pf_5y, years_positive, pnl_2026 · cooldown to 2027-04-06 |
| 41 | `ticker:MU` | ticker | default-ticker | MU | 13 m | -317 / 139 / PF 0.87 · 22:+7 23:-98 24:-173 25:+232 26:-284 · fail: pf_5y, years_positive, pnl_2026 · cooldown to 2027-04-06 |
| 42 | `ticker:SHOP` | ticker | default-ticker | SHOP | 9 m | +622 / 113 / PF 1.30 · 22:+708 23:-445 24:+4 25:+43 26:+312 · fail: pf_5y, min_year_pnl · cooldown to 2027-04-06 |
| 43 | `ticker:UBER` | ticker | default-ticker | UBER | 6 m | -305 / 84 / PF 0.79 · 22:-166 23:-164 24:+270 25:-33 26:-211 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |

## last 20 events

| when (UTC) | candidate | transition | actor | detail |
|---|---|---|---|---|
| 2026-10-08 01:51 | `ticker:UBER` | backtesting → backtest_failed | pipeline | gate tag=cand_43 pass=False |
| 2026-10-08 01:51 | `ticker:UBER` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:47 | `ticker:SHOP` | backtesting → backtest_failed | pipeline | gate tag=cand_42 pass=False |
| 2026-10-08 01:47 | `ticker:SHOP` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:44 | `ticker:MU` | backtesting → backtest_failed | pipeline | gate tag=cand_41 pass=False |
| 2026-10-08 01:44 | `ticker:MU` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:41 | `ticker:PLTR` | backtesting → backtest_failed | pipeline | gate tag=cand_40 pass=False |
| 2026-10-08 01:41 | `ticker:PLTR` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:37 | `ticker:COIN` | backtesting → backtest_failed | pipeline | gate tag=cand_39 pass=False |
| 2026-10-08 01:37 | `ticker:COIN` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:34 | `ticker:NFLX` | backtesting → backtest_failed | pipeline | gate tag=cand_38 pass=False |
| 2026-10-08 01:34 | `ticker:NFLX` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:31 | `ticker:AVGO` | backtesting → backtest_failed | pipeline | gate tag=cand_37 pass=False |
| 2026-10-08 01:31 | `ticker:AVGO` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:28 | `ticker:TSLA` | backtesting → backtest_failed | pipeline | gate tag=cand_36 pass=False |
| 2026-10-08 01:28 | `ticker:TSLA` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:24 | `ticker:XLE` | backtesting → backtest_failed | pipeline | gate tag=cand_35 pass=False |
| 2026-10-08 01:24 | `ticker:XLE` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 01:21 | `ticker:IWM` | backtesting → backtest_failed | pipeline | gate tag=cand_34 pass=False |
| 2026-10-08 01:21 | `ticker:IWM` | proposed → backtesting | pipeline | backtest_start |
