#!/usr/bin/env python3
"""r2-second-books: writes every patch + spec of the desk (re-run to reproduce)."""
import copy, json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
P = ROOT / "research/lab/patches/r2/books"
S = ROOT / "research/lab/specs/r2/books"
LONG = ROOT / "research/lab/patches/long-2026"
P.mkdir(parents=True, exist_ok=True); S.mkdir(parents=True, exist_ok=True)

def wp(name, d):
    (P / f"{name}.json").write_text(json.dumps(d, indent=1) + "\n")
    return f"research/lab/patches/r2/books/{name}.json"

def ws(name, config_id, patches, note, tickers=None, args=None):
    d = {"name": f"r2-books/{name}", "config_id": config_id, "patches": patches, "note": note}
    if tickers: d["tickers"] = tickers
    if args: d["args"] = args
    (S / f"{name}.json").write_text(json.dumps(d, indent=1) + "\n")

NOWARM = {"warmup_days": None}   # row 23 carries warmup_days 30: remove it (the book must warm 8 like the primary)

def noise(ls=5, mn=4):
    return {"instance_id": "noise", "indicator_type": "noise_area", "timescale": "OneMinute", "weight": 0.0, "enabled": True,
            "params": {"band_mult": 1.0, "gap_adjust": True, "lookback_sessions": ls, "min_sessions": mn,
                       "decision_every_min": 30, "decision_tolerance_min": 2}}

def im(direction, dm, extra=(), name=None, prio=30):
    sgn = 1 if direction == "long" else -1
    conds = [{"type": "indicator_min", "instance_id": "noise.decision", "min_score": 0.5}]
    if sgn > 0:
        conds += [{"type": "indicator_min", "instance_id": "noise.pos", "min_score": 1.0},
                  {"type": "indicator_min", "instance_id": "noise.vwap_pct", "min_score": 0.0}]
    else:
        conds += [{"type": "indicator_max", "instance_id": "noise.pos", "max_score": -1.0},
                  {"type": "indicator_max", "instance_id": "noise.vwap_pct", "max_score": 0.0}]
    conds += [{"type": "indicator_min", "instance_id": "noise.day_move_pct", "min_score": dm}] + list(extra)
    return {"instance_id": name or f"window_im_{direction}", "action_type": "entry_window", "phase": "Entry", "priority": prio,
            "enabled": True, "params": {"name": f"im {direction}", "direction": direction, "conditions": conds,
            "entry_after": "11:59", "entry_before": "15:30",
            "exit_overrides": {"max_hold_ms": 25200000, "force_exit_by": "15:58", "score_exit_threshold": -1e9}}}

VP = lambda v: {"type": "indicator_min", "instance_id": "vpin_1m.raw_vpin", "min_score": v}
BULL = {"type": "indicator_min", "instance_id": "st1h_std.is_bullish", "min_score": 0.5}
ST1H = {"instance_id": "st1h_std", "indicator_type": "supertrend", "timescale": "OneHour", "weight": 0.0, "enabled": True,
        "params": {"period": 10, "multiplier": 3.0, "standard": True}}

# ---- Q: QQQ afternoon noise book (row 23) at lookback 8
ws("q0_asis_lb8", 23, [wp("q0_asis_lb8", {"session": NOWARM})], "row 23 (cand_48) as-is at lookback 8: noise needs 14 sessions -> expect 0 trades", "QQQ")
ws("q1_ls5", 23, [wp("q1_ls5", {"indicators": [noise()], "session": NOWARM})], "row 23, noise lookback_sessions 5 / min 4 (fits 8-day warm-up), both sides as row 23", "QQQ")
ws("q2_ls5_long", 23, [wp("q2_ls5_long", {"indicators": [noise()], "session": NOWARM, "disable": ["window_im_short"]})], "q1 long side only", "QQQ")
ws("q3_ls5_long_dm09", 23, [wp("q3_ls5_long_dm09", {"indicators": [noise()], "session": NOWARM, "disable": ["window_im_short"],
    "actions": [im("long", 0.9)]})], "q2 with day_move >= 0.9 (5-session sigma is noisier)", "QQQ")
ws("q4_ls5_long_vp", 23, [wp("q4_ls5_long_vp", {"indicators": [noise()], "session": NOWARM, "disable": ["window_im_short"],
    "actions": [im("long", 0.7, [VP(0.217)])]})], "q2 + VPIN >= 0.217", "QQQ")
