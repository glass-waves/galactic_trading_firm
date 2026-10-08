# research pipeline — status

generated 2026-10-08 00:33 UTC by `scripts/pipeline/pipeline.py report`. do not edit: regenerated nightly. lane: proposed → backtesting → backtest_passed → shadow → shadow_passed → promotion_proposed → promoted (human). failures: backtest_failed / shadow_failed (180 d cooldown), withdrawn (human).

- gate sweeps run at the research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, IEX cache, 3 bps + $0.005); the per-year floor (no year < −300) is stated at 36 %. live and shadow books size at the blob's fraction; parity replays use no sizing override.
- shadow-book creation: **enabled** (`PIPELINE_SHADOW_BOOKS=1`; while disabled `advance` only prints the books it would create).
- baseline `iex_v18` (promoted row on its own tickers): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429

## promotion proposals (open)

none.

## flags (plumbing, not verdicts)

none.

## candidates by stage

### proposed (27)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 17 | `ticker:ADBE` | ticker | default-ticker | ADBE | 5 d | — |
| 18 | `ticker:CRM` | ticker | default-ticker | CRM | 5 d | — |
| 19 | `ticker:ORCL` | ticker | default-ticker | ORCL | 5 d | — |
| 20 | `ticker:QCOM` | ticker | default-ticker | QCOM | 5 d | — |
| 21 | `ticker:CAT` | ticker | default-ticker | CAT | 5 d | — |
| 22 | `ticker:GS` | ticker | default-ticker | GS | 5 d | — |
| 23 | `ticker:PG` | ticker | default-ticker | PG | 5 d | — |
| 24 | `ticker:ABBV` | ticker | default-ticker | ABBV | 5 d | — |
| 25 | `ticker:MRK` | ticker | default-ticker | MRK | 5 d | — |
| 26 | `ticker:TXN` | ticker | default-ticker | TXN | 5 d | — |
| 27 | `ticker:AMAT` | ticker | default-ticker | AMAT | 5 d | — |
| 30 | `ticker:QQQ` | ticker | default-ticker | QQQ | 3 h | — |
| 31 | `ticker:SMH` | ticker | default-ticker | SMH | 3 h | — |
| 32 | `ticker:XLK` | ticker | default-ticker | XLK | 3 h | — |
| 33 | `ticker:XLF` | ticker | default-ticker | XLF | 3 h | — |
| 34 | `ticker:IWM` | ticker | default-ticker | IWM | 3 h | — |
| 35 | `ticker:XLE` | ticker | default-ticker | XLE | 3 h | — |
| 36 | `ticker:TSLA` | ticker | default-ticker | TSLA | 3 h | — |
| 37 | `ticker:AVGO` | ticker | default-ticker | AVGO | 3 h | — |
| 38 | `ticker:NFLX` | ticker | default-ticker | NFLX | 3 h | — |
| 39 | `ticker:COIN` | ticker | default-ticker | COIN | 3 h | — |
| 40 | `ticker:PLTR` | ticker | default-ticker | PLTR | 3 h | — |
| 41 | `ticker:MU` | ticker | default-ticker | MU | 3 h | — |
| 42 | `ticker:SHOP` | ticker | default-ticker | SHOP | 3 h | — |
| 43 | `ticker:UBER` | ticker | default-ticker | UBER | 3 h | — |
| 44 | `spy-sqrt-band` | config | volume-config | base | 4 m | — |
| 45 | `spy-sqrt-band-1h15` | config | quality-config | base | 0 m | — |

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

