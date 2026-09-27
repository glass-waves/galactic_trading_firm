#!/usr/bin/env python3
"""emit the stress-mode study's --patch-json files (research/stress/*.json).

every cell = v18 unchanged (its two promoted short windows re-added as `_am` copies that carry
their own 11:30 / 11:55 clocks, conditions identical to config_versions row 12) plus a pair of
stress windows: the same two windows with the SPY-flat band removed and a stress trigger added,
entry_after 09:45, entry_before 15:30, exit_overrides {force_exit_by 15:55, max_hold_ms,
score_exit_threshold}. shorts score-exit when composite >= -threshold, so the threshold is
NEGATIVE (-0.6 = exit when the composite flips to +0.6, -10 = disabled). pdl_5m is appended
(weight 0, not in v18); cross_1m / vpin_1m already exist in v18 and are read via metadata keys.

usage: python3 research/stress/make_patches.py   (writes all files, prints the list)
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PROMOTED_SHORTS = ["window_5m_thrust_short", "window_strong_core_short"]
VPIN = {"type": "indicator_min", "instance_id": "vpin_1m.raw_vpin", "min_score": 0.217}
SPY_BAND = [
    {"type": "indicator_min", "instance_id": "cross_1m", "min_score": -0.4},
    {"type": "indicator_max", "instance_id": "cross_1m", "max_score": 0.4},
]
THRUST_CORE = [  # row 12 `5m thrust short` minus band / vpin
    {"type": "composite_max", "max_score": -0.35},
    {"type": "timescale_lag", "lag_by": 0.1, "timescale": "FiveMinute"},
    {"type": "timescale_max", "max_score": -0.5, "timescale": "FiveMinute"},
    {"type": "timescale_max", "max_score": 0.0, "timescale": "OneHour"},
]
CORE_CORE = [  # row 12 `strong core short` minus band / vpin
    {"type": "composite_max", "max_score": -0.35},
    {"type": "timescale_max", "max_score": -0.4, "timescale": "FiveMinute"},
    {"type": "timescale_max", "max_score": -0.4, "timescale": "OneHour"},
]
PDL = {"indicator_type": "prior_day_levels", "instance_id": "pdl_5m", "timescale": "FiveMinute",
       "weight": 0.0, "enabled": True, "params": {"scale_pct": 0.005}}

TRIGGERS = {
    # SPY session return (open -> now), cross_1m metadata in percent
    "s05": [{"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -0.5}],
    "s10": [{"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -1.0}],
    "s15": [{"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -1.5}],
    "s20": [{"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -2.0}],
    # the name vs its prior close (includes the gap), pdl_5m metadata in percent
    "n15": [{"type": "indicator_max", "instance_id": "pdl_5m.dist_close_pct", "max_score": -1.5}],
    "n25": [{"type": "indicator_max", "instance_id": "pdl_5m.dist_close_pct", "max_score": -2.5}],
    # combined: SPY <= -1 % and the name <= -1.5 %
    "c10n15": [{"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -1.0},
               {"type": "indicator_max", "instance_id": "pdl_5m.dist_close_pct", "max_score": -1.5}],
    # weaker combined: SPY <= -0.5 % and the name <= -1.5 %
    "c05n15": [{"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -0.5},
               {"type": "indicator_max", "instance_id": "pdl_5m.dist_close_pct", "max_score": -1.5}],
}


def window(instance_id, name, priority, conditions, entry_after, entry_before, exit_overrides):
    p = {"name": name, "direction": "short", "conditions": conditions}
    if entry_after:
        p["entry_after"] = entry_after
    if entry_before:
        p["entry_before"] = entry_before
    if exit_overrides:
        p["exit_overrides"] = exit_overrides
    return {"instance_id": instance_id, "action_type": "entry_window", "phase": "Entry",
            "priority": priority, "enabled": True, "params": p}


def am_windows():
    return [
        window("window_5m_thrust_short_am", "5m thrust short", 10, THRUST_CORE + SPY_BAND + [VPIN],
               None, "11:30", {"force_exit_by": "11:55"}),
        window("window_strong_core_short_am", "strong core short", 20, CORE_CORE + SPY_BAND + [VPIN],
               None, "11:30", {"force_exit_by": "11:55"}),
    ]


FALLING = {"type": "indicator_max", "instance_id": "cross_1m.index_ret_15m", "max_score": -0.1}


def stress_windows(trigger, vpin=True, max_hold_ms=7_200_000, score_exit=-0.6,
                   entry_after="09:45", entry_before="15:30", extra=(), core_only=False):
    ov = {"force_exit_by": "15:55", "max_hold_ms": max_hold_ms, "score_exit_threshold": score_exit}
    v = [VPIN] if vpin else []
    x = list(extra)
    w = [window("window_5m_thrust_stress", "5m thrust stress", 30, THRUST_CORE + v + TRIGGERS[trigger] + x,
                entry_after, entry_before, copy.deepcopy(ov)),
         window("window_strong_core_stress", "strong core stress", 40, CORE_CORE + v + TRIGGERS[trigger] + x,
                entry_after, entry_before, copy.deepcopy(ov))]
    return w[1:] if core_only else w


def cell(trigger=None, am=True, **kw):
    acts = (am_windows() if am else []) + (stress_windows(trigger, **kw) if trigger else [])
    return {"disable": list(PROMOTED_SHORTS), "indicators": [copy.deepcopy(PDL)], "actions": acts}


def sizing_patch(trigger_key, mult=1.5):
    """indicator_tiered on the stress trigger's own value: at/above the trigger x1.0 (ordinary
    book untouched: the morning windows only fire with SPY inside +-0.2 %), below it x`mult`."""
    inst, thr = {"s05": ("cross_1m.index_session_ret", -0.5), "s10": ("cross_1m.index_session_ret", -1.0),
                 "s15": ("cross_1m.index_session_ret", -1.5), "s20": ("cross_1m.index_session_ret", -2.0),
                 "n15": ("pdl_5m.dist_close_pct", -1.5),
                 "n25": ("pdl_5m.dist_close_pct", -2.5)}[trigger_key]
    return {"disable": ["sizing_fixed"], "actions": [
        {"instance_id": "sizing_tiered_stress", "action_type": "indicator_tiered", "phase": "Sizing",
         "priority": 0, "enabled": True,
         "params": {"instance_id": inst, "base_fraction": 0.36,
                    "tiers": [{"min": thr, "mult": 1.0}], "fallback_mult": mult}}]}


def pipeline_patch(cell_file, out_file, extra_files=()):
    """merge a swept cell (+ optional extra patches, e.g. sizing) into one pipeline-format patch
    that carries the session clocks itself instead of the sweep flags."""
    merged = {"disable": [], "indicators": [], "actions": [],
              "session": {"no_new_entries_after": "15:30", "force_exit_by": "15:55"}}
    for f in (cell_file,) + tuple(extra_files):
        p = json.load(open(f))
        merged["disable"] += p.get("disable", [])
        merged["indicators"] += p.get("indicators", [])
        merged["actions"] += p.get("actions", [])
        if "session" in p:
            merged["session"].update(p["session"])
    with open(out_file, "w") as f:
        json.dump(merged, f, indent=1)
        f.write("\n")
    return merged


def main():
    if len(sys.argv) > 2 and sys.argv[1] == "--pipeline":
        # make_patches.py --pipeline OUT CELL.json [EXTRA.json ...]
        pipeline_patch(sys.argv[3], sys.argv[2], sys.argv[4:])
        print(sys.argv[2])
        return
    out = {}
    out["am_only"] = cell()                               # scaffold: must reproduce iex_v18
    for t in TRIGGERS:
        out[t] = cell(t)                                  # default variant: vpin, 2h, se -0.6
    # exit / filter variants (generated for every trigger; only the promising ones get swept)
    for t in TRIGGERS:
        out[f"{t}_h3"] = cell(t, max_hold_ms=10_800_000)
        out[f"{t}_se10"] = cell(t, score_exit=-10.0)
        out[f"{t}_novpin"] = cell(t, vpin=False)
        out[f"{t}_h3_se10"] = cell(t, max_hold_ms=10_800_000, score_exit=-10.0)
        out[f"{t}_only"] = cell(t, am=False)              # stress windows alone (pre-emption)
        out[f"{t}_late"] = cell(t, entry_after="10:30")   # skip the opening flush
        out[f"{t}_fall"] = cell(t, extra=[FALLING])       # SPY still falling (15 m ret <= -0.1 %)
        out[f"{t}_late_fall"] = cell(t, entry_after="10:30", extra=[FALLING])
        out[f"{t}_core"] = cell(t, core_only=True)        # strong-core stress window only
        out[f"{t}_core_only"] = cell(t, am=False, core_only=True)
        out[f"{t}_core_novpin"] = cell(t, core_only=True, vpin=False)
        out[f"{t}_core_h3_se10"] = cell(t, core_only=True, max_hold_ms=10_800_000, score_exit=-10.0)
        out[f"{t}_core_late"] = cell(t, core_only=True, entry_after="10:30")
    for t in ("s05", "s10", "s15", "s20", "n15", "n25"):
        out[f"size_{t}_x1.5"] = sizing_patch(t, 1.5)
    for k, v in out.items():
        with open(os.path.join(HERE, f"{k}.json"), "w") as f:
            json.dump(v, f, indent=1)
            f.write("\n")
    print("\n".join(sorted(out)))


if __name__ == "__main__":
    main()
