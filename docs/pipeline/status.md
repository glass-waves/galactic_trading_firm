# research pipeline — status

generated 2026-10-02 21:38 UTC by `scripts/pipeline/pipeline.py report`. do not edit: regenerated nightly. lane: proposed → backtesting → backtest_passed → shadow → shadow_passed → promotion_proposed → promoted (human). failures: backtest_failed / shadow_failed (180 d cooldown), withdrawn (human).

- gate sweeps run at the research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, IEX cache, 3 bps + $0.005); the per-year floor (no year < −300) is stated at 36 %. live and shadow books size at the blob's fraction; parity replays use no sizing override.
- shadow-book creation: **disabled** (`PIPELINE_SHADOW_BOOKS=0`; while disabled `advance` only prints the books it would create).
- baseline `iex_v18` (promoted row on its own tickers): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429

## promotion proposals (open)

none.

## flags (plumbing, not verdicts)

none.

## candidates by stage

### proposed (21)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 8 | `ticker:JPM` | ticker | default-ticker | JPM | 17 m | — |
| 9 | `ticker:V` | ticker | default-ticker | V | 17 m | — |
| 10 | `ticker:MA` | ticker | default-ticker | MA | 17 m | — |
| 11 | `ticker:UNH` | ticker | default-ticker | UNH | 17 m | — |
| 12 | `ticker:LLY` | ticker | default-ticker | LLY | 17 m | — |
| 13 | `ticker:COST` | ticker | default-ticker | COST | 17 m | — |
| 14 | `ticker:HD` | ticker | default-ticker | HD | 17 m | — |
| 15 | `ticker:XOM` | ticker | default-ticker | XOM | 17 m | — |
| 16 | `ticker:CVX` | ticker | default-ticker | CVX | 17 m | — |
| 17 | `ticker:ADBE` | ticker | default-ticker | ADBE | 17 m | — |
| 18 | `ticker:CRM` | ticker | default-ticker | CRM | 17 m | — |
| 19 | `ticker:ORCL` | ticker | default-ticker | ORCL | 17 m | — |
| 20 | `ticker:QCOM` | ticker | default-ticker | QCOM | 17 m | — |
| 21 | `ticker:CAT` | ticker | default-ticker | CAT | 17 m | — |
| 22 | `ticker:GS` | ticker | default-ticker | GS | 17 m | — |
| 23 | `ticker:PG` | ticker | default-ticker | PG | 17 m | — |
| 24 | `ticker:ABBV` | ticker | default-ticker | ABBV | 17 m | — |
| 25 | `ticker:MRK` | ticker | default-ticker | MRK | 17 m | — |
| 26 | `ticker:TXN` | ticker | default-ticker | TXN | 17 m | — |
| 27 | `ticker:AMAT` | ticker | default-ticker | AMAT | 17 m | — |
| 28 | `size-vpin26-x1.25` | config | sizing-config | base | 1 m | — |

### backtesting (0)

none.

### backtest_passed (0)

none.

### shadow (2)

| id | name | kind | gate | book | since | trial | backtest |
|---:|---|---|---|---|---|---|---|
| 4 | `vpin-0.26` | config | quality-config | shadow:vpin-0.26 | 2 d | 3 sessions / 0 trades / +0 · needs 17 more sessions and 15 more trades (or 57 sessions to the time limit) | +1646 / 324 / PF 1.48 · 22:+820 23:+42 24:+105 25:+210 26:+469 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |
| 6 | `stress-s15-core` | config | additive-config | shadow:stress-s15-core | 2 d | 3 sessions / 0 trades / +0 · needs 17 more sessions and 15 more trades (or 57 sessions to the time limit) | +2003 / 459 / PF 1.41 · 22:+1226 23:-3 24:-24 25:+376 26:+429 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 · added +274 / 23 / PF 3.32 (worst yr -4, worst day -44) |

### shadow_passed (0)

none.

### promotion_proposed (0)

none.

### backtest_failed (5)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 1 | `bear-bounce-sma50` | config | volume-config | base | 5 d | +2179 / 828 / PF 1.23 · 22:+1422 23:+371 24:-436 25:+61 26:+762 · fail: pf_5y, min_year_pnl · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-26 |
| 2 | `ticker:AMD` | ticker | default-ticker | AMD | 5 d | -18 / 158 / PF 0.99 · 22:+476 23:+132 24:-335 25:+171 26:-463 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-03-26 |
| 3 | `ticker:META` | ticker | default-ticker | META | 5 d | -19 / 160 / PF 0.99 · 22:+132 23:-223 24:-76 25:+73 26:+76 · fail: pf_5y, years_positive · cooldown to 2027-03-26 |
| 5 | `rs-long-x0.3` | config | volume-config | base | 3 d | +1997 / 1586 / PF 1.11 · 22:+1490 23:+238 24:-197 25:-213 26:+679 · fail: pf_5y, years_positive · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-28 |
| 7 | `trend-day-ride` | config | additive-config | base | 2 d | +2142 / 803 / PF 1.23 · 22:+2003 23:-260 24:-307 25:+461 26:+246 · fail: added_pf, added_worst_year, combined_years_not_worse · base iex_v18: +1728 / 436 / PF 1.37 · added +395 / 367 / PF 1.08 (worst yr -287, worst day -273) · cooldown to 2027-03-29 |

## last 20 events

| when (UTC) | candidate | transition | actor | detail |
|---|---|---|---|---|
| 2026-10-02 21:37 | `size-vpin26-x1.25` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:AMAT` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:TXN` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:MRK` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:ABBV` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:PG` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:GS` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:CAT` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:QCOM` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:ORCL` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:CRM` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:ADBE` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:CVX` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:XOM` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:HD` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:COST` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:LLY` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:UNH` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:MA` | ∅ → proposed | human | proposed |
| 2026-10-02 21:20 | `ticker:V` | ∅ → proposed | human | proposed |
