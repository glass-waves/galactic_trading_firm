#!/usr/bin/env python3
"""build a --patch-json for one entry-research variant (round two, 2026-09-12).

usage:
  make_variant.py --out FILE --mode standalone|condition|both \
      --cond INSTANCE:OP:THRESHOLD [--cond ...] [--indicator ID=TYPE@TIMESCALE:PARAMS_JSON ...]

  OP is max (score <= THRESHOLD) or min (score >= THRESHOLD). INSTANCE may be an indicator
  instance id or an `{id}.{meta}` key.

modes
  standalone  disable v16's two short windows; add ONE short window whose conditions are the
              --cond list (plus composite_max 1.0 so the window type validates).
  condition   disable v16's two short windows; re-add both with the --cond list appended.
  both        the two v16 windows with the conditions appended AND the standalone window.

the indicator instances referenced must exist in the promoted config or be added with
--indicator (weight 0). research/entries/screen_indicators.json can be passed to the backtest
as a second --patch-json instead of listing them here.
"""
import argparse
import json
import sys

V16_WINDOWS = [
    {
        "instance_id": "window_5m_thrust_short",
        "action_type": "entry_window",
        "phase": "Entry",
        "priority": 10,
        "enabled": True,
        "params": {
            "name": "5m thrust short",
            "direction": "short",
            "conditions": [
                {"type": "composite_max", "max_score": -0.35},
                {"type": "timescale_lag", "lag_by": 0.10, "timescale": "FiveMinute"},
                {"type": "timescale_max", "max_score": -0.50, "timescale": "FiveMinute"},
                {"type": "timescale_max", "max_score": 0.0, "timescale": "OneHour"},
            ],
        },
    },
    {
        "instance_id": "window_strong_core_short",
        "action_type": "entry_window",
        "phase": "Entry",
        "priority": 20,
        "enabled": True,
        "params": {
            "name": "strong core short",
            "direction": "short",
            "conditions": [
                {"type": "composite_max", "max_score": -0.35},
                {"type": "timescale_max", "max_score": -0.40, "timescale": "FiveMinute"},
                {"type": "timescale_max", "max_score": -0.40, "timescale": "OneHour"},
            ],
        },
    },
]


def parse_cond(spec: str) -> dict:
    inst, op, thr = spec.split(":")
    thr = float(thr)
    if op == "max":
        return {"type": "indicator_max", "instance_id": inst, "max_score": thr}
    if op == "min":
        return {"type": "indicator_min", "instance_id": inst, "min_score": thr}
    raise SystemExit(f"bad op in {spec}: use max|min")


def parse_indicator(spec: str) -> dict:
    ident, rest = spec.split("=", 1)
    type_ts, params = rest.split(":", 1) if ":" in rest else (rest, "{}")
    itype, ts = type_ts.split("@")
    return {
        "indicator_type": itype,
        "instance_id": ident,
        "timescale": ts,
        "weight": 0.0,
        "enabled": True,
        "params": json.loads(params),
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--mode", choices=["standalone", "condition", "both"], required=True)
    ap.add_argument("--cond", action="append", default=[], help="INSTANCE:max|min:THRESHOLD")
    ap.add_argument("--indicator", action="append", default=[], help="ID=TYPE@TIMESCALE:PARAMS_JSON")
    ap.add_argument("--name", default=None, help="standalone window name")
    ap.add_argument("--composite-max", type=float, default=1.0, help="composite ceiling for the standalone window")
    a = ap.parse_args()
    conds = [parse_cond(c) for c in a.cond]
    if not conds:
        sys.exit("need at least one --cond")

    patch = {"disable": ["window_5m_thrust_short", "window_strong_core_short"], "indicators": [], "actions": []}
    patch["indicators"] = [parse_indicator(s) for s in a.indicator]

    if a.mode in ("condition", "both"):
        for w in V16_WINDOWS:
            w2 = json.loads(json.dumps(w))
            w2["instance_id"] += "_x"
            w2["params"]["name"] += " +cond"
            w2["params"]["conditions"].extend(conds)
            patch["actions"].append(w2)
    if a.mode in ("standalone", "both"):
        name = a.name or "research: " + " & ".join(a.cond)
        patch["actions"].append(
            {
                "instance_id": "window_research_short",
                "action_type": "entry_window",
                "phase": "Entry",
                "priority": 30,
                "enabled": True,
                "params": {
                    "name": name,
                    "direction": "short",
                    "conditions": [{"type": "composite_max", "max_score": a.composite_max}] + conds,
                },
            }
        )
    with open(a.out, "w") as f:
        json.dump(patch, f, indent=1)
    print(f"wrote {a.out}: mode={a.mode} conds={a.cond} actions={[x['instance_id'] for x in patch['actions']]}")


if __name__ == "__main__":
    main()