ws("q5_ls4_min3", 23, [wp("q5_ls4_min3", {"indicators": [noise(4, 3)], "session": NOWARM})], "row 23 both sides, noise lookback 4 / min 3 (survives holiday weeks)", "QQQ")

# ---- M: afternoon high-vol breakout long on the four names (long-2026 r3/w1) at lookback 8
NM = wp("noise_ls5", {"indicators": [noise()], "session": NOWARM})
ws("m1_r3_ls5", 23, ["research/lab/patches/long-2026/r3_q1_vpin.json", NM], "long-2026 r3_q1_vpin at lookback 8 with the 5-session noise band", "AAPL,AMZN,NVDA,MSFT")
ws("m2_w1_ls5", 23, ["research/lab/patches/long-2026/w1_q1_vpin_bull1h.json", NM], "long-2026 w1 (r3 + 1h std supertrend bullish) at lookback 8, 5-session noise", "AAPL,AMZN,NVDA,MSFT")
ws("m3_idx_vp", 23, [wp("m3_idx_vp", {"indicators": [noise()], "session": NOWARM, "disable": ["window_im_short"],
    "actions": [im("long", 0.7, [VP(0.217)])]})], "the r3 shape on the indices: QQQ+SPY, day_move 0.7, VPIN 0.217, long only", "QQQ,SPY")
ws("m4_r3_ls5_dm125", 23, [wp("m4_r3_ls5_dm125", {"indicators": [noise()], "session": NOWARM, "disable": ["window_im_short"],
    "actions": [im("long", 1.25, [VP(0.217)])]})], "m1 with day_move 1.25 (5-session sigma)", "AAPL,AMZN,NVDA,MSFT")

# ---- S: stress variants inside row 25 (patches on config 25)
blob = json.loads((ROOT / "research/lab/patches/r2/books/_row25_blob.json").read_text())
STRESS = next(a for a in blob["actions"] if a["instance_id"] == "window_strong_core_stress")
def stress(thr=-1.5, h1=-0.4, hold=7200000, se=-0.6, before="15:30"):
    a = copy.deepcopy(STRESS)
    for c in a["params"]["conditions"]:
        if c.get("instance_id") == "cross_1m.index_session_ret": c["max_score"] = thr
        if c.get("timescale") == "OneHour": c["max_score"] = h1
    a["params"]["exit_overrides"] = {"max_hold_ms": hold, "force_exit_by": "15:55", "score_exit_threshold": se}
    a["params"]["entry_before"] = before
    return a
ws("s1_s10_1h06", 25, [wp("s1_s10_1h06", {"actions": [stress(-1.0, -0.6)]})], "row 25, stress window at SPY <= -1.0 % with 1h <= -0.6", args=["--max-position-pct", "0.45"])
ws("s2_s20_hold4h", 25, [wp("s2_s20_hold4h", {"actions": [stress(-2.0, -0.4, 14400000, -1e9)]})], "row 25, stress 2.0 %, max hold 4 h, score exit off", args=["--max-position-pct", "0.45"])
ws("s3_s15_hold4h", 25, [wp("s3_s15_hold4h", {"actions": [stress(-1.5, -0.4, 14400000, -1e9)]})], "row 25, stress 1.5 % (as live), max hold 4 h, score exit off", args=["--max-position-pct", "0.45"])
ws("s4_s15_se_off", 25, [wp("s4_s15_se_off", {"actions": [stress(-1.5, -0.4, 7200000, -1e9)]})], "row 25, stress window score exit off only (hold 2 h)", args=["--max-position-pct", "0.45"])

# ---- R: stress-reversal long book (row 25 with every short window off)
SHORTS = ["window_5m_thrust_short_am", "window_strong_core_short_am", "window_5m_thrust_short_pm", "window_strong_core_short_pm", "window_strong_core_stress"]
def rev(pc=-1.0, sr=0.3, comp=0.35, m5=0.5, after="10:00", before="14:00", extra=()):
    conds = [{"type": "indicator_max", "instance_id": "cross_1m.index_ret_prior_close", "max_score": pc},
             {"type": "indicator_min", "instance_id": "cross_1m.index_session_ret", "min_score": sr},
             {"type": "timescale_min", "timescale": "FiveMinute", "min_score": m5}]
    if comp is not None: conds.append({"type": "composite_min", "min_score": comp})
    conds += list(extra)
    return {"instance_id": "window_stress_rev_long", "action_type": "entry_window", "phase": "Entry", "priority": 10, "enabled": True,
            "params": {"name": "stress reversal long", "direction": "long", "conditions": conds, "entry_after": after, "entry_before": before,
                       "exit_overrides": {"max_hold_ms": 25200000, "force_exit_by": "15:55", "score_exit_threshold": -1e9}}}
