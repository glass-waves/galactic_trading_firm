#!/usr/bin/env python3
"""follow-ups on sim_daily: per-name concentration, concurrency (how many positions open at once),
earnings-reaction-day open->close across the universe (day-level event toggle evidence),
and a capital-constrained version of the volume numbers (max K concurrent positions)."""
import json, numpy as np, os
import sim_daily as S
ROOT = os.path.dirname(os.path.abspath(__file__))
def per_name(trades, top=8):
    by = {}
    for t in trades: by.setdefault(t["sym"], []).append(t["pnl"])
    rows = sorted(((s, len(v), round(sum(v))) for s, v in by.items()), key=lambda r: -r[2])
    tot = sum(r[2] for r in rows); top3 = sum(r[2] for r in rows[:3])
    return {"top": rows[:top], "bottom": rows[-4:], "top3_share": round(top3 / tot, 2) if tot else None, "names_positive": sum(1 for r in rows if r[2] > 0), "names": len(rows)}
def concurrency(trades):
    ev = {}
    for t in trades:
        ev[t["entry"]] = ev.get(t["entry"], 0) + 1; ev[t["exit"]] = ev.get(t["exit"], 0) - 1
    open_, series = 0, []
    for d in sorted(ev): open_ += ev[d]; series.append(open_)
    days_in_mkt = sum(1 for d in S.DATES if d >= S.T0 and any(t["entry"] <= d < t["exit"] for t in trades))
    return {"max_concurrent": max(series), "mean_when_active": round(float(np.mean([x for x in series if x > 0])), 1), "pct_days_in_market": round(100 * days_in_mkt / sum(1 for d in S.DATES if d >= S.T0), 1)}
def capped(trades, K):
    """keep at most K open positions (first-come, by entry date, ties by symbol) — the realistic small-book volume"""
    kept, open_ = [], []
    for t in sorted(trades, key=lambda t: (t["entry"], t["sym"])):
        open_ = [x for x in open_ if x["exit"] > t["entry"]]
        if len(open_) < K: kept.append(t); open_.append(t)
    return kept
def summarize(name, trades):
    st = S.stats(trades); out = {"stats": st, "per_name": per_name(trades), "concurrency": concurrency(trades)}
    for K in (2, 4, 8):
        c = capped(trades, K); cs = S.stats(c); out[f"cap{K}"] = {"trades": cs["trades"], "per_year": round(cs["trades"] / 4.73, 1), "pf": cs.get("pf"), "pnl": cs.get("pnl"), "avg_bps": cs.get("avg_bps"), "maxdd": cs.get("maxdd")}
    return out
OUT = {}
KEYS = {
 "a_pead_h10_universe": lambda: S.run(S.s_pead, S.NAMES, hold=10),
 "a_pead_h5_universe": lambda: S.run(S.s_pead, S.NAMES, hold=5),
 "b_rev_h5_universe": lambda: S.run(S.s_reversal, S.NAMES, hold=5),
 "c_bo_20d_h10_universe": lambda: S.run(S.s_breakout, S.NAMES, hold=10, kind="20d"),
 "c_bo_52w_h10_universe": lambda: S.run(S.s_breakout, S.NAMES, hold=10, kind="52w"),
 "e_bearbounce_h3_four": lambda: S.run(S.s_bearbounce, S.FOUR, hold=3),
 "e_bearbounce_h3_universe": lambda: S.run(S.s_bearbounce, S.NAMES, hold=3),
 "b_rev_h3_four": lambda: S.run(S.s_reversal, S.FOUR, hold=3),
}
for k, f in KEYS.items():
    OUT[k] = summarize(k, f()); o = OUT[k]
    print(f"{k}: top3_share={o['per_name']['top3_share']} names+={o['per_name']['names_positive']}/{o['per_name']['names']} top={o['per_name']['top'][:4]} bottom={o['per_name']['bottom'][-2:]}")
    print(f"   concurrency={o['concurrency']}  cap2={o['cap2']} cap4={o['cap4']} cap8={o['cap8']}")
# PEAD sensitivity: exclude 2022-23 NVDA-style mega winners? -> drop top name; and threshold sweep
pead = KEYS["a_pead_h10_universe"](); top = OUT["a_pead_h10_universe"]["per_name"]["top"][0][0]
OUT["pead_h10_ex_top_name"] = {"dropped": top, **S.stats([t for t in pead if t["sym"] != top])}
print("pead h10 ex", top, OUT["pead_h10_ex_top_name"])
for th in (0.01, 0.02, 0.03, 0.05, 0.08):
    tr = S.run(S.s_pead, S.NAMES, hold=10, thresh=th); OUT[f"pead_h10_thresh{th}"] = S.stats(tr); print("pead h10 thresh", th, S.stats(tr))
# reaction-day-only variant: also negative reaction days (long after >3% down day) as control
def s_pead_neg(sym, a, hold=10, thresh=0.03):
    o, c = a["open"], a["close"]; days, _ = S.EARN[sym]
    for r in sorted(days):
        if r < 1 or r + hold >= S.N: continue
        if c[r] / c[r - 1] - 1 < -thresh:
            e = r + 1; x = e + hold - 1; yield S.trade(sym, e, x, o[e], c[x], "pead_neg")
OUT["control_long_after_neg_reaction_h10"] = S.stats(S.run(s_pead_neg, S.NAMES)); print("control long after -3% reaction:", OUT["control_long_after_neg_reaction_h10"])
# day-level event evidence: open->close return on earnings reaction days vs ordinary days, universe and four
def oc_split(syms):
    ev, od, big_ev, big = [], [], 0, 0
    for s in syms:
        d, a = S.DATA[s]; days, _ = S.EARN[s]
        for i in range(1, S.N):
            if S.DATES[i] < S.T0: continue
            r = a["close"][i] / a["open"][i] - 1
            (ev if i in days else od).append(r)
            if r > 0.02: big += 1; big_ev += (i in days)
    ev, od = np.array(ev), np.array(od)
    return {"event_days": len(ev), "event_oc_mean_bps": round(1e4 * ev.mean(), 1), "event_oc_win%": round(100 * (ev > 0).mean(), 1), "event_oc_std_bps": round(1e4 * ev.std()),
            "ordinary_days": len(od), "ordinary_oc_mean_bps": round(1e4 * od.mean(), 1), "ordinary_oc_win%": round(100 * (od > 0).mean(), 1),
            "share_of_+2%_oc_days_that_are_event_days": round(100 * big_ev / big, 1), "event_day_share_of_all_days": round(100 * len(ev) / (len(ev) + len(od)), 1)}
OUT["oc_event_split_four"] = oc_split(S.FOUR); OUT["oc_event_split_universe"] = oc_split(S.NAMES)
print("o->c four:", OUT["oc_event_split_four"]); print("o->c universe:", OUT["oc_event_split_universe"])
# overnight gross (no cost) for reference
on4 = S.run(S.s_overnight, S.FOUR); onu = S.run(S.s_overnight, S.NAMES)
OUT["overnight_gross_bps"] = {"four": round(1e4 * (np.mean([t["ret"] for t in on4]) + S.COST), 1), "universe": round(1e4 * (np.mean([t["ret"] for t in onu]) + S.COST), 1)}
print("overnight gross bps:", OUT["overnight_gross_bps"])
json.dump(OUT, open(os.path.join(ROOT, "results_analysis.json"), "w"), indent=1, default=str)
