=== comprehensive strategy validation ===
capital: $10000  lookback: 3  costs: --slippage-bps 2.0 --half-spread 0.005
overrides: --entry-threshold 0.58 --no-new-entries-after 15:30

--- section 1: in-sample baseline (99 dates) ---
  2025-03-10  +$0.00         0 trades
  2025-03-12  +$0.00         0 trades
  2025-03-17  +$0.00         0 trades
  2025-03-19  +$136.29       1 trades
  2025-03-24  +$223.18       1 trades
  2025-03-26  +$0.00         0 trades
  2025-03-31  -$49.45        1 trades
  2025-04-02  +$0.00         0 trades
  2025-04-07  +$2002.68     41 trades
  2025-04-09  +$3279.43     49 trades
  2025-04-14  +$178.45       2 trades
  2025-04-16  +$0.00         0 trades
  2025-04-21  -$794.79       1 trades
  2025-04-23  +$0.00         0 trades
  2025-04-28  +$0.00         0 trades
  2025-04-30  +$0.00         0 trades
  2025-05-05  +$0.00         0 trades
  2025-05-07  +$0.00         0 trades
  2025-05-12  +$0.00         0 trades
  2025-05-14  +$0.00         0 trades
  2025-05-19  +$773.93       2 trades
  2025-05-21  +$0.00         0 trades
  2025-05-28  +$0.00         0 trades
  2025-06-02  -$40.36        1 trades
  2025-06-04  +$0.00         0 trades
  2025-06-09  +$0.00         0 trades
  2025-06-11  +$0.00         0 trades
  2025-06-16  +$0.00         0 trades
  2025-06-18  +$0.00         0 trades
  2025-06-23  -$462.60      11 trades
  2025-06-25  +$135.35       1 trades
  2025-06-30  -$379.03      10 trades
  2025-07-02  +$20.38        1 trades
  2025-07-07  -$822.26       1 trades
  2025-07-09  +$184.51       1 trades
  2025-07-14  +$0.00         0 trades
  2025-07-16  +$0.00         0 trades
  2025-07-21  +$0.00         0 trades
  2025-07-23  +$0.00         0 trades
  2025-07-28  +$0.00         0 trades
  2025-07-30  +$0.00         0 trades
  2025-08-04  +$936.17       2 trades
  2025-08-06  +$1572.44      1 trades
  2025-08-11  +$0.00         0 trades
  2025-08-13  +$0.00         0 trades
  2025-08-18  +$135.71       1 trades
  2025-08-20  +$0.00         0 trades
  2025-08-25  +$0.00         0 trades
  2025-08-27  +$0.00         0 trades
  2025-09-03  +$0.00         0 trades
  2025-09-08  +$533.38       1 trades
  2025-09-10  +$0.00         0 trades
  2025-09-15  +$114.48       1 trades
  2025-09-17  +$0.00         0 trades
  2025-09-22  +$1844.44     21 trades
  2025-09-24  +$0.00         0 trades
  2025-09-29  +$314.65       1 trades
  2025-10-01  +$0.00         0 trades
  2025-10-06  +$568.96       3 trades
  2025-10-08  +$0.00         0 trades
  2025-10-13  -$212.58       1 trades
  2025-10-15  +$0.00         0 trades
  2025-10-20  +$792.82       1 trades
  2025-10-22  +$0.00         0 trades
  2025-10-27  +$0.00         0 trades
  2025-10-29  +$166.63       1 trades
  2025-11-03  +$0.00         0 trades
  2025-11-05  +$0.00         0 trades
  2025-11-10  -$1206.55      2 trades
  2025-11-12  +$0.00         0 trades
  2025-11-17  +$0.00         0 trades
  2025-11-19  +$148.80       2 trades
  2025-11-24  +$0.00         0 trades
  2025-11-26  +$0.00         0 trades
  2025-12-01  +$443.56       1 trades
  2025-12-03  +$0.00         0 trades
  2025-12-08  +$1118.36      7 trades
  2025-12-10  +$0.00         0 trades
  2025-12-15  +$0.00         0 trades
  2025-12-17  +$0.00         0 trades
  2025-12-22  +$0.00         0 trades
  2025-12-24  +$0.00         0 trades
  2025-12-29  +$0.00         0 trades
  2025-12-31  +$0.00         0 trades
  2026-01-05  +$0.00         0 trades
  2026-01-07  +$0.00         0 trades
  2026-01-12  -$359.43       1 trades
  2026-01-14  +$0.00         0 trades
  2026-01-21  +$58.77       35 trades
  2026-01-26  +$316.81       1 trades
  2026-01-28  +$0.00         0 trades
  2026-02-02  +$278.95       1 trades
  2026-02-04  +$1018.82      1 trades
  2026-02-09  +$2468.43      2 trades
  2026-02-11  +$0.00         0 trades
  2026-02-18  +$0.00         0 trades
  2026-02-23  -$725.01       1 trades
  2026-02-25  +$796.92       1 trades
  2026-03-02  +$2103.15      6 trades

  in-sample: P&L +$17614.39  PF 4.48  win 29%  (99 days, 219 trades)

