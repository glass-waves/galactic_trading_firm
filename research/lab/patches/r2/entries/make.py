#!/usr/bin/env python3
"""r2-entries cell generator: self-contained patches on config row 25.

    python3 research/lab/patches/r2/entries/make.py            # (re)write every cell
Reads row 25's blob via scripts/psql.sh, re-declares the windows a cell touches with full params
(replace in place by instance_id) and appends weight-0 helper indicators. Writes
research/lab/patches/r2/entries/<cell>.json and research/lab/specs/r2/entries/<cell>.json.
"""
import copy, json, pathlib, subprocess

ROOT = pathlib.Path(__file__).resolve().parents[5]
PDIR = ROOT / "research/lab/patches/r2/entries"
SDIR = ROOT / "research/lab/specs/r2/entries"

blob = json.loads(subprocess.check_output(
    ["./scripts/psql.sh", "-tA", "-c", "select config_blob from config_versions where id=25"], cwd=ROOT))
ACT = {a["instance_id"]: a for a in blob["actions"]}
AM = ["window_5m_thrust_short_am", "window_strong_core_short_am"]
PM = ["window_5m_thrust_short_pm", "window_strong_core_short_pm"]
ST = ["window_strong_core_stress"]
ALL = AM + PM + ST

def ind(t, iid, ts, params=None):
    return {"indicator_type": t, "instance_id": iid, "timescale": ts, "weight": 0.0, "enabled": True,
            "params": params or {}}

HELPERS = {
    "pdl_5m": ind("prior_day_levels", "pdl_5m", "FiveMinute", {"scale_pct": 0.005}),
    "vwapd_1m": ind("vwap_distance", "vwapd_1m", "OneMinute"),
    "rvol_5m": ind("relative_volume", "rvol_5m", "FiveMinute", {"lookback_period": 20}),
    "gap_5m": ind("gap", "gap_5m", "FiveMinute"),
}

def cmin(iid, v): return {"type": "indicator_min", "min_score": v, "instance_id": iid}
def cmax(iid, v): return {"type": "indicator_max", "max_score": v, "instance_id": iid}

class Cell:
    def __init__(self, name, note):
        self.name, self.note = name, note
        self.acts = {}
        self.inds = set()
    def w(self, iid):
        if iid not in self.acts:
            self.acts[iid] = copy.deepcopy(ACT[iid])
        return self.acts[iid]["params"]
    def add_cond(self, ids, cond, helper=None):
        for i in ids:
            self.w(i)["conditions"].append(copy.deepcopy(cond))
        if helper:
            self.inds.add(helper)
    def set_cond(self, ids, ctype, key, val, ts=None, iid=None):
        """replace the value of the matching existing condition"""
        for i in ids:
            hit = 0
            for c in self.w(i)["conditions"]:
                if c["type"] == ctype and (ts is None or c.get("timescale") == ts) and (iid is None or c.get("instance_id") == iid):
                    c[key] = val; hit += 1
            assert hit == 1, (self.name, i, ctype, hit)
    def write(self):
        patch = {"indicators": [HELPERS[h] for h in sorted(self.inds)], "actions": list(self.acts.values())}
        (PDIR / f"{self.name}.json").write_text(json.dumps(patch, indent=1) + "\n")
        spec = {"name": f"r2-entries/{self.name}", "config_id": 25,
                "patches": [f"research/lab/patches/r2/entries/{self.name}.json"],
                "args": ["--max-position-pct", "0.45"], "note": self.note}
        (SDIR / f"{self.name}.json").write_text(json.dumps(spec, indent=1) + "\n")

cells = []
def cell(name, note):
    c = Cell(name, note); cells.append(c); return c

# scaffold: every window re-declared unchanged + every helper at weight 0 → must equal base/row25
c = cell("scaffold", "row 25 windows re-declared unchanged + weight-0 helpers; must equal base/row25")
for i in ALL: c.w(i)
c.inds |= set(HELPERS)

# clocks
c = cell("cut0950", "am/pm cut-over 09:50")
for i in AM: c.w(i)["entry_before"] = "09:50"
for i in PM: c.w(i)["entry_after"] = "09:50"
c = cell("cut1010", "am/pm cut-over 10:10")
for i in AM: c.w(i)["entry_before"] = "10:10"
for i in PM: c.w(i)["entry_after"] = "10:10"
c = cell("pm1115", "pm windows to 11:15")
for i in PM: c.w(i)["entry_before"] = "11:15"
c = cell("pm1130", "pm windows to 11:30")
for i in PM: c.w(i)["entry_before"] = "11:30"
c = cell("am0935", "am windows from 09:35")
for i in AM: c.w(i)["entry_after"] = "09:35"
c = cell("st1000", "stress window entries from 10:00")
c.w(ST[0])["entry_after"] = "10:00"
c = cell("st1300", "stress window entries until 13:00")
c.w(ST[0])["entry_before"] = "13:00"