### backtest_failed (14)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 1 | `bear-bounce-sma50` | config | volume-config | base | 10 d | +2179 / 828 / PF 1.23 · 22:+1422 23:+371 24:-436 25:+61 26:+762 · fail: pf_5y, min_year_pnl · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-26 |
| 2 | `ticker:AMD` | ticker | default-ticker | AMD | 10 d | -18 / 158 / PF 0.99 · 22:+476 23:+132 24:-335 25:+171 26:-463 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-03-26 |
| 3 | `ticker:META` | ticker | default-ticker | META | 10 d | -19 / 160 / PF 0.99 · 22:+132 23:-223 24:-76 25:+73 26:+76 · fail: pf_5y, years_positive · cooldown to 2027-03-26 |
| 5 | `rs-long-x0.3` | config | volume-config | base | 8 d | +1997 / 1586 / PF 1.11 · 22:+1490 23:+238 24:-197 25:-213 26:+679 · fail: pf_5y, years_positive · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-28 |
| 7 | `trend-day-ride` | config | additive-config | base | 7 d | +2142 / 803 / PF 1.23 · 22:+2003 23:-260 24:-307 25:+461 26:+246 · fail: added_pf, added_worst_year, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +395 / 367 / PF 1.08 (worst yr -287, worst day -273) · cooldown to 2027-03-29 |
| 8 | `ticker:JPM` | ticker | default-ticker | JPM | 1 d | +23 / 51 / PF 1.03 · 22:-100 23:+4 24:+45 25:+292 26:-217 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-04 |
| 9 | `ticker:V` | ticker | default-ticker | V | 1 d | -245 / 67 / PF 0.64 · 22:-107 23:+6 24:-62 25:+44 26:-128 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-04 |
| 10 | `ticker:MA` | ticker | default-ticker | MA | 21 h | -234 / 85 / PF 0.80 · 22:-203 23:+1 24:+8 25:+14 26:-53 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-05 |
| 11 | `ticker:UNH` | ticker | default-ticker | UNH | 20 h | -130 / 119 / PF 0.90 · 22:-68 23:-144 24:-12 25:+118 26:-25 · fail: pf_5y, years_positive, pnl_2026 · cooldown to 2027-04-05 |
| 12 | `ticker:LLY` | ticker | default-ticker | LLY | 1 h | +218 / 147 / PF 1.14 · 22:-105 23:+237 24:+69 25:-90 26:+107 · fail: pf_5y, years_positive · cooldown to 2027-04-05 |
| 13 | `ticker:COST` | ticker | default-ticker | COST | 50 m | -56 / 68 / PF 0.92 · 22:+150 23:+6 24:-123 25:-112 26:+24 · fail: pf_5y, years_positive, trades_5y · cooldown to 2027-04-05 |
| 14 | `ticker:HD` | ticker | default-ticker | HD | 27 m | -568 / 92 / PF 0.52 · 22:-71 23:-64 24:-77 25:-186 26:-170 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 15 | `ticker:XOM` | ticker | default-ticker | XOM | 9 m | +131 / 51 / PF 1.28 · 22:+164 23:+64 24:-10 25:+25 26:-112 · fail: pf_5y, years_positive, trades_5y, pnl_2026 · cooldown to 2027-04-06 |
| 16 | `ticker:CVX` | ticker | default-ticker | CVX | 1 m | +213 / 56 / PF 1.43 · 22:+29 23:+44 24:-67 25:+68 26:+139 · fail: trades_5y · cooldown to 2027-04-06 |

## last 20 events

| when (UTC) | candidate | transition | actor | detail |
|---|---|---|---|---|
| 2026-10-08 00:33 | `spy-sqrt-band-1h15` | ∅ → proposed | human | proposed |
| 2026-10-08 00:31 | `ticker:CVX` | backtesting → backtest_failed | pipeline | gate tag=cand_16 pass=False |
| 2026-10-08 00:31 | `ticker:CVX` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 00:29 | `spy-sqrt-band` | ∅ → proposed | human | proposed |
| 2026-10-08 00:24 | `ticker:XOM` | backtesting → backtest_failed | pipeline | gate tag=cand_15 pass=False |
| 2026-10-08 00:24 | `ticker:XOM` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-08 00:05 | `ticker:HD` | backtesting → backtest_failed | pipeline | gate tag=cand_14 pass=False |
| 2026-10-08 00:05 | `ticker:HD` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-07 23:42 | `ticker:COST` | backtesting → backtest_failed | pipeline | gate tag=cand_13 pass=False |
| 2026-10-07 23:42 | `ticker:COST` | proposed → backtesting | pipeline | backtest_start |
| 2026-10-07 23:20 | `ticker:LLY` | backtesting → backtest_failed | pipeline | gate tag=cand_12 pass=False |
| 2026-10-07 23:20 | `ticker:LLY` | backtesting → backtesting | pipeline | backtest_resume |
| 2026-10-07 20:57 | `ticker:UBER` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:SHOP` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:MU` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:PLTR` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:COIN` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:NFLX` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:AVGO` | ∅ → proposed | human | proposed |
| 2026-10-07 20:57 | `ticker:TSLA` | ∅ → proposed | human | proposed |
