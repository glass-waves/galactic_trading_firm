#!/usr/bin/env python3
"""long-2026 desk: generates patches + specs. run from repo root: python3 research/lab/patches/long-2026/make_cells.py
building blocks: row 12's disabled long windows, re-enabled; a directional 1h added as weight-0 instances
(st1h_std = supertrend standard, adx1h_s = signed ADX) so the scoring sums (and v18's shorts) are untouched;
short windows = v18 copies plus the mirror of the long-side filter (the "short twin")."""
import json, copy, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[4]
P = ROOT / "research/lab/patches/long-2026"; S = ROOT / "research/lab/specs/long-2026"

def ind(iid, typ, ts, params):
    return {"instance_id": iid, "indicator_type": typ, "timescale": ts, "weight": 0.0, "enabled": True, "params": params}
IND = [ind("st1h_std", "supertrend", "OneHour", {"period": 10, "multiplier": 3.0, "standard": True}),
       ind("adx1h_s", "adx", "OneHour", {"period": 14, "signed": True}),
       ind("gap_1m", "gap", "OneMinute", {"scale_pct": 1.0})]
DIR1H_GLOBAL = json.loads((ROOT / "research/dir1h/dir1h.patch.json").read_text())["indicators"]

def c_min(i, v): return {"type": "indicator_min", "instance_id": i, "min_score": v}
def c_max(i, v): return {"type": "indicator_max", "instance_id": i, "max_score": v}
def ts_min(t, v): return {"type": "timescale_min", "timescale": t, "min_score": v}
def ts_max(t, v): return {"type": "timescale_max", "timescale": t, "max_score": v}

LONG = {  # row 12, verbatim conditions
 "thrust": ("window_5m_thrust", "5m thrust", 10, [{"type": "composite_min", "min_score": 0.35}, {"type": "timescale_lead", "lead_by": 0.1, "timescale": "FiveMinute"}, ts_min("FiveMinute", 0.5), ts_min("OneHour", 0.0)]),
 "candle": ("window_candle_reversal", "candle reversal", 15, [c_min("candle_5min", 0.4), ts_min("OneHour", 0.2), {"type": "composite_min", "min_score": 0.3}]),
 "core":   ("window_strong_core", "strong core", 20, [{"type": "composite_min", "min_score": 0.35}, ts_min("FiveMinute", 0.4), ts_min("OneHour", 0.3)]),
}
BAND = [c_min("cross_1m", -0.4), c_max("cross_1m", 0.4)]
VPIN = [c_min("vpin_1m.raw_vpin", 0.217)]
SHORT = {
 "thrust": ("window_5m_thrust_short", "5m thrust short", 11, [{"type": "composite_max", "max_score": -0.35}, {"type": "timescale_lag", "lag_by": 0.1, "timescale": "FiveMinute"}, ts_max("FiveMinute", -0.5), ts_max("OneHour", 0.0)]),
 "core":   ("window_strong_core_short", "strong core short", 21, [{"type": "composite_max", "max_score": -0.35}, ts_max("FiveMinute", -0.4), ts_max("OneHour", -0.4)]),
}

def win(iid, name, prio, direction, conds, clock):
    p = {"name": name, "direction": direction, "conditions": conds}
    if clock and clock[0]: p["entry_after"] = clock[0]
    if clock and clock[1]: p["entry_before"] = clock[1]
    return {"instance_id": iid, "action_type": "entry_window", "phase": "Entry", "priority": prio, "enabled": True, "params": p}

def cell(name, note, long_extra=(), short_extra=(), longs=("thrust", "candle", "core"), shorts="mirror",
         short_band=True, short_vpin=True, clock=None, dir1h_global=False, tickers=None, per_window_long=None):
    patch = {"indicators": copy.deepcopy(IND + (DIR1H_GLOBAL if dir1h_global else [])), "actions": [], "disable": []}
    for k, (iid, nm, pr, conds) in LONG.items():
        if k in longs:
            extra = list(long_extra) + list((per_window_long or {}).get(k, []))
            patch["actions"].append(win(iid, nm, pr, "long", copy.deepcopy(conds) + extra, clock))
    for k, (iid, nm, pr, conds) in SHORT.items():
        if shorts == "none":
            patch["disable"].append(iid)
        elif shorts == "v18":
            pass  # promoted windows untouched
        else:
            filt = (BAND if short_band else []) + (VPIN if short_vpin else [])
            patch["actions"].append(win(iid, nm, pr, "short", copy.deepcopy(conds) + filt + list(short_extra), clock))
    (P / f"{name}.json").write_text(json.dumps(patch, indent=1))
    spec = {"name": f"long-2026/{name}", "config_id": 12, "patches": [f"research/lab/patches/long-2026/{name}.json"], "note": note}
    if tickers: spec["tickers"] = tickers
    (S / f"{name}.json").write_text(json.dumps(spec, indent=1))

BULL = [c_min("st1h_std.is_bullish", 0.5)]; BEAR = [c_max("st1h_std.is_bullish", 0.5)]
def adx_l(k): return [c_min("adx1h_s", k)]
def adx_s(k): return [c_max("adx1h_s", -k)]
SPY_UP = [c_min("cross_1m.index_session_ret", 0.0)]; SPY_DN = [c_max("cross_1m.index_session_ret", 0.0)]
SPY_PC_UP = [c_min("cross_1m.index_ret_prior_close", 0.0)]; SPY_PC_DN = [c_max("cross_1m.index_ret_prior_close", 0.0)]
GAP_UP = [c_min("gap_1m.gap_pct", 0.0)]; GAP_DN = [c_max("gap_1m.gap_pct", 0.0)]

# --- scaffold: v18 + the weight-0 instances, nothing else -> must equal base/v18 trade for trade
cell("s0_scaffold", "scaffold: v18 + weight-0 st1h_std/adx1h_s/gap_1m, long windows still disabled; must equal base/v18", longs=(), shorts="v18")
# --- block A: what the 1h means (mirrored books: long side | short twin)
cell("a0_legacy", "row-12 long windows re-enabled as-is (legacy 1h) | shorts = v18", shorts="v18")
cell("a1_bull", "longs + 1h supertrend(standard) bullish | shorts + bearish", BULL, BEAR)
cell("a2_bull_adx20", "longs + st bullish + signed ADX >= 0.2 | shorts mirror", BULL + adx_l(0.2), BEAR + adx_s(0.2))
cell("a3_bull_adx40", "longs + st bullish + signed ADX >= 0.4 | shorts mirror", BULL + adx_l(0.4), BEAR + adx_s(0.4))
cell("a4_dir1h_global", "dir1h patch applied to the 1h sum (both sides), windows as in row 12 | shorts v18 windows", dir1h_global=True)
if __name__ == "__main__" and len(sys.argv) > 1:
    exec(Path(sys.argv[1]).read_text())  # later blocks live in blocks_*.py so this file stays a record of block A
print("ok")