# prior-day-low dial (am/pm windows)
for v, n in [(-1.5, "pdl15"), (-2.5, "pdl25")]:
    c = cell(n, f"pdl_5m.dist_low_pct >= {v} on am/pm windows")
    c.add_cond(AM + PM, cmin("pdl_5m.dist_low_pct", v), "pdl_5m")

# quality
c = cell("vwbelow", "short only below session VWAP (vwapd_1m.dist_pct <= 0), am/pm")
c.add_cond(AM + PM, cmax("vwapd_1m.dist_pct", 0.0), "vwapd_1m")
c = cell("vwstretch", "not stretched: vwapd_1m.dist_pct >= -1.0, am/pm")
c.add_cond(AM + PM, cmin("vwapd_1m.dist_pct", -1.0), "vwapd_1m")
c = cell("rvol10", "5m bar volume >= its 20-bar mean, am/pm")
c.add_cond(AM + PM, cmin("rvol_5m.rvol", 1.0), "rvol_5m")
c = cell("rvolcap", "5m bar rvol <= 3, am/pm")
c.add_cond(AM + PM, cmax("rvol_5m.rvol", 3.0), "rvol_5m")
c = cell("redday", "name below today's open (gap_5m.above_open <= 0), am/pm")
c.add_cond(AM + PM, cmax("gap_5m.above_open", 0.0), "gap_5m")

# peers / cross
c = cell("peersred", ">= 2 of 3 peers red on the session (peers_red_frac >= 0.6), am/pm")
c.add_cond(AM + PM, cmin("cross_1m.peers_red_frac", 0.6))
c = cell("peersgreen", "<= 1 of 3 peers red (peers_red_frac <= 0.4), am/pm")
c.add_cond(AM + PM, cmax("cross_1m.peers_red_frac", 0.4))
c = cell("spy15", "SPY last-15m return <= +0.05 %, am/pm")
c.add_cond(AM + PM, cmax("cross_1m.index_ret_15m", 0.05))
c = cell("relweak", "name down vs SPY since prior close (pdl_5m.rel_close_pct <= 0), am/pm")
c.add_cond(AM + PM, cmax("pdl_5m.rel_close_pct", 0.0), "pdl_5m")

# 1h dial per window
c = cell("t1h_am15", "thrust 1h <= -0.15 am, -0.10 pm")
c.set_cond(AM[:1], "timescale_max", "max_score", -0.15, ts="OneHour")
c = cell("t1h_pm15", "thrust 1h -0.10 am, <= -0.15 pm")
c.set_cond(PM[:1], "timescale_max", "max_score", -0.15, ts="OneHour")

# VPIN on pm only
c = cell("pmv18", "VPIN floor 0.18 on pm windows only")
c.set_cond(PM, "indicator_min", "min_score", 0.18, iid="vpin_1m.raw_vpin")

# round-2 screen interactions (written after the first 21 screens)
c = cell("peersgreen_am", "<= 1 of 3 peers red on the am windows only")
c.add_cond(AM, cmax("cross_1m.peers_red_frac", 0.4))
c = cell("peersnotall_am", "not all 3 peers red (peers_red_frac <= 0.7) on the am windows only — neighbour of peersgreen_am")
c.add_cond(AM, cmax("cross_1m.peers_red_frac", 0.7))
c = cell("relweak_pm1115", "relweak + pm windows to 11:15")
c.add_cond(AM + PM, cmax("pdl_5m.rel_close_pct", 0.0), "pdl_5m")
for i in PM: c.w(i)["entry_before"] = "11:15"
c = cell("peersnotall_thr", "not all 3 peers red on the am THRUST window only (no screen run; straight to train, see report)")
c.add_cond(AM[:1], cmax("cross_1m.peers_red_frac", 0.7))

if __name__ == "__main__":
    PDIR.mkdir(parents=True, exist_ok=True); SDIR.mkdir(parents=True, exist_ok=True)
    for c in cells:
        c.write()
    print(len(cells), "cells:", " ".join(c.name for c in cells))
