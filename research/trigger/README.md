# entry-trigger study (2026-10-02)

report: `2026-10-02_entry_trigger_study.md`. question: is there a better trigger than the two promoted
short windows (v18, row 12) for the same edge, with the SPY band and the VPIN floor fixed in every cell?

- `make_patches.py` generates every cell (`python3 research/trigger/make_patches.py [cell ...]`): the `_am`
  scaffold (both promoted windows re-added unchanged, plus the weight-0 `trig_1m` indicator) and the five
  families a–e. `am.json` reproduces `iex_v18` trade for trade (436 / 436, same P&L).
- `run_cell.sh` / `run_queue.sh` sweep one cell at a time under `logs/.research.lock` (tags `tr_<cell>`,
  five-year IEX replay, honest costs, `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`).
  the scaffold was swept with `DUMP_TICKS=1 ... --dump-window-only` (morning bars, `data/tr_am_*_ticks.csv`).
- `calibrate.py` counts bars on that dump to set the pullback and VPIN-slope thresholds by frequency (never
  by forward return); `analyze.py` builds the grid (pipeline gates imported from `scripts/pipeline/gates.py`),
  per-window subsets, kept / removed / re-timed / new splits vs `iex_v18`, leave-one-year-out and entry-bar
  profiles.
- code added: `crates/indicators/src/custom/trigger_context.rs` (`trigger_context`: pullback geometry over
  today's last N 1m bars + raw VPIN and its 3/5-bar slope; registered in `lib.rs`, tests in
  `crates/indicators/tests/custom_indicators.rs`). `vpin.rs` is untouched.

kept: generator, runners, analysis scripts, `am.json` (scaffold) and `thrust_1h15.patch.json` (pipeline
candidate #29 `thrust-1h15`, gate quality-config: cell `b_1h15` minus the unused weight-0 `trig_1m`).
every other grid patch was pruned; `make_patches.py` regenerates them. sweep outputs live in `data/tr_*` and
`logs/sweeps/tr_*` (untracked).
