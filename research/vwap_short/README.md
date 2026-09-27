# VWAP-anchored short study (2026-09-27)

report: `2026-09-27_vwap_anchored_short.md` (verdict: fails as specified, no pipeline patch).
`make_patches.py` generates every cell (it imports the `_am` windows from `research/stress/make_patches.py`);
`run_cell.sh` / `run_queue.sh` sweep them one at a time under `logs/.research.lock` (tags `vw_<cell>`);
`analyze.py` (copied from the stress study, plus `--walk`) builds the grid, the tail-day tables, pre-emption
and gate checks; `bar_check.py` is the engine-independent raw-bar check behind §4.

kept patches: `am_only.json` (scaffold, reproduces `iex_v18` 436/436), `s15_d08_b05.json` (the hypothesis as
specified: SPY ≤ −1.5 %, D 0.8 %, VWAP stop 0.5 %) and `s05_d08_nostop.json` (the best cell). every other
grid patch was pruned before any commit; `python3 research/vwap_short/make_patches.py` regenerates them all.
sweep outputs live in `data/vw_*` and `logs/sweeps/vw_*` (untracked). code added for this study:
`crates/actions/src/exit/vwap_stop.rs` (+ registration, tests in `crates/actions/tests/action_tests.rs`) and
the metadata keys in `crates/indicators/src/composable/vwap_distance.rs` (+ tests in
`crates/indicators/tests/composable_indicators.rs`). all numbers are on the VWAP-fixed replay (plan doc §16).
