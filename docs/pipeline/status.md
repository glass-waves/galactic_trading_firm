# research pipeline — status

generated 2026-09-27 19:45 UTC by `scripts/pipeline/pipeline.py report`. do not edit: regenerated nightly. lane: proposed → backtesting → backtest_passed → shadow → shadow_passed → promotion_proposed → promoted (human). failures: backtest_failed / shadow_failed (180 d cooldown), withdrawn (human).

- gate sweeps run at the research sizing (`--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`, IEX cache, 3 bps + $0.005); the per-year floor (no year < −300) is stated at 36 %. live and shadow books size at the blob's fraction; parity replays use no sizing override.
- shadow-book creation: **disabled** (`PIPELINE_SHADOW_BOOKS=0`; while disabled `advance` only prints the books it would create).
- baseline `iex_v18` (promoted row on its own tickers): +1728 / 436 trades / PF 1.37 · 22:+991 23:-3 24:-20 25:+332 26:+429

## promotion proposals (open)

none.

## flags (plumbing, not verdicts)

none.

## candidates by stage

### proposed (0)

none.

### backtesting (0)

none.

### backtest_passed (0)

none.

### shadow (0)

none.

### shadow_passed (0)

none.

### promotion_proposed (0)

none.

### backtest_failed (3)

| id | name | kind | gate | tickers | since | backtest (5y P&L / trades / PF · per year · gate) |
|---:|---|---|---|---|---|---|
| 1 | `bear-bounce-sma50` | config | volume-config | base | 10 h | +2179 / 828 / PF 1.23 · 22:+1422 23:+371 24:-436 25:+61 26:+762 · fail: pf_5y, min_year_pnl · base iex_v18: +1728 / 436 / PF 1.37 · cooldown to 2027-03-26 |
| 2 | `ticker:AMD` | ticker | default-ticker | AMD | 3 m | -18 / 158 / PF 0.99 · 22:+476 23:+132 24:-335 25:+171 26:-463 · fail: pf_5y, years_positive, min_year_pnl, pnl_2026 · cooldown to 2027-03-26 |
| 3 | `ticker:META` | ticker | default-ticker | META | 0 m | -19 / 160 / PF 0.99 · 22:+132 23:-223 24:-76 25:+73 26:+76 · fail: pf_5y, years_positive · cooldown to 2027-03-26 |

## last 20 events

| when (UTC) | candidate | transition | actor | detail |
|---|---|---|---|---|
| 2026-09-27 19:45 | `ticker:META` | backtesting → backtest_failed | pipeline | gate tag=cand_3 pass=False |
| 2026-09-27 19:45 | `ticker:META` | backtesting → backtesting | pipeline | sweep_done tag=cand_3 |
| 2026-09-27 19:42 | `ticker:META` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-27 19:42 | `ticker:AMD` | backtesting → backtest_failed | pipeline | gate tag=cand_2 pass=False |
| 2026-09-27 19:42 | `ticker:AMD` | backtesting → backtesting | pipeline | sweep_done tag=cand_2 |
| 2026-09-27 19:39 | `ticker:AMD` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-27 09:22 | `bear-bounce-sma50` | backtesting → backtest_failed | pipeline | gate tag=cand_1 pass=False |
| 2026-09-27 09:22 | `bear-bounce-sma50` | backtesting → backtesting | pipeline | sweep_done tag=cand_1 |
| 2026-09-27 07:55 | `bear-bounce-sma50` | backtesting → backtesting | pipeline | backtest_resume |
| 2026-09-27 07:55 | `bear-bounce-sma50` | backtest_passed → backtesting | pipeline | backtest_rerun |
| 2026-09-27 07:31 | `bear-bounce-sma50` | backtesting → backtest_passed | pipeline | gate tag=cand_1 pass=True |
| 2026-09-27 07:31 | `bear-bounce-sma50` | backtesting → backtesting | pipeline | sweep_done tag=cand_1 |
| 2026-09-27 06:08 | `ticker:META` | ∅ → proposed | human | proposed |
| 2026-09-27 06:08 | `ticker:AMD` | ∅ → proposed | human | proposed |
| 2026-09-27 06:08 | `bear-bounce-sma50` | backtesting → backtesting | pipeline | backtest_resume |
| 2026-09-27 06:06 | `bear-bounce-sma50` | backtesting → backtesting | pipeline | materialized config_version_id=13 |
| 2026-09-27 06:06 | `bear-bounce-sma50` | proposed → backtesting | pipeline | backtest_start |
| 2026-09-27 06:06 | `bear-bounce-sma50` | ∅ → proposed | human | proposed |
