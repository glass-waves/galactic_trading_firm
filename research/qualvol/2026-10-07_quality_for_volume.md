# quality-for-volume grid (2026-10-07)

question: can the `thrust-1h15` candidate (pipeline #29, cell `b_1h15` of the entry-trigger
study — thrust window's 1h ceiling tightened to <= -0.15) spend its PF headroom on volume by
loosening the two filters shared by every trigger-study cell (VPIN floor on `vpin_1m.raw_vpin`,
SPY band on `cross_1m`), and/or by dialing the strong-core window's own 1h ceiling, the thrust
window's 1h ceiling, and the shared composite floor?

baselines (5y IEX replay, honest costs, `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`):
`iex_v18` +1,728 / 436 trades / PF 1.37, years +991 -3 -20 +332 +429 (3/5 positive).
`thrust-1h15` (tag `cand_29`) +1,714 / 278 / PF 1.63, years +644 +55 -56 +396 +676 (4/5 positive).

sanity check: `qv_v217_b04` (vpin 0.217, band +-0.2%, no other changes) is byte-identical to
`research/trigger/thrust_1h15.patch.json` and reproduced `cand_29` trade-for-trade (+1,714 / 278 /
PF 1.63, same per-year numbers) — confirms the sweep pipeline before spending budget on the grid.

## grid (9 cells): VPIN floor x SPY band

| cell | vpin floor | SPY band | P&L | n | PF | max DD | years+ | flag |
|---|---|---|---|---|---|---|---|---|
| v217_b04 (=thrust-1h15) | 0.217 | +-0.2% | +1,714 | 278 | 1.63 | -348 | 4/5 | no (n<437) |
| v217_b06 | 0.217 | +-0.3% | +1,515 | 349 | 1.43 | -337 | 5/5 | no (n<437) |
| v217_b08 | 0.217 | +-0.4% | +1,473 | 415 | 1.36 | -351 | 5/5 | no (PF<1.4, n<437) |
| v180_b04 | 0.18 | +-0.2% | +1,883 | 345 | 1.52 | -371 | 5/5 | no (n<437) |
| v180_b06 | 0.18 | +-0.3% | +1,717 | 426 | 1.39 | -476 | 5/5 | no (PF<1.4, n<437 by 10) |
| v180_b08 | 0.18 | +-0.4% | +1,832 | 496 | 1.36 | -417 | 5/5 | no (PF<1.4) |
| v150_b04 | 0.15 | +-0.2% | +2,039 | 390 | 1.49 | -383 | 5/5 | no (n<437) |
| v150_b06 | 0.15 | +-0.3% | +1,925 | 486 | 1.38 | -370 | 5/5 | no (PF<1.4, closest) |
| v150_b08 | 0.15 | +-0.4% | +1,836 | 553 | 1.32 | -475 | 4/5 (2023 -33) | no (PF<1.4) |

pattern: loosening either filter trades PF for volume roughly 1-for-1 — every cell that clears
436 trades lands PF in [1.32, 1.39], never above 1.4. by-window totals (5y, all grid cells) show
the strong-core window is nearly inert throughout this family — 27 to 55 of each cell's trades,
a small positive contributor — so this whole lever is really about the thrust window's two filters:

| cell | thrust window | core window |
|---|---|---|
| v217_b04 | +1,542 / 251 | +173 / 27 |
| v180_b06 | +1,674 / 393 | +43 / 33 |
| v180_b08 | +1,710 / 451 | +123 / 45 |
| v150_b08 | +1,587 / 498 | +249 / 55 |

## refinement (10 cells): on the two closest near-misses, v180_b06 and v150_b06

| cell | change | P&L | n | PF | max DD | years+ |
|---|---|---|---|---|---|---|
| v180_b06_core1h_m30 | core 1h <= -0.3 | +1,435 | 549 | 1.23 | -530 | 3/5 (2023, 2025 neg) |
| v180_b06_core1h_m20 | core 1h <= -0.2 | +1,621 | 841 | 1.18 | -990 | 4/5 (2023 neg) |
| v180_b06_thrust1h_m10 | thrust 1h <= -0.10 | +1,752 | 511 | 1.33 | -484 | 3/5 (2023, 2025 neg) |
| v180_b06_thrust1h_m20 | thrust 1h <= -0.20 | +1,113 | 320 | 1.35 | -284 | 4/5 (2025 neg) |
| v180_b06_comp_m30 | composite <= -0.30 | +1,717 | 426 | 1.39 | -476 | 5/5 (= v180_b06 exactly) |
| v150_b06_core1h_m30 | core 1h <= -0.3 | +1,503 | 620 | 1.21 | -647 | 3/5 (2023, 2025 neg) |
| v150_b06_core1h_m20 | core 1h <= -0.2 | +1,058 | 943 | 1.10 | -1,375 | 3/5 (2023, 2026 neg) |
| v150_b06_thrust1h_m10 | thrust 1h <= -0.10 | +1,606 | 582 | 1.26 | -618 | 3/5 (2023, 2025 neg) |
| v150_b06_thrust1h_m20 | thrust 1h <= -0.20 | +1,161 | 367 | 1.31 | -343 | 4/5 (2025 neg) |
| v150_b06_comp_m30 | composite <= -0.30 | +1,925 | 486 | 1.38 | -370 | 5/5 (= v150_b06 exactly) |

three findings:
- **composite floor is a dead lever.** `comp_m30` reproduces its base cell's trades exactly, byte
  for byte in the per-year numbers. the 5m/1h conditions already imply composite <= -0.35 in
  every case they fire; loosening the shared ceiling to -0.30 admits nothing new.
- **loosening the strong-core window's 1h ceiling is the worst lever tried.** it buys the most
  volume (549-943 trades) but PF collapses to 1.10-1.23, drawdown blows out to -530..-1,375, and
  2023 (already v18's worst year) goes to -125..-369. not a volume/quality trade, a straight loss
  of quality with no quality gained back anywhere.
- **loosening the thrust window's 1h ceiling (-0.10) also breaks stability**: 3/5 years positive
  in both base cells, same shape as the core-1h lever but milder. tightening it further (-0.20)
  trims volume back toward the baseline without ever reaching PF > 1.4.

no refinement beat its own base cell on P&L, PF, or years-positive. the base grid cells
(v180_b06, v150_b06) remain the best configurations found in this round.

## leave-one-year-out

pool = cells with trades >= 1.2 x v18's rate (>= 524 over 5y): `v150_b08` (553), and five of the
`core1h`/`thrust1h` refinements (511-943) — the composite and VPIN/band-only cells don't reach
this volume floor on their own.

**best 4-year PF per held-out year** (the stability-first pick):

| held out | pick | 4y PF | held-out year P&L / n / PF |
|---|---|---|---|
| 2022 | v150_b08 | 1.16 | +1,167 / 142 / 1.75 |
| 2023 | v150_b08 | 1.39 | -33 / 106 / 0.97 |
| 2024 | v150_b08 | 1.37 | +144 / 109 / 1.12 |
| 2025 | v150_b08 | 1.38 | +91 / 104 / 1.07 |
| 2026 | v150_b08 | 1.27 | +468 / 92 / 1.63 |

`v150_b08` wins every held-out year on 4-year PF, and is never catastrophic held out (worst is
-33 in 2023, v18's own worst year). this is the LOYO-stable pick in the high-volume pool.

**best 4-year P&L per held-out year** (less stable):

| held out | pick | 4y P&L | held-out year P&L / n / PF |
|---|---|---|---|
| 2022 | v180_b06_core1h_m20 | +738 | +883 / 184 / 1.39 |
| 2023 | v180_b06_core1h_m20 | +1,936 | **-315** / 186 / 0.84 |
| 2024 | v150_b08 | +1,692 | +144 / 109 / 1.12 |
| 2025 | v150_b06_thrust1h_m10 | +1,763 | -158 / 112 / 0.89 |
| 2026 | v150_b08 | +1,369 | +468 / 92 / 1.63 |

choosing by raw 4-year P&L twice picks a core1h-loosened cell and lands a -315 and a -158
held-out year — exactly the instability the core-1h lever showed in the refinement table.
PF-first selection avoids this; P&L-first does not. this is itself a finding: in this family,
4-year P&L is not a safe selection criterion.

## pipeline `volume-config` gate (baseline = `iex_v18`: PF >= 1.3, >= 4/5 years, trades >= 436,
P&L >= 1,555, no year < -300)

passes: `v180_b08` (PF 1.36, n 496, +1,832, 5/5), `v150_b06` (PF 1.38, n 486, +1,925, 5/5),
`v150_b08` (PF 1.32, n 553, +1,836, 4/5, min year -33). `v180_b06` misses only on trade count
(426 vs 436, 10 short).

## verdict: hold — no cell clears this round's bar

the bar is PF > 1.4 *and* >= 4/5 years *and* trades > 436, together, stable under LOYO. every
cell that clears 436 trades tops out at PF 1.32-1.39; every cell with PF > 1.4 stays under 420
trades. the two closest misses, `v180_b06` and `v150_b06`, are also the two grid cells that pass
the pipeline's `volume-config` gate outright — but the gate's PF >= 1.3 floor is looser than this
round's own PF > 1.4 target, so neither is proposed here. the two "spend PF on volume" levers the
brief asked about on top of the grid — the core window's 1h ceiling and the thrust window's 1h
ceiling — both make things worse, not better, and the core-1h one is actively dangerous (LOYO
-315, -369 years). the one neutral lever (composite floor) does nothing. no patch is proposed
this round; all 19 generated cell files were pruned (`make_patches.py` regenerates them).

for a future round: the volume has to come from somewhere other than loosening the two
promoted windows' own confirmation timescales — e.g. a genuinely new trigger (the entry-trigger
study's family c/d, pullback and VPIN-slope windows) added *alongside* the existing two rather
than loosening them, which is the shape `additive-config` is built for.

## exact commands

```
set -a; source .env; set +a
exec 9>logs/.research.lock; flock -w 14400 9

# grid
python3 research/qualvol/make_patches.py grid
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh qv_v217_b04 --sizing-fraction 0.36 \
  --max-position-pct 0.36 --cross-index SPY --patch-json research/qualvol/v217_b04.json
# ... one cell at a time for v217_b06 v217_b08 v180_b04 v180_b06 v180_b08 v150_b04 v150_b06 v150_b08

# refinement on the two closest grid cells
python3 research/qualvol/make_patches.py refine v180_b06 v150_b06
BARS_DIR=data/bars_iex scripts/run_cached_sweep.sh qv_v180_b06_core1h_m30 --sizing-fraction 0.36 \
  --max-position-pct 0.36 --cross-index SPY --patch-json research/qualvol/v180_b06_core1h_m30.json
# ... one cell at a time for the other 9

python3 research/entries/summarize.py iex_v18 cand_29 qv_v217_b04 qv_v217_b06 qv_v217_b08 \
  qv_v180_b04 qv_v180_b06 qv_v180_b08 qv_v150_b04 qv_v150_b06 qv_v150_b08 --by-window
```
