# quality-for-volume grid (2026-10-07)

report: `2026-10-07_quality_for_volume.md`. question: does the entry-trigger study's `thrust-1h15`
candidate (pipeline #29, cand_29) have PF headroom to spend on volume, by loosening the VPIN floor
and SPY band every trigger-study cell shares, or by loosening the strong-core/thrust windows' own
1h ceilings and the shared composite floor?

- `make_patches.py grid` writes the 9 VPIN-floor x SPY-band cells, starting from
  `research/trigger/thrust_1h15.patch.json` (cell `v217_b04` reproduces it, and `cand_29`, exactly).
  `make_patches.py refine CELL [CELL...]` writes 5 single-dial variants (core-1h x2, thrust-1h x2,
  composite floor x1) on named base cells.
- `run_cached_sweep.sh` under `logs/.research.lock` as in `research/trigger/`; tags `qv_<cell>`.
- verdict: hold. no cell clears PF > 1.4 with trades > 436 and >= 4/5 years, stable under
  leave-one-year-out. loosening the two windows' own 1h ceilings is actively unstable (LOYO years
  as low as -315/-369); the composite floor is a dead lever (never binding once the other
  conditions fire). the two closest misses pass the pipeline's `volume-config` gate on its own
  (looser PF floor) but not this round's own bar, so nothing was proposed.

kept: `make_patches.py`, this report. every generated cell (`v*.json`, the 10 refinement files)
was pruned — regenerate with `make_patches.py grid` / `make_patches.py refine <cell> <cell>`.
sweep outputs live in `data/qv_*` and `logs/sweeps/qv_*` (untracked).
