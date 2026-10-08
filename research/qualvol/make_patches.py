#!/usr/bin/env python3
"""emit the quality-for-volume study's --patch-json files (research/qualvol/<cell>.json).

every cell starts from `research/trigger/thrust_1h15.patch.json` (pipeline candidate #29
`thrust-1h15`: v18's two promoted short windows, re-added as `_am` copies with their own 11:30 /
11:55 clock, thrust window's 1h condition tightened to <= -0.15). the grid loosens the two filters
that are identical in every trigger-study cell (VPIN floor on `vpin_1m.raw_vpin`, SPY band on
`cross_1m`) to buy back the volume the 1h tightening gave up, then — on the best two grid cells —
probes the strong-core window's own 1h condition, the thrust window's 1h dial, and the shared
composite floor.

grid (9): vpin floor in {0.217, 0.18, 0.15} x spy band (cross_1m [-b, b]) in {0.4, 0.6, 0.8}
  (0.4 = +-0.2 % of SPY's open, 0.6 = +-0.3 %, 0.8 = +-0.4 %; see
  research/entries/variants/iex_b0.4_v0.18.json for how these conditions read).
  cell `v217_b04` == thrust_1h15.patch.json exactly (sanity check: same trades as cand_29).

refinement (on each of the two cells named on the command line after `--refine`):
  core1h_m30 / core1h_m20   strong-core window's OneHour ceiling: -0.3 / -0.2 (baseline -0.4)
  thrust1h_m10 / thrust1h_m20   thrust window's OneHour ceiling: -0.10 / -0.20 (baseline -0.15)
  comp_m30                  composite_max on both windows: -0.30 (baseline -0.35)

usage:
  python3 research/qualvol/make_patches.py grid                  # write the 9 grid cells
  python3 research/qualvol/make_patches.py refine CELL [CELL...] # write the 5 refinements x each CELL
  python3 research/qualvol/make_patches.py all CELL [CELL...]    # grid + refine on the given cells
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
TRIGGER_DIR = os.path.join(HERE, "..", "trigger")

VPIN_FLOORS = [0.217, 0.18, 0.15]
BANDS = [0.4, 0.6, 0.8]


_VPIN_TAG = {0.217: "217", 0.18: "180", 0.15: "150"}
_BAND_TAG = {0.4: "04", 0.6: "06", 0.8: "08"}


def _tag(vpin, band):
    return f"v{_VPIN_TAG[vpin]}_b{_BAND_TAG[band]}"


def base_patch():
    with open(os.path.join(TRIGGER_DIR, "thrust_1h15.patch.json")) as f:
        return json.load(f)


def set_filters(patch, vpin=None, band=None):
    """mutate every window's cross_1m band and vpin_1m.raw_vpin floor in place."""
    for action in patch["actions"]:
        conds = action["params"]["conditions"]
        for c in conds:
            if c.get("type") == "indicator_min" and c.get("instance_id") == "cross_1m" and band is not None:
                c["min_score"] = -band
            elif c.get("type") == "indicator_max" and c.get("instance_id") == "cross_1m" and band is not None:
                c["max_score"] = band
            elif c.get("type") == "indicator_min" and c.get("instance_id") == "vpin_1m.raw_vpin" and vpin is not None:
                c["min_score"] = vpin
    return patch


def set_thrust_1h(patch, h1):
    for action in patch["actions"]:
        if action["instance_id"] != "window_5m_thrust_short_am":
            continue
        for c in action["params"]["conditions"]:
            if c.get("type") == "timescale_max" and c.get("timescale") == "OneHour":
                c["max_score"] = h1
    return patch


def set_core_1h(patch, h1):
    for action in patch["actions"]:
        if action["instance_id"] != "window_strong_core_short_am":
            continue
        for c in action["params"]["conditions"]:
            if c.get("type") == "timescale_max" and c.get("timescale") == "OneHour":
                c["max_score"] = h1
    return patch


def set_composite(patch, c_max):
    for action in patch["actions"]:
        for c in action["params"]["conditions"]:
            if c.get("type") == "composite_max":
                c["max_score"] = c_max
    return patch


def grid_cells():
    out = {}
    for vpin in VPIN_FLOORS:
        for band in BANDS:
            name = _tag(vpin, band)
            p = set_filters(copy.deepcopy(base_patch()), vpin=vpin, band=band)
            out[name] = p
    return out


def refine_cells(base_names):
    out = {}
    grid = grid_cells()
    for base_name in base_names:
        if base_name not in grid:
            raise SystemExit(f"unknown base cell {base_name!r}; choose from {sorted(grid)}")
        base = grid[base_name]
        variants = {
            "core1h_m30": lambda p: set_core_1h(p, -0.30),
            "core1h_m20": lambda p: set_core_1h(p, -0.20),
            "thrust1h_m10": lambda p: set_thrust_1h(p, -0.10),
            "thrust1h_m20": lambda p: set_thrust_1h(p, -0.20),
            "comp_m30": lambda p: set_composite(p, -0.30),
        }
        for vname, fn in variants.items():
            out[f"{base_name}_{vname}"] = fn(copy.deepcopy(base))
    return out


def write(cells):
    for k, v in cells.items():
        with open(os.path.join(HERE, f"{k}.json"), "w") as f:
            json.dump(v, f, indent=1)
            f.write("\n")
    print("\n".join(sorted(cells)))


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    mode = sys.argv[1]
    if mode == "grid":
        write(grid_cells())
    elif mode == "refine":
        write(refine_cells(sys.argv[2:]))
    elif mode == "all":
        g = grid_cells()
        r = refine_cells(sys.argv[2:])
        write({**g, **r})
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
