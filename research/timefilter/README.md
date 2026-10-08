# time-consistent market filter study (2026-10-07)

report: `2026-10-07_time_consistent_filter.md`. question: is v18's SPY-flat band (SPY session return
within ±0.2 % of its open) a clock, and does a filter with the same meaning at any time of day open
more of the session to the short book's edge?

- `make_patches.py` generates every cell (`python3 research/timefilter/make_patches.py [cell ...]`):
  the `_am` scaffold `am` (v18's two short windows re-added unchanged with their own 11:30 / 11:55
  clock, session opened to 15:30 / 15:55 with the `session` key, weight-0 `vpin_p` for the dump;
  reproduces `iex_v18` 436 / 436, P&L to the cent) and the cells of the report's grid.
  `--pipeline CELL OUT.json` writes the pipeline-format patch (disable + windows only).
- `run_cell.sh` / `run_queue.sh` sweep one cell at a time under `logs/.research.lock` (tags `tf_<cell>`,
  five-year IEX replay, honest costs, `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`;
  extra args pass through, e.g. `--cross-lag 1` for the `_lag1` cells). `wait_cell.sh` blocks until a
  cell's sweep is done; `build_locked.sh` rebuilds the shared backtest binary under the lock.
- `diag.py` reads the scaffold's full-day dump (`DUMP_TICKS=1 research/timefilter/run_cell.sh am`,
  `data/tf_am_<year>_ticks.csv`, ~3.4 GB, deleted after the study) and prints the diagnosis tables;
  its output is kept in `diag_out.txt`.
- `analyze.py grid|loyo|loyo_vol|windows|diff|card` (grid / LOYO / diff reuse
  `research/trigger/analyze.py`, gates from `scripts/pipeline/gates.py`); `tables.sh` prints all of them.
- `verify_patch.sh <tag> <patch> <date> ...` replays a pipeline patch alone and compares trade rows.
- code: `crates/indicators/src/custom/cross_context.rs` gained one metadata key,
  `index_session_ret_per_sqrt_min` (SPY session return % ÷ sqrt(minutes since 09:30 ET incl. the
  current bar)), with four unit tests in the same file.

kept: scripts above, `am.json` (scaffold), `spy_sqrt_band.patch.json` (pipeline candidate
`spy-sqrt-band`, cell `c_sq05_1100`), `diag_out.txt`. every other grid patch was pruned;
`make_patches.py` regenerates them. sweep outputs live in `data/tf_*` and `logs/sweeps/tf_*` (untracked).
