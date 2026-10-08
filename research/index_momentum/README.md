# index intraday momentum: a second, decoupled book (2026-10-07)

report: `2026-10-07_index_intraday_momentum.md`. question: can a standalone book trading the
index's own intraday momentum (Zarattini–Aziz noise-area breakout; Gao's last half hour) run next
to v18 from the same building blocks, uncorrelated, active on more days?

- code: `crates/indicators/src/custom/noise_area.rs` (new `noise_area` indicator, registered in
  `crates/indicators/src/lib.rs`, 9 unit tests). gap-adjusted noise boundaries from the hourly
  window (14 sessions, sqrt-time interpolation between hourly marks), plus `decision`,
  `day_move_pct`, `vwap_pct`, `ret_first30_pct`, `ret_hour_pct` metadata for entry windows.
- `make_patches.py` generates every swept cell (`python3 research/index_momentum/make_patches.py
  [cell ...]`); `--pipeline CELL OUT.json` writes the pipeline-format patch (adds `_sweep_args
  --lookback-days 22` for the 14-session lookback).
- `run_cell.sh` / `run_queue.sh` sweep one cell at a time under `logs/.research.lock` (tags
  `im_<cell>`, five-year IEX replay, 0.36 sizing, `--cross-index SPY`, `LOOKBACK=22` default,
  `COST_ARGS` / `TAG` env overrides). `build_locked.sh` rebuilds the release backtest under the lock.
- `analyze.py card|grid|decouple|dmgrid|loyo TAG[+TAG]`: per-cell card, correlation / overlap /
  combined book vs `iex_v18` and `tf_c_sq05_1100` (spy-sqrt-band), the day-level vol-filter grid
  (`--dm-min` derives a filtered cell exactly from its base), LOYO of the floor.
- `sim.py`: independent per-minute replica of the paper's rules on the bar cache (exact sigma,
  decision-point / every-bar / no stop, band, start, cadence, cost multiplier); 72-cell scan in
  the report §2.
- `verify_patch.sh <tag> <patch> <date> ...`: replays a pipeline patch alone (lookback 22) and
  compares trade rows with the sweep.

kept: scripts above and `qqq_noise_pm_vol.patch.json` (pipeline candidate #46 `qqq-noise-pm-vol`,
cell `na_qqq_pm_hold_dm07`). every other cell patch was pruned; `make_patches.py` regenerates
them. sweep outputs live in `data/im_*` and `logs/sweeps/im_*` (untracked).

verdict: the paper's full-day strategy and Gao fail on SPY and QQQ at our costs; one QQQ
afternoon / high-vol-day cell clears the owner's bar and is proposed, with a live blocker (needs
≥ 22 days of warmup; the trader warms 8) and 2022 / selection caveats (report §8).
