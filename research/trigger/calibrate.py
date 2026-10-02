#!/usr/bin/env python3
"""frequency calibration for the pullback (c) and VPIN-slope (d) windows on the scaffold's morning
tick dump (data/tr_am_<year>_ticks.csv, DUMP_TICKS=1 --dump-window-only). counts bars, not P&L:
thresholds are chosen so a new window fires about as often as the window it replaces / adds to,
never on forward returns.

usage: calibrate.py pullback | slope | nearmiss
"""
import csv
import json
import sys
from collections import defaultdict

YEARS = ["2022", "2023", "2024", "2025", "2026"]
csv.field_size_limit(10**8)


def rows():
    for y in YEARS:
        with open(f"data/tr_am_{y}_ticks.csv") as fh:
            for r in csv.DictReader(fh):
                ind = json.loads(r["indicators"]) if r["indicators"] else {}
                yield y, r, ind


def num(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def thrust(c, s1, s5, s1h):
    return (c is not None and s5 is not None and s1 is not None and s1h is not None and c <= -0.35
            and s5 <= s1 - 0.1 and s5 <= s1h - 0.1 and s5 <= -0.5 and s1h <= 0.0)


def band(ind):
    x = ind.get("cross_1m")
    return x is not None and -0.4 <= x <= 0.4


def episodes():
    """per (date, ticker): the ordered bar list with what the windows need."""
    seq = defaultdict(list)
    for y, r, ind in rows():
        seq[(r["date"], r["ticker"])].append(dict(
            y=y, ts=r["ts"], c=num(r["composite"]), s1=num(r["s1m"]), s5=num(r["s5m"]), s1h=num(r["s1h"]),
            band=band(ind), vpin=ind.get("vpin_1m.raw_vpin"), ev=r["event"], nm=r["near_miss"],
            **{k.split(".", 1)[1]: v for k, v in ind.items() if k.startswith("trig_1m.")}))
    for k in seq:
        seq[k].sort(key=lambda b: b["ts"])
    return seq


def pullback(seq):
    n_thr = n_thr_f = 0
    grid = defaultdict(int)
    for (d, t), bars in seq.items():
        last_thrust = -999
        for i, b in enumerate(bars):
            if thrust(b["c"], b["s1"], b["s5"], b["s1h"]):
                last_thrust = i
                n_thr += 1
                n_thr_f += b["band"] and (b["vpin"] or 0) >= 0.217
            f_ok = b["band"] and (b["vpin"] or 0) >= 0.217
            if not f_ok or b.get("drop_pct") is None or b["c"] is None or b["s5"] is None:
                continue
            for drop in (0.3, 0.5, 0.7):
                for rmin, rmax in ((0.2, 0.6), (0.3, 0.7), (0.4, 0.8)):
                    for m5 in (-0.3, -0.4):
                        ok = (b["drop_pct"] >= drop and 2 <= b["bars_since_low"] <= 12
                              and rmin <= b["retrace"] <= rmax and b["s5"] <= m5 and b["c"] <= -0.2
                              and b["s1h"] is not None and b["s1h"] <= 0.0)
                        if ok:
                            grid[(drop, rmin, rmax, m5)] += 1
                            if i - last_thrust <= 15:
                                grid[(drop, rmin, rmax, m5, "after_thrust")] += 1
    print(f"thrust-trigger bars (no filters): {n_thr}; with SPY band + VPIN: {n_thr_f}")
    print("pullback candidate bars with filters (drop %, retrace range, 5m max): bars / of which <=15 bars after a thrust bar")
    for k in sorted(k for k in grid if len(k) == 4):
        print(f"  drop>={k[0]} retrace[{k[1]},{k[2]}] 5m<={k[3]}: {grid[k]} / {grid[k + ('after_thrust',)]}")


def slope(seq):
    """thrust- or core-trigger bars with the SPY band and VPIN in [lo, 0.217): how many, and of those how
    many see VPIN >= 0.217 within the next 5 bars (the tiered study's front-runner population)."""
    for lo in (0.15, 0.17):
        for key in ("vpin_slope_3", "vpin_slope_5"):
            for th in (0.0, 0.01, 0.02, 0.03, 0.04):
                n = cross = 0
                days = set()
                for (d, t), bars in seq.items():
                    for i, b in enumerate(bars):
                        v = b["vpin"]
                        if v is None or not (lo <= v < 0.217) or not b["band"]:
                            continue
                        core = (b["c"] is not None and b["c"] <= -0.35 and (b["s5"] or 0) <= -0.4 and (b["s1h"] or 0) <= -0.4)
                        if not (thrust(b["c"], b["s1"], b["s5"], b["s1h"]) or core):
                            continue
                        sl = b.get(key)
                        if sl is None or sl < th:
                            continue
                        n += 1
                        days.add((d, t))
                        cross += any((x["vpin"] or 0) >= 0.217 for x in bars[i + 1:i + 6])
                print(f"lo {lo} {key} >= {th:.2f}: {n} bars on {len(days)} ticker-days, "
                      f"VPIN >= 0.217 within 5 bars: {cross} ({cross / n * 100 if n else 0:.0f} %)")


def nearmiss(seq):
    """thrust window, morning bars with composite <= -0.35: which single condition was the only one failing."""
    single = defaultdict(int)
    for bars in seq.values():
        for b in bars:
            c, s1, s5, h = b["c"], b["s1"], b["s5"], b["s1h"]
            if None in (c, s1, s5, h) or c > -0.35:
                continue
            single["composite <= -0.35 (all)"] += 1
            conds = {"5m lag": s5 <= s1 - 0.1 and s5 <= h - 0.1, "5m <= -0.5": s5 <= -0.5, "1h <= 0": h <= 0,
                     "SPY band": b["band"], "VPIN": (b["vpin"] or 0) >= 0.217}
            fail = [k for k, v in conds.items() if not v]
            single[fail[0] if len(fail) == 1 else ("PASS" if not fail else "two or more")] += 1
    for k, v in sorted(single.items(), key=lambda kv: -kv[1]):
        print(f"{v:7d}  {k}")


if __name__ == "__main__":
    seq = episodes()
    {"pullback": pullback, "slope": slope, "nearmiss": nearmiss}[sys.argv[1]](seq)
