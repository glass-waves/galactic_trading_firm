# research/macd — scale-free MACD (RQ-5 / RQ-8)

**what** `k01.patch.json` replaces row 12's `macd_5min` (5m 12/26/9, w 0.4) and `macd_fast_1min` (1m 6/13/5, w 0.25) in place, same id/timescale/weight, with `normalization_factor: 0`.
**mode** in `crates/indicators/src/native/macd.rs`, `normalization_factor <= 0` sets the divisor to 1 % of the *last* close of that timescale's window (not an average), so score = clamp(histogram / (0.01 × close), ±1). factor 1.0 (row 12) is histogram in dollars.
**why** dollar MACD makes the composite's sensitivity depend on price level (NVDA $120 vs $1,200, AAPL vs MSFT) and is why split-adjusted replays move ~80 trades (docs/research/2026-10-14_split_adjustment.md).
**caveat** the 1 % divisor is not "same sensitivity, scale-free": it shrinks the scores ~2.5–5x on these names (mean |macd_5min| on 2026-10-22: MSFT 0.21→0.04, AMZN 0.22→0.09; nothing reaches the clamp). RQ-5's "scaled so the clamp binds at the same % move" needs a new knob (e.g. a pct divisor param); k01 tests the existing mode only.
**run (one day)** `./target/release/backtest --date 2026-10-22 --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --sizing-fraction 0.36 --max-position-pct 0.36 --bars-dir data/bars_iex --cross-index SPY --cross-lag 1 --output-trades-csv --patch-json research/macd/k01.patch.json` (add `--dump-ticks F` for per-bar macd scores).
**5y** `scripts/pipeline/pipeline.py propose --name macd-k01 --kind config --patch research/macd/k01.patch.json --gate quality-config` — the pipeline sweeps; do not hand-run 5y sweeps.
**check 2026-10-22** baseline 3 trades / +89.16; k01 1 trade / +45.75 (AMZN only, entry 4 min later; NVDA and MSFT shorts no longer fire).

## phase B (RQ-12): `pct_<k>.patch.json`
same two instances, `normalization_factor` back to 1.0 and `normalization_pct` k ∈ {0.003, 0.004, 0.005, 0.007}
(the histogram is scored against k × close; today's $1 norm ≈ 0.0033 × price at $300). added 2026-10-29.
