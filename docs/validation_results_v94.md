=== comprehensive strategy validation ===
capital: $10000  lookback: 3  costs: --slippage-bps 2.0 --half-spread 0.005

--- section 1: in-sample baseline (99 dates) ---
  2025-03-10  +$0.00         0 trades
  2025-03-12  +$0.00         0 trades
  2025-03-17  +$0.00         0 trades
  2025-03-19  +$143.80       1 trades
  2025-03-24  +$220.49       1 trades
  2025-03-26  +$0.00         0 trades
  2025-03-31  +$0.00         0 trades
  2025-04-02  +$0.00         0 trades
  2025-04-07  +$1273.29      3 trades
  2025-04-09  -$436.93       1 trades
  2025-04-14  +$0.00         0 trades
  2025-04-16  +$0.00         0 trades
  2025-04-21  -$794.79       1 trades
  2025-04-23  +$0.00         0 trades
  2025-04-28  +$0.00         0 trades
  2025-04-30  +$0.00         0 trades
  2025-05-05  +$0.00         0 trades
  2025-05-07  +$0.00         0 trades
  2025-05-12  +$0.00         0 trades
  2025-05-14  +$0.00         0 trades
  2025-05-19  +$773.71       2 trades
  2025-05-21  +$0.00         0 trades
  2025-05-28  +$0.00         0 trades
  2025-06-02  -$40.36        1 trades
  2025-06-04  +$0.00         0 trades
  2025-06-09  +$0.00         0 trades
  2025-06-11  +$0.00         0 trades
  2025-06-16  +$0.00         0 trades
  2025-06-18  +$0.00         0 trades
  2025-06-23  +$0.00         0 trades
  2025-06-25  +$137.04       1 trades
  2025-06-30  +$0.00         0 trades
  2025-07-02  +$18.52        1 trades
  2025-07-07  -$822.26       1 trades
  2025-07-09  +$184.51       1 trades
  2025-07-14  +$0.00         0 trades
  2025-07-16  +$0.00         0 trades
  2025-07-21  +$0.00         0 trades
  2025-07-23  +$0.00         0 trades
  2025-07-28  +$0.00         0 trades
  2025-07-30  +$0.00         0 trades
  2025-08-04  +$949.45       2 trades
  2025-08-06  +$1572.44      1 trades
  2025-08-11  +$0.00         0 trades
  2025-08-13  +$0.00         0 trades
  2025-08-18  +$135.71       1 trades
  2025-08-20  +$0.00         0 trades
  2025-08-25  +$0.00         0 trades
  2025-08-27  +$0.00         0 trades
  2025-09-03  +$0.00         0 trades
  2025-09-08  +$543.71       1 trades
  2025-09-10  +$0.00         0 trades
  2025-09-15  +$0.00         0 trades
  2025-09-17  +$0.00         0 trades
  2025-09-22  +$1569.09      1 trades
  2025-09-24  +$0.00         0 trades
  2025-09-29  +$316.55       1 trades
  2025-10-01  +$0.00         0 trades
  2025-10-06  +$547.95       1 trades
  2025-10-08  +$0.00         0 trades
  2025-10-13  -$212.58       1 trades
  2025-10-15  +$0.00         0 trades
  2025-10-20  +$841.43       1 trades
  2025-10-22  +$0.00         0 trades
  2025-10-27  +$0.00         0 trades
  2025-10-29  +$148.51       1 trades
  2025-11-03  +$0.00         0 trades
  2025-11-05  +$0.00         0 trades
  2025-11-10  -$1206.55      2 trades
  2025-11-12  +$0.00         0 trades
  2025-11-17  +$0.00         0 trades
  2025-11-19  +$149.02       2 trades
  2025-11-24  +$0.00         0 trades
  2025-11-26  +$0.00         0 trades
  2025-12-01  +$443.56       1 trades
  2025-12-03  +$0.00         0 trades
  2025-12-08  +$503.91       1 trades
  2025-12-10  +$0.00         0 trades
  2025-12-15  +$0.00         0 trades
  2025-12-17  +$0.00         0 trades
  2025-12-22  +$0.00         0 trades
  2025-12-24  +$0.00         0 trades
  2025-12-29  +$0.00         0 trades
  2025-12-31  +$0.00         0 trades
  2026-01-05  +$0.00         0 trades
  2026-01-07  +$0.00         0 trades
  2026-01-12  -$357.57       1 trades
  2026-01-14  +$0.00         0 trades
  2026-01-21  +$397.75       1 trades
  2026-01-26  +$315.19       1 trades
  2026-01-28  +$0.00         0 trades
  2026-02-02  +$278.95       1 trades
  2026-02-04  +$1045.17      1 trades
  2026-02-09  +$2426.70      2 trades
  2026-02-11  +$0.00         0 trades
  2026-02-18  +$0.00         0 trades
  2026-02-23  -$736.07       1 trades
  2026-02-25  +$796.92       1 trades
  2026-03-02  +$1972.78      2 trades

  in-sample: P&L +$13099.04  PF 3.84  win 26%  (99 days, 42 trades)

