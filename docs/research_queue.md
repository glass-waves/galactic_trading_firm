# research queue

proposals from the evening routine; results written back by `scripts/research_runner.sh` on the
desktop. the human approves an entry by changing `status: proposed` to `status: approved`.
format and rules: `docs/routines/eod-report.md` §6. append only.

### RQ-1 plateau cell: SPY ±0.3 % / VPIN ≥ 0.18 as v19 candidate   status: done
proposed: 2026-09-21 by human (from plan doc §11.6 / §13.5)
hypothesis: widening the two v17 filters to the validated plateau cell raises trade count ~39 % while keeping PF > 1.3 and every year positive.
why now: v18 live rate 2 trades in 8 sessions vs 0.4/day expected; the grid was pre-computed.
command: (already run as tag e_grid_b0.6_v0.18)
accept if: PF ≥ 1.3 pooled, positive in 5 of 5 years.
result: 652 trades / +3,068 / PF 1.47; 2022 +1,667 · 2023 +138 · 2024 +252 · 2025 +314 · 2026 +697. passes. held as the fallback if the week of 09-22 ends without trades.

### RQ-2 runner smoke test: single-day replay   status: done
proposed: 2026-09-23 by human
hypothesis: the research runner executes an approved command and writes the result back.
why now: first wiring of the wheel.
command: ./target/release/backtest --date 2026-09-23 --lookback-days 8 --capital 10000 --slippage-bps 3.0 --half-spread 0.005 --output-trades-csv --bars-dir data/bars --cross-index SPY | grep -c '^trade,'
accept if: result shows the trade count for 2026-09-23 (expected 1).
result: ran 2026-09-23 20:28 PT, 0 min
```
1
```
stderr: DATA_QUALITY:AMZN:2730:8
DATA_QUALITY:AAPL:2730:8
DATA_QUALITY:NVDA:2730:8
DATA_QUALITY:MSFT:2730:8
