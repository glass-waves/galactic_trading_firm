#!/usr/bin/env python3
"""emit the entry-trigger study's --patch-json files (research/trigger/<cell>.json).

every cell = row 12 (v18) with its two promoted short windows disabled and re-added as `_am`
copies (conditions identical, own 11:30 / 11:55 clock — the stress / vwap_short scaffold), plus the
weight-0 `trig_1m` (trigger_context) indicator so every cell, the scaffold included, runs the same
indicator set. the SPY band (cross_1m in [-0.4, 0.4]) and the VPIN floor (vpin_1m.raw_vpin >= 0.217)
are in every window of every cell; only the trigger changes.

families (R = replaces a promoted window, A = adds a window on top of the unchanged two):
  a  thrust strength     R  composite ceiling on both windows; 5m ceiling on the thrust window
  b  confirmation        R  thrust window: no 1h condition / 1h <= -0.2 / 1m leading instead of 5m lagging
  c  pullback entry      R  thrust window replaced by "thrust in the last N bars, then a bounce of X"
                         A  the same pullback window added below the two promoted windows
  d  VPIN slope          A  both windows' triggers with VPIN in [0.17, 0.217) and rising
  e  time of day         R  per-window entry_after / entry_before on both windows

usage: python3 research/trigger/make_patches.py [--pipeline OUT CELL]   (writes every cell, prints the list;
       --pipeline turns a cell into the pipeline-format patch: same json, documented `session` untouched)
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
FILTERS = SPY_BAND + [VPIN]
TRIG = {"indicator_type": "trigger_context", "instance_id": "trig_1m", "timescale": "OneMinute",
        "weight": 0.0, "enabled": True, "params": {"lookback": 15}}


def comp(x):
    return {"type": "composite_max", "max_score": x}


def ts_max(ts, x):
    return {"type": "timescale_max", "max_score": x, "timescale": ts}


def lag5(x=0.1):
    return {"type": "timescale_lag", "lag_by": x, "timescale": "FiveMinute"}


def ind_min(i, x):
    return {"type": "indicator_min", "instance_id": i, "min_score": x}


def ind_max(i, x):
    return {"type": "indicator_max", "instance_id": i, "max_score": x}


def thrust_trigger(c=-0.35, m5=-0.5, h1=0.0, confirm="lag5", m1=None):
    t = [comp(c)]
    if m1 is not None:  # post-hoc (entry-score buckets of iex_v18): the 1m must still be pressing
        t.append(ts_max("OneMinute", m1))
    if confirm == "lag5":
        t.append(lag5())
    elif confirm == "lead1m":  # the literal reading: 1m >= every other timescale + 0.1
        t.append({"type": "timescale_lead", "lead_by": 0.1, "timescale": "OneMinute"})
    elif confirm == "lag1m":   # 1m the most bearish timescale by 0.1 (1m leads the move down)
        t.append({"type": "timescale_lag", "lag_by": 0.1, "timescale": "OneMinute"})
    t.append(ts_max("FiveMinute", m5))
    if h1 is not None:
        t.append(ts_max("OneHour", h1))
    return t


def core_trigger(c=-0.35, m5=-0.4, h1=-0.4):
    return [comp(c), ts_max("FiveMinute", m5), ts_max("OneHour", h1)]


def window(iid, name, prio, conds, after=None, before="11:30"):
    p = {"name": name, "direction": "short", "conditions": conds}
    if after:
        p["entry_after"] = after
    if before:
        p["entry_before"] = before
    p["exit_overrides"] = {"force_exit_by": "11:55"}
    return {"instance_id": iid, "action_type": "entry_window", "phase": "Entry",
            "priority": prio, "enabled": True, "params": p}


def thrust_w(after=None, before="11:30", **kw):
    return window("window_5m_thrust_short_am", "5m thrust short", 10, thrust_trigger(**kw) + FILTERS, after, before)


def core_w(after=None, before="11:30", **kw):
    return window("window_strong_core_short_am", "strong core short", 20, core_trigger(**kw) + FILTERS, after, before)


def pullback_trigger(drop=0.5, rmin=0.3, rmax=0.7, bmin=2, bmax=12, m5=-0.4, c=-0.2, h1=0.0, resume=False):
    """a thrust of >= `drop` % within today's last 15 bars whose low is 2..12 bars old, price back
    `rmin`..`rmax` of the way to where the thrust started, the 5m still bearish (thrust state)."""
    t = [comp(c), ts_max("FiveMinute", m5)]
    if h1 is not None:
        t.append(ts_max("OneHour", h1))
    t += [ind_min("trig_1m.drop_pct", drop),
          ind_min("trig_1m.bars_since_low", bmin), ind_max("trig_1m.bars_since_low", bmax),
          ind_min("trig_1m.retrace", rmin), ind_max("trig_1m.retrace", rmax)]
    if resume:  # enter the first red bar of the resumption, not the bounce bar
        t.append(ind_max("trig_1m.last_ret_pct", -0.01))
    return t


def pullback_w(prio=10, **kw):
    return window("window_pullback_short_am", "pullback short", prio, pullback_trigger(**kw) + FILTERS)


def slope_windows(lo=0.17, slope_key="vpin_slope_5", slope=0.02, core=True, **thrust_kw):
    band = [ind_min("vpin_1m.raw_vpin", lo), ind_max("vpin_1m.raw_vpin", 0.2169999)]
    if slope_key:
        band.append(ind_min(f"trig_1m.{slope_key}", slope))
    w = [window("window_thrust_slope_short", "thrust vpin-slope short", 30, thrust_trigger(**thrust_kw) + SPY_BAND + band)]
    if core:
        w.append(window("window_core_slope_short", "core vpin-slope short", 40, core_trigger() + SPY_BAND + band))
    return w


def cell(actions):
    return {"disable": list(PROMOTED_SHORTS), "indicators": [copy.deepcopy(TRIG)], "actions": actions}


def cells():
    out = {}
    out["am"] = cell([thrust_w(), core_w()])                                  # scaffold == iex_v18
    # a. thrust strength
    for c in (-0.30, -0.40, -0.45):
        out[f"a_c{abs(c) * 100:.0f}"] = cell([thrust_w(c=c), core_w(c=c)])
    for m5 in (-0.6, -0.7):
        out[f"a_t{abs(m5) * 100:.0f}"] = cell([thrust_w(m5=m5), core_w()])
    # b. confirmation timescale (thrust window only)
    out["b_no1h"] = cell([thrust_w(h1=None), core_w()])
    for h in (-0.05, -0.1, -0.15, -0.2):
        out[f"b_1h{abs(h) * 100:02.0f}"] = cell([thrust_w(h1=h), core_w()])
    out["b_lead1m"] = cell([thrust_w(confirm="lead1m"), core_w()])
    out["b_1m40"] = cell([thrust_w(m1=-0.4), core_w()])
    out["b_lag1m"] = cell([thrust_w(confirm="lag1m"), core_w()])
    # c. pullback entry
    out["c_pb"] = cell([pullback_w(), core_w()])                              # R: thrust -> pullback
    out["c_pb_add"] = cell([thrust_w(), core_w(), pullback_w(prio=30)])      # A: pullback below both
    out["c_pb_resume"] = cell([pullback_w(resume=True), core_w()])
    out["c_pb_r2"] = cell([pullback_w(rmin=0.2, rmax=0.6), core_w()])
    out["c_pb_d03"] = cell([pullback_w(drop=0.3), core_w()])
    # d. VPIN slope (A)
    out["d_s5"] = cell([thrust_w(), core_w()] + slope_windows())
    out["d_s5_03"] = cell([thrust_w(), core_w()] + slope_windows(slope=0.03))
    out["d_s3_02"] = cell([thrust_w(), core_w()] + slope_windows(slope_key="vpin_slope_3", slope=0.02))
    out["d_s5_l15"] = cell([thrust_w(), core_w()] + slope_windows(lo=0.15))
    out["d_lvl"] = cell([thrust_w(), core_w()] + slope_windows(slope_key=None))   # control: no slope condition
    # combination of the two deepened families: thrust window without 1h + the slope addition
    out["bd_no1h_s5"] = cell([thrust_w(h1=None), core_w()] + slope_windows(h1=None))
    # e. time of day (both windows)
    for a, b in (("09:45", "11:00"), ("10:00", "11:30"), ("09:45", "11:30"), ("09:30", "11:00")):
        out[f"e_{a.replace(':', '')}_{b.replace(':', '')}"] = cell([thrust_w(after=a, before=b), core_w(after=a, before=b)])
    return out


def main():
    if len(sys.argv) > 3 and sys.argv[1] == "--pipeline":
        src = json.load(open(os.path.join(HERE, f"{sys.argv[3]}.json")))
        with open(sys.argv[2], "w") as f:
            json.dump(src, f, indent=1)
            f.write("\n")
        print(sys.argv[2])
        return
    out = cells()
    only = set(sys.argv[1:])
    for k, v in out.items():
        if only and k not in only:
            continue
        with open(os.path.join(HERE, f"{k}.json"), "w") as f:
            json.dump(v, f, indent=1)
            f.write("\n")
    print("\n".join(sorted(k for k in out if not only or k in only)))


if __name__ == "__main__":
    main()
