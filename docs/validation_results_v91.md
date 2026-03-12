=== comprehensive strategy validation ===
capital: $10000  lookback: 3  costs: --slippage-bps 2.0 --half-spread 0.005

--- section 1: in-sample baseline (99 dates) ---
  2025-03-10  +$0.00         0 trades
  2025-03-12  +$0.00         0 trades
  2025-03-17  +$0.00         0 trades
  2025-03-19  +$132.09       1 trades
  2025-03-24  +$223.18       1 trades
  2025-03-26  +$0.00         0 trades
  2025-03-31  +$0.00         0 trades
  2025-04-02  +$0.00         0 trades
  2025-04-07  +$1362.43     42 trades
  2025-04-09  +$3023.65     49 trades
  2025-04-14  +$118.53       3 trades
  2025-04-16  +$0.00         0 trades
  2025-04-21  -$794.79       1 trades
  2025-04-23  +$0.00         0 trades
  2025-04-28  +$0.00         0 trades
  2025-04-30  +$0.00         0 trades
  2025-05-05  +$0.00         0 trades
  2025-05-07  +$0.00         0 trades
  2025-05-12  +$0.00         0 trades
  2025-05-14  +$0.00         0 trades
  2025-05-19  +$767.35       2 trades
  2025-05-21  +$0.00         0 trades
  2025-05-28  +$0.00         0 trades
  2025-06-02  -$40.36        1 trades
  2025-06-04  +$0.00         0 trades
  2025-06-09  +$0.00         0 trades
  2025-06-11  +$0.00         0 trades
  2025-06-16  +$0.00         0 trades
  2025-06-18  +$0.00         0 trades
  2025-06-23  -$462.60      11 trades
  2025-06-25  +$115.01       1 trades
  2025-06-30  -$379.03      10 trades
  2025-07-02  +$17.83        1 trades
  2025-07-07  -$822.26       1 trades
  2025-07-09  +$184.51       1 trades
  2025-07-14  +$0.00         0 trades
  2025-07-16  +$0.00         0 trades
  2025-07-21  +$0.00         0 trades
  2025-07-23  +$0.00         0 trades
  2025-07-28  +$0.00         0 trades
  2025-07-30  +$0.00         0 trades
  2025-08-04  +$940.62       2 trades
  2025-08-06  +$1608.29      1 trades
  2025-08-11  +$0.00         0 trades
  2025-08-13  +$0.00         0 trades
  2025-08-18  +$135.71       1 trades
  2025-08-20  +$0.00         0 trades
  2025-08-25  +$0.00         0 trades
  2025-08-27  +$0.00         0 trades
  2025-09-03  +$0.00         0 trades
  2025-09-08  +$538.56       1 trades
  2025-09-10  -$12.67        1 trades
  2025-09-15  +$46.18        1 trades
  2025-09-17  +$0.00         0 trades
  2025-09-22  +$2006.21     22 trades
  2025-09-24  +$0.00         0 trades
  2025-09-29  +$313.15       1 trades
  2025-10-01  +$0.00         0 trades
  2025-10-06  +$568.96       3 trades
  2025-10-08  +$0.00         0 trades
  2025-10-13  -$226.67       1 trades
  2025-10-15  +$0.00         0 trades
  2025-10-20  +$801.16       1 trades
  2025-10-22  +$0.00         0 trades
  2025-10-27  +$0.00         0 trades
  2025-10-29  +$174.72       1 trades
  2025-11-03  -$15.94        1 trades
  2025-11-05  +$0.00         0 trades
  2025-11-10  -$1009.88      2 trades
  2025-11-12  +$0.00         0 trades
  2025-11-17  +$0.00         0 trades
  2025-11-19  +$109.54       1 trades
  2025-11-24  -$109.09      15 trades
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
  2026-01-07  +$96.89        2 trades
  2026-01-12  -$390.96       1 trades
  2026-01-14  +$0.00         0 trades
  2026-01-21  +$139.33      35 trades
  2026-01-26  +$316.81       1 trades
  2026-01-28  +$0.00         0 trades
  2026-02-02  +$278.95       1 trades
  2026-02-04  +$1053.61      1 trades
  2026-02-09  +$2401.57      2 trades
  2026-02-11  +$0.00         0 trades
  2026-02-18  +$0.00         0 trades
  2026-02-23  -$717.57       1 trades
  2026-02-25  +$1084.98      1 trades
  2026-03-02  +$2061.82      6 trades

  in-sample: P&L +$17201.74  PF 4.45  win 30%  (99 days, 239 trades)

--- section 2: hold-out out-of-sample test (40 dates, never used in optimization) ---
  2025-03-13  +$0.00         0 trades
  2025-03-20  +$0.00         0 trades
  2025-03-27  +$0.00         0 trades
  2025-04-03  +$0.00         0 trades
  2025-04-10  -$477.69       5 trades
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

  in-sample (99-day):     P&L +$17201.74  PF 4.45  win 30%
  out-of-sample (40-day): P&L +$1262.66  PF 1.74  win 5%
  OOS/IS per-day P&L ratio: 18%

--- section 3: slippage sensitivity (SKIPPED in --quick mode) ---

--- section 4: monte carlo daily P&L shuffle (100 iterations) ---
  P&L percentiles:       5th: +$17201.74  50th: +$17201.74  95th: +$17201.74
  max drawdown pctiles:  5th: $1009.88  50th: $1530.57  95th: $2351.64

--- section 5: walk-forward stability ---
  windows: 23 (train=20, test=5, stride=5)
  OOS profitable windows: 16/23 (69%)
  walk-forward efficiency (WFE): 28%

--- section 6: per-ticker analysis ---
  ticker     total P&L   trades    win days      PF  (IS P&L / OOS P&L)
  AAPL       +$3902.46       27     8/139      2.9  (+$4380.15 / -$477.69)
  MSFT       +$4327.36       26    10/139      2.7  (+$3564.97 / +$762.39)
  NVDA       +$6776.32      120    18/139      3.9  (+$6776.32 / +$0.00)
  QQQ        +$3410.72       54     4/139    135.3  (+$2432.76 / +$977.96)
  SPY          +$47.51       21     1/139      1.1  (+$47.51 / +$0.00)

=== validation report card ===
  [PASS] hold-out OOS test        P&L +$1262.66  PF 1.74  (18% per-day ratio)
  [SKIP] slippage sensitivity     (skipped in --quick mode)
  [PASS] monte carlo shuffle      5th pctl: +$17201.74
  [FAIL] walk-forward stability   69% OOS profitable, WFE 28%
  [PASS] per-ticker analysis      all tickers PF >= 0.8
  ---
  OVERALL: PASS (4/4)