--- section 2: hold-out out-of-sample test (40 dates, never used in optimization) ---
  2025-03-13  +$0.00         0 trades
  2025-03-20  +$0.00         0 trades
  2025-03-27  +$0.00         0 trades
  2025-04-03  +$0.00         0 trades
  2025-04-10  -$198.97       1 trades
  2025-04-17  +$0.00         0 trades
  2025-04-24  +$2123.21      2 trades
  2025-05-01  -$1244.47      1 trades
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

  in-sample (99-day):     P&L +$13099.04  PF 3.84  win 26%
  out-of-sample (40-day): P&L +$1524.47  PF 2.05  win 5%
  OOS/IS per-day P&L ratio: 28%

--- section 3: slippage sensitivity sweep (20-day × 6 levels) ---
   1 bps:  +$7443.10  (20 days)
   2 bps:  +$7192.44  (20 days)
   4 bps:  +$6696.26  (20 days)
   6 bps:  +$6206.79  (20 days)
   8 bps:  +$5724.05  (20 days)
  10 bps:  +$4162.52  (20 days)
  breakeven: >10 bps

--- section 4: monte carlo daily P&L shuffle (1000 iterations) ---
  P&L percentiles:       5th: +$13099.04  50th: +$13099.04  95th: +$13099.04
  max drawdown pctiles:  5th: $1206.55  50th: $1607.59  95th: $2554.71

--- section 5: walk-forward stability ---
  windows: 23 (train=20, test=5, stride=5)
  OOS profitable windows: 15/23 (65%)
  walk-forward efficiency (WFE): 38%

--- section 6: per-ticker analysis ---
  ticker     total P&L   trades    win days      PF  (IS P&L / OOS P&L)
  AAPL       +$4139.63       11     8/139      3.9  (+$4338.60 / -$198.97)
  MSFT       +$4345.15       11     9/139      3.1  (+$3599.67 / +$745.48)
  NVDA       +$5304.55       20    14/139      3.4  (+$5304.55 / +$0.00)
  QQQ         +$798.30        4     2/139      2.7  (-$179.66 / +$977.96)
  SPY          +$35.87        1     1/139      inf  (+$35.87 / +$0.00)

=== validation report card ===
  [PASS] hold-out OOS test        P&L +$1524.47  PF 2.05  (28% per-day ratio)
  [PASS] slippage sensitivity     profitable at 6 bps (breakeven: >10 bps)
  [PASS] monte carlo shuffle      5th pctl: +$13099.04
  [FAIL] walk-forward stability   65% OOS profitable, WFE 38%
  [PASS] per-ticker analysis      all tickers PF >= 0.8
  ---
  OVERALL: FAIL (4/5)

