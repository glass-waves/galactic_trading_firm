#!/usr/bin/env python3
"""emit research/trend_day/trend_day_v1.patch.json (pipeline-format --patch-json).

candidate "trend-day-ride": from the crash-days hindsight study (research/crash_days/
2026-09-27_crash_days_hindsight.md) — SPY down >= 1.0% from its open fires ~4x/year (60% of
those days close < -1%), the session low comes after 14:00 on ~90% of tail days and the close
sits near the low (trend days, not mean-reverting). The vwap-anchored-short study's retest entry
never fires on the days that matter (research/vwap_short/2026-09-27_vwap_anchored_short.md) — this
candidate skips the retest and just rides the trend to the close.

base = v18 unchanged (config_versions row 12): both promoted short windows disabled and re-added
as `_am` copies with their own 11:30/11:55 clocks (`stress` study's `am_windows()`, identical
conditions to row 12 — verified against the live blob). new window `window_trend_day_short`
(priority 30, short, 10:15-14:00 ET):
  1. SPY already down >= 1.0% from today's open: cross_1m.index_session_ret <= -1.0
  2. the name is below its own session VWAP: vwap_1m.dist_pct <= -0.2 (vwap_distance, OneMinute,
     weight 0 -- not in row 12, added here)
  3. the name is underperforming SPY since the prior close: pdl_5m.rel_close_pct <= -0.5
     (prior_day_levels, FiveMinute, weight 0, scale_pct 0.005 -- not in row 12, added here)
  4. composite_max 0.0 -- non-positive composite (no separate strength requirement; the SPY/VWAP/
     relative-weakness conditions above do the selecting)
exit_overrides: force_exit_by 15:55, max_hold_ms 6h (21,600,000 ms), score_exit_threshold -10
(effectively off) -- rides to the close with only the promoted hard stop (2.5%) and breakeven
monitor as guards, same as every other window in row 12.

usage: python3 research/trend_day/make_patch.py
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "stress"))
from make_patches import PROMOTED_SHORTS, am_windows  # noqa: E402  (stress study)

VWAP_1M = {"indicator_type": "vwap_distance", "instance_id": "vwap_1m", "timescale": "OneMinute",
           "weight": 0.0, "enabled": True, "params": {}}
PDL_5M = {"indicator_type": "prior_day_levels", "instance_id": "pdl_5m", "timescale": "FiveMinute",
          "weight": 0.0, "enabled": True, "params": {"scale_pct": 0.005}}


def trend_day_window():
    return {
        "instance_id": "window_trend_day_short",
        "action_type": "entry_window",
        "phase": "Entry",
        "priority": 30,
        "enabled": True,
        "params": {
            "name": "trend day short",
            "direction": "short",
            "conditions": [
                {"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -1.0},
                {"type": "indicator_max", "instance_id": "vwap_1m.dist_pct", "max_score": -0.2},
                {"type": "indicator_max", "instance_id": "pdl_5m.rel_close_pct", "max_score": -0.5},
                {"type": "composite_max", "max_score": 0.0},
            ],
            "entry_after": "10:15",
            "entry_before": "14:00",
            "exit_overrides": {
                "force_exit_by": "15:55",
                "max_hold_ms": 21_600_000,
                "score_exit_threshold": -10.0,
            },
        },
    }


def main():
    patch = {
        "disable": list(PROMOTED_SHORTS),
        "indicators": [VWAP_1M, PDL_5M],
        "actions": am_windows() + [trend_day_window()],
        "session": {"no_new_entries_after": "15:30", "force_exit_by": "15:55"},
    }
    out = os.path.join(HERE, "trend_day_v1.patch.json")
    with open(out, "w") as f:
        json.dump(patch, f, indent=1)
        f.write("\n")
    print(out)


if __name__ == "__main__":
    main()
