# research/entries — entry-window studies (closed rounds)

what this directory holds now:
- `make_variant.py`, `summarize.py`, `screen_indicators.json` — the tooling later studies reuse (`scripts/research_runner.sh` and the pipeline call `summarize.py`).
- `variants/` — only the cells a later step still consumes: `iex_*.json` (the SPY-band × VPIN grid on the IEX cache; `iex_b0.4_v0.217.json` is the v17/v18 baseline cell, the `--patch-json` base of the tiered-sizing study), `long_filters.json`, `long_unfiltered.json`.
- `data/earnings_*.txt` — derived earnings dates; the raw EDGAR `sec_*.json` they came from were deleted.

pruned in the commit following 1f196a4d019c954ca68820d869205eeb150fe53a (`docs/plans/2026-09-26_pipeline_and_books.md` §10): the SIP-era `grid_b*_v*.json` cells (§10's `grid_b0.4_v0.217.json` was never tracked; `iex_b0.4_v0.217.json` is that cell on IEX), the nine `cs_*` candlestick variants, the gap / opening-range / prior-day-low / peer / SPY-red / FOMC condition patches, and `data/sec_*.json`.
`git show 1f196a4:research/entries/variants/<file>` recovers any of them; the write-ups that cite them are `docs/paper_trading_plan_2026-09.md` §11–§13.
