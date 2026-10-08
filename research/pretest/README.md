# research/pretest — stage 0 of the research pipeline

Cheap python replicas (stdlib + numpy, no engine, no pandas) of documented anomalies from
`docs/edge_catalog.md`, run cell by cell from `research/edge_matrix.md`. A cell that passes its
pre-registered kill criterion here earns an engine study; a cell that fails is marked `dead` in the
matrix with the number that killed it. Nothing here proposes anything to `scripts/pipeline/`.

```
python3 research/pretest/harness.py                 # every cell in cells.py (~20 s)
python3 research/pretest/harness.py c02_turn_of_month c05_earnings_gap
```

Outputs: `results/<cell>.json` (every variant x instrument x grid point: metrics, per-year, LOYO,
pass/why, the primary point's daily P&L series, cell `extra` diagnostics) and `results/summary.md`
(one screen). The first run builds `cache/<SYM>.npz` from `data/bars_iex/<SYM>.csv` (gitignored,
rebuilt automatically when the CSV is newer).

## files
- `harness.py` — (a) loaders: `bars(sym)` = minute grid (sessions x 390 slots, ET clock, NaN where
  IEX has no bar), `load_daily(sym)` for `research/swing/daily/`; (b) the rule interface:
  `rule(b, i, params) -> [Order(si, t_in, side, xi, t_out, stop, target)]`, clock boundaries in
  minutes since 09:30, exit session `xi > si` = an overnight hold; (c) `cost()` 3 bps + $0.005/share
  per leg; (d) `metrics()` — summarize.py definitions (36 % of $10k, whole shares, P&L, n, win %, PF,
  per-year, max DD on daily P&L, worst day) plus net/gross bps per trade, trades/yr, active-day
  share, P&L ex the 3 best trades; `loyo()` across a cell's grid; `kill()` = the standard bar;
  (e) the runner and the json/markdown writers.
- `cells.py` — the registry: per cell id, anomaly, instruments (home first, `A+B` = one pooled
  book), variants (rule, small grid whose first point is the paper's parameters, a flag for
  versions the intraday-only engine cannot trade), the pre-registered kill text, `verdict_fn` and
  the cell `verdict()` (`pass-pretest` / `needs-product` / `dead`), optional `extra()` diagnostics.

## conventions
- entering at boundary T fills at the open of the first bar >= T; exiting at T fills at the close
  of the last bar < T (T = 390 is the session's last close). Stops/targets checked on bar high/low,
  gap-through fills at the bar open, stop wins a bar that touches both.
- a grid point other than the paper's may pass only if leave-one-year-out selection also clears
  the bar. Overnight / multi-day versions are reported but can only earn `needs-product`.
- `calendar_control()` compares a calendar rule with the same trade on random sessions (5000 draws)
  — a long-only rule in a rising market needs this to mean anything.

## adding a cell
Write a rule in `cells.py`, append a dict to `CELLS` with the kill criterion copied from the
matrix *before* running it, run `harness.py <id>`, and record the verdict in the matrix.

## rounds
- `2026-10-08_pretest_round1.md` — matrix top-5: pre-FOMC, turn-of-month, HKS half-hour, 5-min ORB,
  earnings-day gap fade.
- `2026-10-08_pretest_round2.md` — matrix items 6-10: gap fade/fill, overnight-decile lean,
  hedging-demand momentum, FOMC-day and OpEx-day range compression. Each cell scans a small
  pre-registered neighbourhood (threshold/decile/window, 2-3 values; home-class instruments —
  SPY+QQQ+IWM for index cells, the 4 names + the other ~30 large caps in `data/bars_iex` for
  single-stock cells) rather than one grid point, and uses finer verdicts (`dead` / `sub-cost` /
  `decayed` / `fragile` / `pass`) alongside the matrix's `pass-pretest`/`needs-product`/`dead`.