--- section 2: hold-out out-of-sample test (40 dates, never used in optimization) ---
  2025-03-13  +$0.00         0 trades
  2025-03-20  +$0.00         0 trades
  2025-03-27  +$0.00         0 trades
  2025-04-03  +$0.00         0 trades
  2025-04-10  -$278.14       5 trades
  2025-04-17  +$0.00         0 trades
  2025-04-24  +$2123.21      2 trades
  2025-05-01  -$1227.56      1 trades
  2025-05-08  +$0.00         0 trades
  2025-05-15  +$0.00         0 trades
  2025-05-22  +$0.00         0 trades
  2025-05-29  +$0.00         0 trades
  2025-06-05  +$0.00         0 trades
  2025-06-12  +$0.00         0 trades
  2025-06-19  +$0.00         0 trades
  2025-06-26  +$0.00         0 trades
  2025-07-10  +$0.00         0 trades
  2025-07-17  +$0.00         0 trades
  2025-07-24  +$0.00         0 trades
  2025-07-31  +$0.00         0 trades
  2025-08-07  +$0.00         0 trades
  2025-08-14  +$0.00         0 trades
  2025-08-21  +$0.00         0 trades
  2025-08-28  +$0.00         0 trades
  2025-09-04  +$0.00         0 trades
  2025-09-11  +$0.00         0 trades
  2025-09-18  +$0.00         0 trades
  2025-09-25  +$0.00         0 trades
  2025-10-02  +$0.00         0 trades
  2025-10-09  +$0.00         0 trades
  2025-10-16  +$0.00         0 trades
  2025-10-23  +$0.00         0 trades
  2025-10-30  +$0.00         0 trades
  2025-11-06  +$0.00         0 trades
  2025-11-13  +$0.00         0 trades
  2025-11-20  +$0.00         0 trades
  2025-12-04  +$0.00         0 trades
  2025-12-11  +$0.00         0 trades
  2025-12-18  +$844.70       1 trades
  2026-01-08  +$0.00         0 trades

  in-sample (99-day):     P&L +$17614.39  PF 4.48  win 29%
  out-of-sample (40-day): P&L +$1462.21  PF 1.97  win 5%
  OOS/IS per-day P&L ratio: 20%

--- section 3: slippage sensitivity sweep (20-day × 6 levels) ---
   1 bps:  +$9504.96  (20 days)
   2 bps:  +$8366.09  (20 days)
   4 bps:  +$6221.16  (20 days)
   6 bps:  +$3593.94  (20 days)
   8 bps:  +$2089.42  (20 days)
  10 bps:  -$456.07  (20 days)
  breakeven: 9.6 bps

--- section 4: monte carlo daily P&L shuffle (1000 iterations) ---
  P&L percentiles:       5th: +$17614.39  50th: +$17614.39  95th: +$17614.39
  max drawdown pctiles:  5th: $1206.55  50th: $1619.88  95th: $2729.47

--- section 5: walk-forward stability ---
  windows: 23 (train=20, test=5, stride=5)
  OOS profitable windows: 15/23 (65%)
  walk-forward efficiency (WFE): 26%

--- section 6: per-ticker analysis ---
  ticker     total P&L   trades    win days      PF  (IS P&L / OOS P&L)
  AAPL       +$3751.81       27     9/139      3.0  (+$4029.95 / -$278.14)
  MSFT       +$3895.80       24     9/139      2.5  (+$3133.41 / +$762.39)
  NVDA       +$7887.35      102    18/139      4.5  (+$7887.35 / +$0.00)
  QQQ        +$3485.66       54     4/139    138.3  (+$2507.70 / +$977.96)
  SPY          +$55.96       21     1/139      1.2  (+$55.96 / +$0.00)

=== validation report card ===
  [PASS] hold-out OOS test        P&L +$1462.21  PF 1.97  (20% per-day ratio)
  [PASS] slippage sensitivity     profitable at 6 bps (breakeven: 9.6 bps)
  [PASS] monte carlo shuffle      5th pctl: +$17614.39
  [FAIL] walk-forward stability   65% OOS profitable, WFE 26%
  [PASS] per-ticker analysis      all tickers PF >= 0.8
  ---
  OVERALL: FAIL (4/5)

