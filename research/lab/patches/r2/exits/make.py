"""generate the r2-exits patches + specs from row 25's blob (dumped to argv[1])."""
import copy, json, sys, os
blob = json.load(open(sys.argv[1]))
A = {a["instance_id"]: a for a in blob["actions"]}
P = "research/lab/patches/r2/exits"; S = "research/lab/specs/r2/exits"
AM = ["window_5m_thrust_short_am", "window_strong_core_short_am"]
PM = ["window_5m_thrust_short_pm", "window_strong_core_short_pm"]
ST = "window_strong_core_stress"

def act(iid, **params):
    a = copy.deepcopy(A[iid]); a["params"].update(params); return a

def ovr(iid, **o):
    a = copy.deepcopy(A[iid]); eo = a["params"].setdefault("exit_overrides", {})
    for k, v in o.items():
        if v is None: eo.pop(k, None)
        else: eo[k] = v
    return a

NEUTRAL = [ovr(w, max_hold_ms=1) for w in AM] + [ovr(w, max_hold_ms=2_400_000) for w in PM]
def vw(scope, buf=0.2):
    return {"phase": "Exit", "params": {"buffer_pct": buf, "scope_min_max_hold_ms": scope}, "enabled": True,
            "priority": 3, "action_type": "vwap_stop", "instance_id": "vwap_stop"}

cells = {
 "scaffold_ovr": ({"actions": NEUTRAL}, "neutral overrides am 1 ms / pm 2.4 M — must equal base"),
 "mh60": ({"actions": [act("max_hold", max_hold_ms=3_600_000, loss_reduction_ms=1_200_000)]}, "max hold 60/40"),
 "mh120": ({"actions": [act("max_hold", max_hold_ms=7_200_000, loss_reduction_ms=4_800_000)]}, "max hold 120/40"),
 "lr30": ({"actions": [act("max_hold", loss_reduction_ms=3_600_000)]}, "max hold 90/30"),
 "lr60": ({"actions": [act("max_hold", loss_reduction_ms=1_800_000)]}, "max hold 90/60"),
 "lr0": ({"actions": [act("max_hold", loss_reduction_ms=0)]}, "max hold 90/90"),
 "pm_hold90": ({"actions": [ovr(w, max_hold_ms=5_340_001) for w in PM]}, "pm windows: losers held to 90 min"),
 "pm_fx1230": ({"actions": [ovr(w, force_exit_by="12:30") for w in PM]}, "pm windows force exit 12:30"),
 "st_mh90": ({"actions": [ovr(ST, max_hold_ms=5_340_001)]}, "stress max hold 90 min"),
 "st_mh180": ({"actions": [ovr(ST, max_hold_ms=10_800_000)]}, "stress max hold 3 h"),
 "st_seoff": ({"actions": [ovr(ST, score_exit_threshold=-1.1)]}, "stress score exit off"),
 "st_se03": ({"actions": [ovr(ST, score_exit_threshold=None)]}, "stress score exit at the default -0.3"),
 "tr5": ({"actions": [act("trailing_stop_atr", multiplier=5.0)]}, "ATR trail 5x"),
 "troff": ({"disable": ["trailing_stop_atr"]}, "ATR trail off"),
 "hs15": ({"actions": [act("hard_stop", stop_loss_pct=0.015)]}, "hard stop 1.5%"),
 "be03": ({"actions": [act("breakeven", trigger_pct=0.003, breakeven_trigger_pct=0.003)]}, "breakeven 0.3%"),
 "be08": ({"actions": [act("breakeven", trigger_pct=0.008, breakeven_trigger_pct=0.008)]}, "breakeven 0.8%"),
 "beoff": ({"disable": ["breakeven"]}, "breakeven off"),
 "vw_st": ({"actions": [vw(7_200_000)]}, "VWAP stop 0.2% on stress only"),
 "vw_pm": ({"actions": NEUTRAL + [vw(2)]}, "VWAP stop 0.2% on pm + stress (neutral overrides scope it)"),
 "vw_all": ({"actions": [vw(0)]}, "VWAP stop 0.2% on every window"),
 "rq14_p1": ({"actions": [act("max_hold", max_hold_ms=5_460_000), ovr(ST, max_hold_ms=7_260_000)]}, "RQ-14: max hold +1 bar (91/41, stress 2h01)"),
 "rq14_m1": ({"actions": [act("max_hold", max_hold_ms=5_340_000), ovr(ST, max_hold_ms=7_140_000)]}, "RQ-14: max hold -1 bar (89/39, stress 1h59)"),
}
for name, (patch, note) in cells.items():
    json.dump(patch, open(f"{P}/{name}.json", "w"), indent=1)
    json.dump({"name": f"r2/exits/{name}", "config_id": 25, "patches": [f"{P}/{name}.json"],
               "args": ["--max-position-pct", "0.45"], "note": note}, open(f"{S}/{name}.json", "w"), indent=1)
print(len(cells))
