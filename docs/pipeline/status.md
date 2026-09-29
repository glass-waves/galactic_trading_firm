# research pipeline — status

generated 2026-09-29 03:40 UTC by `scripts/pipeline/pipeline.py report`. do not edit: regenerated nightly. lane: proposed → backtesting → backtest_passed → shadow → shadow_passed → promotion_proposed → promoted (human). failures: backtest_failed / shadow_failed (180 d cooldown), withdrawn (human).

- gate sweeps run at the research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, IEX cache, 3 bps + $0.005); the per-year floor (no year < −300) is stated at 36 %. live and shadow books size at the blob's fraction; parity replays use no sizing override.
- shadow-book creation: **disabled** (`PIPELINE_SHADOW_BOOKS=0`; while disabled `advance` only prints the books it would create).
- baseline `iex_v18` (promoted row on its own tickers): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429

## promotion proposals (open)

none.

## flags (plumbing, not verdicts)

none.

## candidates by stage

### proposed (2)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 6 | `stress-s15-core` | config | quality-config | base | 1 d | — |
| 7 | `trend-day-ride` | config | quality-config | base | 1 d | — |

### backtesting (0)

none.

### backtest_passed (1)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 4 | `vpin-0.26` | config | quality-config | base | 5 m | +1646 / 324 / PF 1.48 · 22:+820 23:+42 24:+105 25:+210 26:+469 · **pass** · base iex_v18: +1728 / 436 / PF 1.37 |

### shadow (0)

none.

### shadow_passed (0)

none.

### promotion_proposed (0)

none.

### backtest_failed (4)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 1 | `bear-bounce-sma50` | config | volume-config | base | 1 d | +2179 / 828 / PF 1.23 · 22:+1422 23:+371 24:-436 25:+61 26:+762 · fail: pf_5y, min_year_pnl · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-26 |
| 2 | `ticker:AMD` | ticker | default-ticker | AMD | 1 d | -18 / 158 / PF 0.99 · 22:+476 23:+132 24:-335 25:+171 26:-463 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-03-26 |
| 3 | `ticker:META` | ticker | default-ticker | META | 1 d | -19 / 160 / PF 0.99 · 22:+132 23:-223 24:-76 25:+73 26:+76 · fail: pf_5y, years_positive · cooldown to 2027-03-26 |
| 5 | `rs-long-x0.3` | config | volume-config | base | 0 m | +1997 / 1586 / PF 1.11 · 22:+1490 23:+238 24:-197 25:-213 26:+679 · fail: pf_5y, years_positive · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-28 |

## last 20 events

| when (UTC) | candidate | transition | actor | detail |
|---|---|---|---|---|
| 2026-09-29 03:40 | `rs-long-x0.3` | backtesting → backtest_failed | pipeline | gate tag=cand_5 pass=False |
| 2026-09-29 03:40 | `rs-long-x0.3` | backtesting → backtesting | pipeline | sweep_done tag=cand_5 |
| 2026-09-29 03:35 | `rs-long-x0.3` | backtesting → backtesting | pipeline | materialized config_version_id=15 |
| 2026-09-29 03:35 | `rs-long-x0.3` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-29 03:35 | `vpin-0.26` | backtesting → backtest_passed | pipeline | gate tag=cand_4 pass=True |
| 2026-09-29 03:35 | `vpin-0.26` | backtesting → backtesting | pipeline | sweep_done tag=cand_4 |
| 2026-09-29 03:30 | `vpin-0.26` | backtesting → backtesting | pipeline | materialized config_version_id=14 |
| 2026-09-29 03:30 | `vpin-0.26` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-27 22:18 | `trend-day-ride` | ∅ → proposed | human | proposed |
| 2026-09-27 19:46 | `stress-s15-core` | ∅ → proposed | human | proposed |
| 2026-09-27 19:46 | `rs-long-x0.3` | ∅ → proposed | human | proposed |
| 2026-09-27 19:46 | `vpin-0.26` | ∅ → proposed | human | proposed |
| 2026-09-27 19:45 | `ticker:META` | backtesting → backtest_failed | pipeline | gate tag=cand_3 pass=False |
| 2026-09-27 19:45 | `ticker:META` | backtesting → backtesting | pipeline | sweep_done tag=cand_3 |
| 2026-09-27 19:42 | `ticker:META` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-27 19:42 | `ticker:AMD` | backtesting → backtest_failed | pipeline | gate tag=cand_2 pass=False |
| 2026-09-27 19:42 | `ticker:AMD` | backtesting → backtesting | pipeline | sweep_done tag=cand_2 |
| 2026-09-27 19:39 | `ticker:AMD` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-27 09:22 | `bear-bounce-sma50` | backtesting → backtest_failed | pipeline | gate tag=cand_1 pass=False |
| 2026-09-27 09:22 | `bear-bounce-sma50` | backtesting → backtesting | pipeline | sweep_done tag=cand_1 |