ws("r1_rev_pc10", 25, [wp("r1_rev_pc10", {"disable": SHORTS + ["breakeven", "trailing_stop_atr"], "actions": [rev()]})],
   "own book: long when SPY <= -1 % vs prior close but >= +0.3 % vs open (gap-down reversal), name 5m >= 0.5 & composite >= 0.35, 10:00-14:00, hold to 15:55", args=["--max-position-pct", "0.45"])
ws("r2_rev_pc075_loose", 25, [wp("r2_rev_pc075_loose", {"disable": SHORTS + ["breakeven", "trailing_stop_atr"], "actions": [rev(-0.75, 0.0, None, 0.4)]})],
   "r1 looser: SPY <= -0.75 % vs prior close, >= 0 vs open, 5m >= 0.4, no composite", args=["--max-position-pct", "0.45"])
ws("r3_rev_idx", 25, [wp("r3_rev_idx", {"disable": SHORTS + ["breakeven", "trailing_stop_atr"], "actions": [rev(-1.0, 0.3, None, 0.4)]})],
   "the reversal long on SPY+QQQ themselves (no composite, 5m >= 0.4)", tickers="SPY,QQQ", args=["--max-position-pct", "0.45"])

# ---- A: afternoon short windows inside row 25
PM = next(a for a in blob["actions"] if a["instance_id"] == "window_5m_thrust_short_pm")
a1 = copy.deepcopy(PM); a1["instance_id"] = "window_5m_thrust_short_mid"; a1["priority"] = 60
a1["params"].update({"name": "5m thrust short mid", "entry_after": "11:00", "entry_before": "13:00", "exit_overrides": {"force_exit_by": "13:55"}})
ws("a1_thrust_1100_1300", 25, [wp("a1_thrust_1100_1300", {"actions": [a1]})], "row 25 + thrust short 11:00-13:00 (pm conditions, flat 13:55)", args=["--max-position-pct", "0.45"])
a2 = {"instance_id": "window_vwap_fail_short", "action_type": "entry_window", "phase": "Entry", "priority": 60, "enabled": True,
      "params": {"name": "vwap reclaim fail short", "direction": "short", "entry_after": "12:00", "entry_before": "15:00",
                 "conditions": [{"type": "indicator_range", "instance_id": "vwap_dist_1hr.dist_pct", "min_score": -0.15, "max_score": 0.0},
                                {"type": "indicator_max", "instance_id": "vwap_dist_1hr.min_dist_pct_today", "max_score": -0.6},
                                {"type": "timescale_max", "timescale": "FiveMinute", "max_score": -0.3},
                                {"type": "timescale_max", "timescale": "OneHour", "max_score": -0.4},
                                VP(0.2)],
                 "exit_overrides": {"force_exit_by": "15:55"}}}
ws("a2_vwap_fail", 25, [wp("a2_vwap_fail", {"actions": [a2]})], "row 25 + VWAP-reclaim-fail short 12:00-15:00 (was <= -0.6 % below VWAP, back within 0.15 %, 5m <= -0.3, 1h <= -0.4, VPIN 0.2)", args=["--max-position-pct", "0.45"])
print("ok")

# ---- round 2 of the screen
a3 = copy.deepcopy(a2); a3["instance_id"] = "window_vwap_fail_short_loose"
a3["params"]["conditions"] = [{"type": "indicator_range", "instance_id": "vwap_dist_1hr.dist_pct", "min_score": -0.25, "max_score": 0.05},
    {"type": "indicator_max", "instance_id": "vwap_dist_1hr.min_dist_pct_today", "max_score": -0.4},
    {"type": "timescale_max", "timescale": "FiveMinute", "max_score": -0.3},
    {"type": "timescale_max", "timescale": "OneHour", "max_score": -0.1}]
ws("a3_vwap_fail_loose", 25, [wp("a3_vwap_fail_loose", {"actions": [a3]})], "a2 loosened (back within 0.25 %, was <= -0.4 %, 1h <= -0.1, no VPIN) — checks the window can fire", args=["--max-position-pct", "0.45"])
ws("m5_r3_ls5_dm175", 23, [wp("m5_r3_ls5_dm175", {"indicators": [noise()], "session": NOWARM, "disable": ["window_im_short"],
    "actions": [im("long", 1.75, [VP(0.217)])]})], "m1 with day_move 1.75 (long-2026 v4 plateau)", "AAPL,AMZN,NVDA,MSFT")
