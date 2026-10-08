#!/usr/bin/env python3
"""emit the time-consistent-filter study's --patch-json files (research/timefilter/*.json).

scaffold (`am`): v18's two promoted short windows disabled and re-added as the stress study's
`_am` copies (conditions identical to config_versions row 12, own 11:30 / 11:55 clock), the
session opened with the `session` key (no_new_entries_after 15:30, force_exit_by 15:55), plus a
weight-0 `vpin_p` (vpin, pctile_window 1950) for the diagnosis dump. must equal iex_v18 436/436.

every other cell replaces the two `_am` windows with copies whose market filter and/or clock
differ (see CELLS below); trigger conditions (thrust / strong core), the VPIN floor unless stated,
priorities 10 / 20 and the promoted exit stack are unchanged.

usage: python3 research/timefilter/make_patches.py [cell ...]   (no args: every cell)
       python3 research/timefilter/make_patches.py --pipeline CELL OUT.json   (pipeline-format patch)
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "stress"))
from make_patches import CORE_CORE, PROMOTED_SHORTS, SPY_BAND, THRUST_CORE, VPIN, window  # noqa: E402

SESSION = {"no_new_entries_after": "15:30", "force_exit_by": "15:55"}
VPIN_P = {"indicator_type": "vpin", "instance_id": "vpin_p", "timescale": "OneMinute", "weight": 0.0,
          "enabled": True, "params": {"bucket_count": 20, "pctile_window": 1950}}


def rng(inst, lo, hi):
    return [{"type": "indicator_min", "instance_id": inst, "min_score": lo},
            {"type": "indicator_max", "instance_id": inst, "max_score": hi}]


def pair(filt, vpin=(VPIN,), after=None, before="11:30", fexit="11:55", suffix="am", prio=(10, 20),
         thrust_core=THRUST_CORE, overrides=None):
    """the two promoted windows with market filter `filt` and VPIN conditions `vpin`."""
    ov = {"force_exit_by": fexit}
    ov.update(overrides or {})
    f, v = list(filt), list(vpin)
    tail = "" if suffix == "am" else f" {suffix}"   # entry_reason names: `_am` keeps v18's
    return [
        window(f"window_5m_thrust_short_{suffix}", "5m thrust short" + tail, prio[0], thrust_core + f + v,
               after, before, copy.deepcopy(ov)),
        window(f"window_strong_core_short_{suffix}", "strong core short" + tail, prio[1], CORE_CORE + f + v,
               after, before, copy.deepcopy(ov)),
    ]


def cell(actions, indicators=()):
    return {"disable": list(PROMOTED_SHORTS), "indicators": [copy.deepcopy(VPIN_P)] + list(indicators),
            "actions": actions, "session": dict(SESSION)}


def thrust_1h(x):
    """the thrust window with its hourly ceiling at `x` (v18: 0; candidate #29 thrust-1h15: -0.15)."""
    return [dict(c, max_score=x) if c.get("timescale") == "OneHour" else c for c in THRUST_CORE]


THRUST_1H15 = thrust_1h(-0.15)
SQM = "cross_1m.index_session_ret_per_sqrt_min"   # SPY session return % / sqrt(min since 09:30)
R15 = "cross_1m.index_ret_15m"                      # SPY 15-min return %, 0 before 09:45
LATE = {"before": "15:30", "fexit": "15:55"}


def sq(k):
    return rng(SQM, -k, k)


def r15(x):
    return rng(R15, -x, x)


def split(open_filt, later_filt, cut="10:00", before="11:30", fexit="11:55", **kw):
    """(c): one pair with `open_filt` until `cut`, a second pair with `later_filt` from `cut`."""
    return (pair(open_filt, before=cut, fexit=fexit, **kw)
            + pair(later_filt, after=cut, before=before, fexit=fexit, suffix="pm", prio=(30, 40), **kw))


CELLS = {
    "am": lambda: cell(pair(SPY_BAND)),
    # 4. session extension with v18's own filters
    "x1300": lambda: cell(pair(SPY_BAND, before="13:00", fexit="15:55")),
    "x1530": lambda: cell(pair(SPY_BAND, **LATE)),
    # 2a. rolling SPY return instead of the session band
    "r15_15": lambda: cell(pair(r15(0.15))),
    # 2b. time-scaled band (k = 0.05: equals +-0.2 % at 09:45)
    "sq05": lambda: cell(pair(sq(0.05))),
    "sq05_x1530": lambda: cell(pair(sq(0.05), **LATE)),
    # 2c. session band to 10:00, rolling band after
    "c_r10": lambda: cell(split(SPY_BAND, r15(0.10))),
    # deepen the time-scaled band: the k dial, v18's band at the open + k after 10:00, the 1h dial
    # (thrust window 1h <= -0.15, candidate #29), the 11:00 tail, the clock to 13:00
    "sq04": lambda: cell(pair(sq(0.04))),
    "sq06": lambda: cell(pair(sq(0.06))),
    "c_sq05": lambda: cell(split(SPY_BAND, sq(0.05))),
    "sq05_1h15": lambda: cell(pair(sq(0.05), thrust_core=THRUST_1H15)),
    "sq05_1100": lambda: cell(pair(sq(0.05), before="11:00")),
    "sq05_x1300": lambda: cell(pair(sq(0.05), before="13:00", fexit="15:55")),
    # the 11:00 clock (entry-trigger study §3e: v18's 11:00-11:30 tail is flat) across the k dial,
    # and with v18's own band kept before 10:00
    "sq04_1100": lambda: cell(pair(sq(0.04), before="11:00")),
    "sq06_1100": lambda: cell(pair(sq(0.06), before="11:00")),
    "c_sq05_1100": lambda: cell(split(SPY_BAND, sq(0.05), before="11:00")),
    # live timing: identical patches, swept with `--cross-lag 1` (SPY state one minute late)
    "am_lag1": lambda: cell(pair(SPY_BAND)),
    "c_sq05_1100_lag1": lambda: cell(split(SPY_BAND, sq(0.05), before="11:00")),
    # the 1h dial between v18 (0) and #29 (-0.15) on top of the time-scaled band
    "sq05_1h05": lambda: cell(pair(sq(0.05), thrust_core=thrust_1h(-0.05))),
    "sq05_1h10": lambda: cell(pair(sq(0.05), thrust_core=thrust_1h(-0.10))),
}


def pipeline_patch(cell_name, out_file):
    """the pipeline-format patch for a swept cell: its disable list and windows only. the weight-0
    `vpin_p` (diagnosis only) and the opened session are dropped: every window carries its own
    clock inside v18's 11:30 / 11:55 session, so the trades are the same (verify_patch.sh)."""
    c = CELLS[cell_name]()
    p = {"disable": c["disable"], "actions": c["actions"]}
    with open(out_file, "w") as fh:
        json.dump(p, fh, indent=1)
        fh.write("\n")


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "--pipeline":
        pipeline_patch(sys.argv[2], sys.argv[3])     # make_patches.py --pipeline CELL OUT.json
        print(sys.argv[3])
        return
    names = sys.argv[1:] or list(CELLS)
    for k in names:
        with open(os.path.join(HERE, f"{k}.json"), "w") as fh:
            json.dump(CELLS[k](), fh, indent=1)
            fh.write("\n")
    print(" ".join(names))


if __name__ == "__main__":
    main()
