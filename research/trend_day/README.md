# trend-day-ride

*2026-09-27. One pipeline candidate patch, `trend_day_v1.patch.json` (`make_patch.py` builds it),
built from `research/crash_days/2026-09-27_crash_days_hindsight.md` (SPY down >= 1% from open
fires ~4x/year, 60% of those close < -1%, session low after 14:00 on ~90% of tail days, close
near the low — trend days, not mean-reverting) and the failure mode of
`research/vwap_short/2026-09-27_vwap_anchored_short.md` (the VWAP-retest entry never fires on the
days that matter). This candidate skips the retest: once SPY is already down >= 1% on the day, it
shorts a name that is below its own session VWAP and underperforming SPY since the prior close,
and holds to the close instead of exiting on any bounce.

Base = v18 unchanged (config_versions row 12): both promoted short windows disabled and re-added
as `_am` copies with their original 11:30/11:55 clocks (verified byte-for-byte against the live
blob). New window `window_trend_day_short`, priority 30, short, 10:15-14:00 ET:
1. `cross_1m.index_session_ret` <= -1.0 (SPY already down >= 1% from today's open)
2. `vwap_1m.dist_pct` <= -0.2 (the name is below its session VWAP; `vwap_1m` = `vwap_distance` on
   OneMinute, weight 0, added here — not in row 12)
3. `pdl_5m.rel_close_pct` <= -0.5 (the name underperforming SPY since the prior close; `pdl_5m` =
   `prior_day_levels` on FiveMinute, weight 0, `scale_pct` 0.005, added here — not in row 12)
4. `composite_max` 0.0 (non-positive composite)

`exit_overrides`: `force_exit_by` 15:55, `max_hold_ms` 21,600,000 (6h), `score_exit_threshold` -10
(off) — rides to the close with only the promoted hard stop (2.5%) and breakeven monitor as
guards, same as every other window in row 12. One entry per name per day: single position per
ticker plus the hold to close.

## Verification (`./target/release/backtest --date D --lookback-days 8 --capital 10000
--slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars_iex --cross-index
SPY --sizing-fraction 0.36 --max-position-pct 0.36 --patch-json research/trend_day/trend_day_v1.patch.json`)

**2025-04-08 (SPY -4.9%)** — window fires twice, both after 10:15, `entry_reason` "window:trend
day short", both exit `SessionClose` at 15:55 ET:

| ticker | dir | entry (ET) | exit (ET) | entry px | exit px | size | pnl |
|---|---|---|---|---|---|---|---|
| AMZN | Short | 11:34 | 15:56 | 178.17 | 170.39 | 20 | +155.71 |
| AAPL | Short | 11:20 | 15:56 | 184.70 | 172.02 | 19 | +241.07 |

NVDA and MSFT: 0 trades (no fire that day). Total: +396.78 on 2 trades.

**2025-01-17 (flat day)** — patched run: 0 trades on all four tickers. Unpatched (promoted row
12) run on the same date: 0 trades on all four tickers. Identical (both empty) — only the `_am`
windows could fire and did not that day.

## Proposal

`python3 scripts/pipeline/pipeline.py propose --name trend-day-ride --kind config --patch
research/trend_day/trend_day_v1.patch.json --gate quality-config --notes "trend-day ride: SPY <=
-1% from open, 10:15-14:00, short a name below VWAP and underperforming SPY, hold to 15:55; from
the crash-day hindsight study"`. The nightly `pipeline.timer` runs the 5-year backtest gate and
shadow trial; this candidate is not swept here.
