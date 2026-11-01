#!/usr/bin/env python3
"""frontier desk cell generator: one patch + one spec per cell, from a parameter dict.

    python3 research/lab/patches/frontier/make.py NAME key=value ...   # writes patch + spec
keys (defaults = v18 re-created as `_am` windows, the scaffold):
  am_end   entry_before of the fixed-band windows ("11:30" = v18 clock; "10:00" when a pm sqrt window exists)
  k        sqrt-band half-width for the 10:00..pm_end windows (0 = no pm windows)
  pm_end   end of the pm windows ("11:00")
  t1h_am / t1h_pm   thrust window 1h ceiling (v18 0.0)
  vpin     raw VPIN floor on every window (0.217)
  c5 / c1  strong-core 5m / 1h ceilings, all core windows (-0.4 / -0.4)
  core_am / core_pm  1 = keep the core window in that clock (1)
  band     fixed band half-width in cross_1m score units (0.4 = +-0.2 % SPY)
  tier     1.25 etc: size x tier at raw VPIN >= tier_min (0 = off); spec gets --max-position-pct base*tier
  tier_min 0.26
  pdl      prior-day-low exclusion: pdl_5m.dist_low_pct >= -pdl on every am/pm window (0 = off; adds weight-0 pdl_5m)
  se       score-exit threshold override on every am/pm window (0 = config's -0.30; -1.1 = never fires)
  noadx    1 = adx_14_1hr re-declared enabled=false (indicators desk finding; `disable` does not work on indicators)
  stress   SPY session ret <= -stress % strong-core window 09:45..15:30, hold 2 h (0 = off); opens the session
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
D = dict(am_end="11:30", k=0.0, pm_end="11:00", t1h_am=0.0, t1h_pm=0.0, vpin=0.217, c5=-0.4, c1=-0.4,
         core_am=1, core_pm=1, band=0.4, tier=0.0, tier_min=0.26, stress=0.0, pdl=0.0, se=0.0, noadx=0.0)


def thrust(t1h, filt):
    return [{"type": "composite_max", "max_score": -0.35},
            {"type": "timescale_lag", "lag_by": 0.1, "timescale": "FiveMinute"},
            {"type": "timescale_max", "max_score": -0.5, "timescale": "FiveMinute"},
            {"type": "timescale_max", "max_score": t1h, "timescale": "OneHour"}] + filt


def core(p, filt):
    return [{"type": "composite_max", "max_score": -0.35},
            {"type": "timescale_max", "max_score": p["c5"], "timescale": "FiveMinute"},
            {"type": "timescale_max", "max_score": p["c1"], "timescale": "OneHour"}] + filt


def win(iid, name, prio, conds, after=None, before=None, exits=None):
    params = {"name": name, "direction": "short", "conditions": conds}
    if after: params["entry_after"] = after
    if before: params["entry_before"] = before
    params["exit_overrides"] = dict(exits or {"force_exit_by": "11:55"})
    return {"instance_id": iid, "action_type": "entry_window", "phase": "Entry", "priority": prio,
            "enabled": True, "params": params}


def build(p):
    vp = {"type": "indicator_min", "instance_id": "vpin_1m.raw_vpin", "min_score": p["vpin"]}
    ex = {"force_exit_by": "11:55"}
    if p["se"] != 0:
        ex["score_exit_threshold"] = p["se"]
    pd = [{"type": "indicator_min", "instance_id": "pdl_5m.dist_low_pct", "min_score": -p["pdl"]}] if p["pdl"] > 0 else []
    band = pd + [{"type": "indicator_min", "instance_id": "cross_1m", "min_score": -p["band"]},
            {"type": "indicator_max", "instance_id": "cross_1m", "max_score": p["band"]}, vp]
    sq = pd + [{"type": "indicator_min", "instance_id": "cross_1m.index_session_ret_per_sqrt_min", "min_score": -p["k"]},
          {"type": "indicator_max", "instance_id": "cross_1m.index_session_ret_per_sqrt_min", "max_score": p["k"]}, vp]
    acts = [win("window_5m_thrust_short_am", "5m thrust short", 10, thrust(p["t1h_am"], band), before=p["am_end"], exits=ex)]
    if p["core_am"]:
        acts.append(win("window_strong_core_short_am", "strong core short", 20, core(p, band), before=p["am_end"], exits=ex))
    if p["k"] > 0:
        acts.append(win("window_5m_thrust_short_pm", "5m thrust short pm", 30, thrust(p["t1h_pm"], sq),
                        after=p["am_end"], before=p["pm_end"], exits=ex))
        if p["core_pm"]:
            acts.append(win("window_strong_core_short_pm", "strong core short pm", 40, core(p, sq),
                            after=p["am_end"], before=p["pm_end"], exits=ex))
    patch = {"disable": ["window_5m_thrust_short", "window_strong_core_short"], "actions": acts}
    args = []
    if p["pdl"] > 0:
        patch["indicators"] = [{"indicator_type": "prior_day_levels", "instance_id": "pdl_5m", "timescale": "FiveMinute",
                                "weight": 0.0, "enabled": True, "params": {"scale_pct": 0.005}}]
    if p["noadx"]:
        patch.setdefault("indicators", []).append({"instance_id": "adx_14_1hr", "indicator_type": "adx", "timescale": "OneHour",
                                                   "weight": 0.2, "enabled": False, "params": {"period": 14}})
    if p["stress"] > 0:
        acts.append(win("window_strong_core_stress", "strong core stress", 50,
                        [{"type": "composite_max", "max_score": -0.35},
                         {"type": "timescale_max", "max_score": -0.4, "timescale": "FiveMinute"},
                         {"type": "timescale_max", "max_score": -0.4, "timescale": "OneHour"}, vp,
                         {"type": "indicator_max", "instance_id": "cross_1m.index_session_ret", "max_score": -p["stress"]}],
                        after="09:45", before="15:30",
                        exits={"force_exit_by": "15:55", "max_hold_ms": 7200000, "score_exit_threshold": -0.6}))
        patch["session"] = {"no_new_entries_after": "15:30", "force_exit_by": "15:55"}
    if p["tier"] > 0:
        patch["disable"].append("sizing_fixed")
        acts.append({"instance_id": "sizing_tiered_vpin", "action_type": "indicator_tiered", "phase": "Sizing",
                     "priority": 0, "enabled": True,
                     "params": {"instance_id": "vpin_1m.raw_vpin", "base_fraction": 0.36,
                                "tiers": [{"min": p["tier_min"], "mult": p["tier"]}, {"min": 0.0, "mult": 1.0}],
                                "fallback_mult": 1.0}})
        args = ["--max-position-pct", str(round(0.36 * p["tier"] + 1e-9, 4))]
    return patch, args


def main():
    name = sys.argv[1]
    p = dict(D)
    for kv in sys.argv[2:]:
        k, v = kv.split("=", 1)
        p[k] = v if k in ("am_end", "pm_end") else float(v)
    patch, args = build(p)
    rel = f"research/lab/patches/frontier/{name}.json"
    (ROOT / rel).write_text(json.dumps(patch, indent=1) + "\n")
    diff = {k: v for k, v in p.items() if D[k] != v}
    spec = {"name": f"frontier/{name}", "config_id": 12, "patches": [rel], "args": args,
            "note": "frontier: " + (", ".join(f"{k}={v}" for k, v in diff.items()) or "scaffold (v18 as _am windows)"),
            "_params": diff}
    (ROOT / f"research/lab/specs/frontier/{name}.json").write_text(json.dumps(spec, indent=1) + "\n")
    print(name, args, diff)


if __name__ == "__main__":
    main()
