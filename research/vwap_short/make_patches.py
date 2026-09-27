#!/usr/bin/env python3
"""emit the VWAP-anchored short study's --patch-json files (research/vwap_short/*.json).

every cell = v18 unchanged (its two promoted short windows re-added as the stress study's `_am`
copies with their own 11:30 / 11:55 clocks, conditions identical to config_versions row 12) plus:
- `vwap_1m`: a vwap_distance instance on OneMinute at weight 0 (metadata only: dist_pct,
  min_dist_pct_today, rebound_since_min_pct, high_dist_pct, ...);
- one short window `vwap retest stress` (10:00-15:15 ET): SPY session return <= trigger
  (none for the control), the name was >= D % below its running VWAP earlier today, is now within
  [-0.15 %, +0.10 %] of VWAP, and this is the first close back in that band since the day's low
  (rebound_since_min_pct <= -0.16: one entry per retest; a second needs a new low vs VWAP).
  exit_overrides {force_exit_by 15:55, max_hold_ms 3 h, score_exit_threshold -10 (off)};
  priority -5 so it is evaluated before the 1m noise reject gates (a VWAP retest *is* a 1m
  counter-move, which the long-side gate rejects; see the report);
- `vwap_stop` (exit, priority 2): short exits once close > VWAP x (1 + buffer %); scoped with
  scope_min_max_hold_ms 7,200,000 so only positions whose window set a >= 2 h max hold (the VWAP
  window's 3 h override; the ordinary windows run on the 90 min default) are touched.

usage: python3 research/vwap_short/make_patches.py        (writes all cells, prints the list)
       python3 research/vwap_short/make_patches.py --pipeline OUT CELL.json [EXTRA.json ...]
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "stress"))
from make_patches import PROMOTED_SHORTS, am_windows, pipeline_patch  # noqa: E402  (stress study)

VWAP_1M = {"indicator_type": "vwap_distance", "instance_id": "vwap_1m", "timescale": "OneMinute",
           "weight": 0.0, "enabled": True, "params": {}}
SPY = "cross_1m.index_session_ret"
TRIG = {"c": None, "s00": 0.0, "s05": -0.5, "s10": -1.0, "s15": -1.5, "s20": -2.0}


def vwap_window(trigger, depth, band=(-0.15, 0.10), first_retest=True, entry_after="10:00",
                entry_before="15:15", max_hold_ms=10_800_000, priority=-5, extra=(), name="vwap retest stress"):
    cond = []
    if TRIG[trigger] is not None:
        cond.append({"type": "indicator_max", "instance_id": SPY, "max_score": TRIG[trigger]})
    cond += [
        {"type": "indicator_max", "instance_id": "vwap_1m.min_dist_pct_today", "max_score": -depth},
        {"type": "indicator_range", "instance_id": "vwap_1m.dist_pct", "min_score": band[0], "max_score": band[1]},
    ]
    if first_retest:
        cond.append({"type": "indicator_max", "instance_id": "vwap_1m.rebound_since_min_pct",
                     "max_score": band[0] - 0.01})
    cond += list(extra)
    return {"instance_id": "window_vwap_retest_stress", "action_type": "entry_window", "phase": "Entry",
            "priority": priority, "enabled": True,
            "params": {"name": name, "direction": "short", "conditions": cond,
                       "entry_after": entry_after, "entry_before": entry_before,
                       "exit_overrides": {"force_exit_by": "15:55", "max_hold_ms": max_hold_ms,
                                          "score_exit_threshold": -10.0}}}


def vwap_stop(buffer_pct):
    return {"instance_id": "vwap_stop", "action_type": "vwap_stop", "phase": "Exit", "priority": 2,
            "enabled": True, "params": {"buffer_pct": buffer_pct, "scope_min_max_hold_ms": 7_200_000}}


def cell(trigger=None, depth=0.5, buffer_pct=0.3, am=True, **kw):
    acts = am_windows() if am else []
    if trigger is not None:
        acts.append(vwap_window(trigger, depth, **kw))
    acts.append(vwap_stop(buffer_pct))
    return {"disable": list(PROMOTED_SHORTS), "indicators": [copy.deepcopy(VWAP_1M)], "actions": acts}


def sizing_patch(thr=-1.5, mult=1.5):
    """indicator_tiered on SPY's session return: at/above the trigger x1.0 (the morning windows
    only fire with SPY inside +-0.2 %), below it x`mult` (the VWAP window's day filter)."""
    return {"disable": ["sizing_fixed"], "actions": [
        {"instance_id": "sizing_tiered_stress", "action_type": "indicator_tiered", "phase": "Sizing",
         "priority": 0, "enabled": True,
         "params": {"instance_id": SPY, "base_fraction": 0.36,
                    "tiers": [{"min": thr, "mult": 1.0}], "fallback_mult": mult}}]}


def tag(t, d, b):
    return f"{t}_d{str(d).replace('.', '')}_b{str(b).replace('.', '')}"


def main():
    if len(sys.argv) > 2 and sys.argv[1] == "--pipeline":
        pipeline_patch(sys.argv[3], sys.argv[2], sys.argv[4:])
        print(sys.argv[2])
        return
    out = {"am_only": cell()}   # scaffold (vwap_1m + scoped vwap_stop, no window): must equal iex_v18
    for t in TRIG:
        for d in (0.5, 0.8):
            for b in (0.3, 0.5):
                out[tag(t, d, b)] = cell(t, d, b)
                out[tag(t, d, b) + "_only"] = cell(t, d, b, am=False)        # pre-emption count
                out[tag(t, d, b) + "_multi"] = cell(t, d, b, first_retest=False)  # re-entries allowed
                out[tag(t, d, b) + "_gated"] = cell(t, d, b, priority=30)   # behind the noise gates
                out[tag(t, d, b) + "_h4"] = cell(t, d, b, max_hold_ms=14_400_000)
                out[tag(t, d, b) + "_cut1430"] = cell(t, d, b, entry_before="14:30")
                # rejection confirmation: the bar's high reached VWAP, the close is back below it
                out[tag(t, d, b) + "_rej"] = cell(t, d, b, band=(-0.15, 0.0), extra=[
                    {"type": "indicator_min", "instance_id": "vwap_1m.high_dist_pct", "min_score": -0.02}])
    # follow-ups: the VWAP stop effectively off (buffer 5 %: only the promoted 2.5 % hard stop,
    # breakeven, 3 h max hold and 15:55 remain)
    out["s05_d08_nostop"] = cell("s05", 0.8, 5.0)
    out["size_s15_x1.5"] = sizing_patch(-1.5, 1.5)
    out["size_s10_x1.5"] = sizing_patch(-1.0, 1.5)
    for k, v in out.items():
        with open(os.path.join(HERE, f"{k}.json"), "w") as f:
            json.dump(v, f, indent=1)
            f.write("\n")
    print(" ".join(sorted(out)))


if __name__ == "__main__":
    main()
