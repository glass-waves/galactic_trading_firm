# entry-feature screen brief (round two, 2026-09-12)

repo /home/dylmet/Projects/galactic_trading_firm. read CLAUDE.md §"key architecture concepts" and
docs/paper_trading_plan_2026-09.md §10–§11 for context. OFFLINE analysis only: python 3.14 STANDARD
LIBRARY (no pandas/numpy). do not touch postgres, crates/, scripts/, migrations/. write scripts and
outputs under /tmp/claude-1000/-home-dylmet-Projects-galactic-trading-firm/48685858-bad9-4000-a57f-b038fffcc726/scratchpad/screen/ .

## goal

the promoted strategy (v16) is a SHORT-ONLY morning book on AMZN/AAPL/NVDA/MSFT with an honest
five-year profit factor of 1.12. the user's bar is PF > 1.3. the entry signal (a blend of 1m/5m/1h
momentum indicator scores) does not see the best shorts (their 5-minute score is bullish just before
the drop). we now have 28 candidate entry features computed on every morning bar. the question:
**which features, alone or as a hard condition added to v16's windows, separate profitable short
entries from the rest — consistently across 2022–2026?**

## files

### per-bar features: data/scr16_<year>_ticks.csv  (years 2022..2026; only bars 09:30–11:29 ET)
columns: date,ticker,ts,open,high,low,close,volume,vwap,composite,s1m,s5m,s1h,position,unrealized_pct,hold_min,event,entry_reason,blocked_by,near_miss,indicators
- `indicators` is a JSON object (csv-quoted) of every indicator instance score and `{id}.{meta}`
  value on that bar, e.g. `cs_engulfing_5m` (−1/0/+1), `pdl_5m` and `pdl_5m.dist_low_pct`,
  `or15_1m`, `gap_5m`, `cross_1m.index_session_ret` (%), `cross_1m.peers_red_frac`,
  `ofi_1m`, `vpin_1m`, `rvol_1m`, `cal_fomc` (−1 on FOMC decision days). null = not computable yet.
- the promoted config's own indicators are there too (rsi_7_1min, macd_5min, candle_5min …).
- `event == "open"` marks bars where v16 actually entered (fill at next bar open); `position ==
  "short"` marks bars already in a position. ~250 MB per year — stream with csv.reader, parse the
  json column with json.loads only after cheap filters.

### per-bar outcome labels: data/labels/bars_<year>.csv  (built by the previous round; column list below)
one row per 09:30–11:29 bar per ticker-day with the value of a hypothetical SHORT entered at the
next bar's open and managed by the live exit stack (hard stop 2.5 %, max hold 90/75/120, flat
11:55, 3 bps + $0.005 adverse each way): column `opp_pnl_pct` (signed %, positive = the short
made money), plus `opp_full_pct`, `ret_1155`, `mfe90`, `mae90`, category columns. read
scripts/analysis/missed/common.py / label_bars.py for the exact column list and join keys
(date, ticker, ts). NOTE these labels use the v15 exit stack (75-min losing limit); v16 uses 40 min
+ breakeven 0.5 %. for the screen this is fine (ranking features, not absolute P&L); the real
replay is the final arbiter.

### earnings dates: research/entries/data/earnings_<TICKER>.txt — one YYYY-MM-DD per line, the
8-K "results of operations" filing date (these companies report after the close, so the reaction
session is the NEXT trading day). FOMC decision days are in the `cal_fomc` indicator.

### raw bars: data/bars/<TICKER>.csv (epoch,o,h,l,c,v; SPY.csv and QQQ.csv too) if you need paths.

## v16 entry windows (both must hold on the bar; fill next bar open; direction short)
- `5m thrust short`: composite ≤ −0.35, s5m ≤ −0.50, s5m ≤ s1m − 0.10 and s5m ≤ s1h − 0.10, s1h ≤ 0.0
- `strong core short`: composite ≤ −0.35, s5m ≤ −0.40, s1h ≤ −0.40
one position per ticker, 30 s cooldown, entries until 11:30 ET.

## required analyses (per year always; pooled is secondary)

1. **feature-alone screen.** for every candidate feature (each candle pattern firing bearish;
   pdl below prior low / distance buckets; or15/or30 below range; gap buckets (gap up vs down);
   cross index_session_ret buckets, index_ret_5m/15m buckets, peers_red_frac ∈ {0, 1/3, 2/3, 1};
   ofi/vpin/rvol quintiles; FOMC day; earnings reaction day and day-before; time-of-day 10-min
   buckets as a reference): count of bars and of *episodes* (runs of consecutive firing bars per
   ticker-day), mean/median opp_pnl_pct, hit rate, share in the year's top-5 % of opp_pnl, and
   lift vs the all-bar base rate. rank by consistency: how many of the 5 years the feature's mean
   opp_pnl beats the base rate AND the pooled mean > 0.

2. **conditional on v16's windows.** on the bars where a v16 window fires (re-evaluate the
   conditions above from composite/s1m/s5m/s1h — do not rely on `event`, which is throttled by
   position/cooldown), split by each feature and report the same stats: does requiring the
   feature raise the mean and hit rate of the trades v16 would take, and how many trades survive?
   a feature that keeps ≥ 40 % of the trades and raises mean opp_pnl in ≥ 4 of 5 years is a
   candidate hard condition.

3. **feature as a standalone trigger.** for the top features from (1), simulate them as a short
   entry window on their own (one position per ticker, 30 s cooldown, take the first firing bar,
   nothing until the simulated exit; use the label's exit model), report per-year P&L at $3,600
   per position, trade count, win rate, PF — versus v16's own windows simulated the same way
   (calibration: your v16 simulation should be within ~10 % of these real numbers at 36 %
   sizing: 2022 +1,380 / 2023 −177 / 2024 +229 / 2025 +99 / 2026 +153, 1,179 trades).

4. **pairs.** for the top 5 features, test each pair as a joint condition (both firing / both
   as conditions on the window). report only pairs that beat both parents in ≥ 4 of 5 years.

5. **what the bearish patterns actually catch.** for the candle patterns: mean opp_pnl when the
   pattern fires with the 5m score already ≤ −0.4 vs when it fires with the 5m score > 0 (i.e.
   is the pattern an early signal or just a confirmation of what the scores already say?).

## report
markdown at .../scratchpad/screen/report.md, returned in full as your final message. lead with a
ranked table of the ≤ 8 features worth a real-replay test, each with per-year mean opp_pnl,
episodes/yr, and what to test (standalone window / added condition / both). then the pair
results, then the negative results (features that look good pooled but fail per-year). state what
could not be determined. be explicit about episode counts: 40 consecutive firing bars on one day
is one opportunity, not 40. scripts alongside the report so it can be re-run.
