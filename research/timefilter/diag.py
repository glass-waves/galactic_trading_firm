#!/usr/bin/env python3
"""time-of-day diagnosis on the scaffold's full-day tick dump (data/tf_am_<year>_ticks.csv,
DUMP_TICKS=1 run_cell.sh am). trigger conditions are recomputed offline from the dumped scores
(thrust: composite <= -0.35, 5m <= -0.5 and lagging 1m and 1h by 0.1, 1h <= 0; core: composite
<= -0.35, 5m <= -0.4, 1h <= -0.4). forward returns are the short's gross close-to-close return
30 / 60 min later (capped at the day's last bar), in bps; honest costs are ~8 bps per round trip.

"entries" = non-overlapping samples: per ticker-day, a qualifying bar is taken only if the last
taken one was >= 30 min earlier (a crude stand-in for one position at a time).

usage: diag.py [tag]        (default tf_am) -> prints every table; caches parsed rows in
                            logs/<tag>_diag.pkl
"""
import csv
import datetime as dt
import math
import os
import pickle
import re
import statistics
import sys
import zoneinfo
from collections import defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
YEARS = ["2022", "2023", "2024", "2025", "2026"]
ET = zoneinfo.ZoneInfo("America/New_York")
KEYS = ["cross_1m", "cross_1m.index_session_ret", "cross_1m.index_ret_5m", "cross_1m.index_ret_15m",
        "vpin_1m.raw_vpin", "vpin_p.pctile"]
PAT = {k: re.compile(r'"' + re.escape(k) + r'":(-?[0-9.]+|null)') for k in KEYS}
csv.field_size_limit(10**9)


def num(x):
    try:
        return float(x)
    except (TypeError, ValueError):
        return None


def spy_sigma():
    """20-day stdev of SPY daily close-to-close returns (fraction), keyed by the NEXT date (no look-ahead)."""
    closes = {}
    with open(os.path.join(ROOT, "data/bars_iex/SPY.csv")) as fh:
        for r in csv.DictReader(fh):
            t = dt.datetime.fromtimestamp(int(r["timestamp"]), ET)
            if (t.hour, t.minute) < (16, 0) and (t.hour, t.minute) >= (9, 30):
                closes[t.date()] = float(r["close"])
    days = sorted(closes)
    rets = [closes[days[i]] / closes[days[i - 1]] - 1 for i in range(1, len(days))]
    out = {}
    for i in range(21, len(days)):
        out[days[i].isoformat()] = statistics.pstdev(rets[i - 21:i - 1])
    return out


def load(tag):
    pk = os.path.join(ROOT, f"logs/{tag}_diag.pkl")   # cache (logs/ is untracked)
    if os.path.exists(pk):
        return pickle.load(open(pk, "rb"))
    seq = defaultdict(list)
    for y in YEARS:
        with open(os.path.join(ROOT, f"data/{tag}_{y}_ticks.csv")) as fh:
            for r in csv.DictReader(fh):
                ind = r["indicators"]
                v = {}
                for k, p in PAT.items():
                    m = p.search(ind)
                    v[k] = num(m.group(1)) if m else None
                t = dt.datetime.fromisoformat(r["ts"]).astimezone(ET)
                mins = t.hour * 60 + t.minute - 570
                if mins < 0 or mins >= 390:
                    continue
                seq[(r["date"], r["ticker"])].append((
                    mins, float(r["close"]), num(r["composite"]), num(r["s1m"]), num(r["s5m"]), num(r["s1h"]),
                    v["cross_1m"], v["cross_1m.index_session_ret"], v["cross_1m.index_ret_5m"],
                    v["cross_1m.index_ret_15m"], v["vpin_1m.raw_vpin"], v["vpin_p.pctile"], r["event"]))
    for k in seq:
        seq[k].sort()
    seq = dict(seq)
    pickle.dump(seq, open(pk, "wb"))
    return seq


def thrust(c, s1, s5, s1h):
    return (None not in (c, s1, s5, s1h) and c <= -0.35 and s5 <= s1 - 0.1 and s5 <= s1h - 0.1
            and s5 <= -0.5 and s1h <= 0.0)


def core(c, s5, s1h):
    return None not in (c, s5, s1h) and c <= -0.35 and s5 <= -0.4 and s1h <= -0.4


def fwd(bars, i, k):
    m0, c0 = bars[i][0], bars[i][1]
    j = i
    while j + 1 < len(bars) and bars[j + 1][0] <= m0 + k:
        j += 1
    return -(bars[j][1] / c0 - 1) * 1e4


HH = [f"{(570 + 30 * h) // 60:02d}:{(570 + 30 * h) % 60:02d}" for h in range(13)]


def enrich(seq, sig):
    """flat list of trigger bars with filter states and forward returns."""
    out = []
    for (d, t), bars in seq.items():
        s = sig.get(d)
        for i, b in enumerate(bars):
            mins, close, c, s1, s5, s1h, x, sret, r5, r15, vp, pct, ev = b
            T, C = thrust(c, s1, s5, s1h), core(c, s5, s1h)
            if not (T or C):
                continue
            z = None
            if s and sret is not None:
                z = (sret / 100) / (s * math.sqrt(max(mins + 1, 1) / 390))
            out.append(dict(d=d, y=d[:4], t=t, m=mins, hh=min(mins // 30, 12), T=T, C=C,
                            band=x is not None and -0.4 <= x <= 0.4, sret=sret, r5=r5, r15=r15,
                            vpin=vp, pct=pct, z=z, sqm=(sret / math.sqrt(mins + 1)) if sret is not None else None,
                            f30=fwd(bars, i, 30), f60=fwd(bars, i, 60), ev=ev))
    return out


def sample(rows, pred, gap=30):
    """non-overlapping qualifying bars per ticker-day."""
    last = {}
    out = []
    for r in sorted(rows, key=lambda r: (r["d"], r["t"], r["m"])):
        if not pred(r):
            continue
        k = (r["d"], r["t"])
        if k in last and r["m"] - last[k] < gap:
            continue
        last[k] = r["m"]
        out.append(r)
    return out


def fr(rs, key="f30"):
    if not rs:
        return "—"
    v = [r[key] for r in rs]
    hit = sum(1 for x in v if x > 0) / len(v) * 100
    return f"{statistics.mean(v):+.1f} ({hit:.0f} %)"


def vp(r, floor=0.217):
    return r["vpin"] is not None and r["vpin"] >= floor


def table_block(rows):
    print("\n## A. trigger bars by half-hour: how often each filter blocks (all years)\n")
    print("| half-hour | trigger bars | band fails | VPIN fails | both pass | entries (both pass) | SPY |sess ret| p50 | raw VPIN p50 / p80 |")
    print("|---|---|---|---|---|---|---|---|")
    for h in range(12):
        rs = [r for r in rows if r["hh"] == h]
        if not rs:
            continue
        bf = sum(1 for r in rs if not r["band"]) / len(rs) * 100
        vf = sum(1 for r in rs if not vp(r)) / len(rs) * 100
        both = [r for r in rs if r["band"] and vp(r)]
        ent = sample(both, lambda r: True)
        sr = sorted(abs(r["sret"]) for r in rs if r["sret"] is not None)
        vv = sorted(r["vpin"] for r in rs if r["vpin"] is not None)
        print(f"| {HH[h]} | {len(rs)} | {bf:.0f} % | {vf:.0f} % | {len(both)} | {len(ent)} | "
              f"{sr[len(sr)//2]:.2f} % | {vv[len(vv)//2]:.3f} / {vv[int(len(vv)*0.8)]:.3f} |")


def table_fwd(rows):
    print("\n## B. forward short return of non-overlapping trigger entries by half-hour and filter state\n")
    print("mean gross bps (hit rate), 30 min / 60 min. n = entries.\n")
    print("| half-hour | both pass n | 30 | 60 | band fails, VPIN ok n | 30 | 60 | band ok, VPIN fails n | 30 | 60 | both fail n | 30 | 60 |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    states = [lambda r: r["band"] and vp(r), lambda r: (not r["band"]) and vp(r),
              lambda r: r["band"] and not vp(r), lambda r: not r["band"] and not vp(r)]
    for h in range(12):
        rs = [r for r in rows if r["hh"] == h]
        cells = []
        for st in states:
            e = sample(rs, st)
            cells += [str(len(e)), fr(e, "f30"), fr(e, "f60")]
        print(f"| {HH[h]} | " + " | ".join(cells) + " |")


def table_alt(rows, filters):
    print("\n## C. alternative market filters (with VPIN >= 0.217): entries / fwd60 bps (hit) by clock range\n")
    ranges = [("09:30-10:00", 0, 30), ("10:00-11:30", 30, 120), ("11:30-13:00", 120, 210), ("13:00-15:30", 210, 360)]
    print("| filter | " + " | ".join(r[0] for r in ranges) + " | per-year fwd60 (all clocks) |")
    print("|---|" + "---|" * (len(ranges) + 1))
    for name, f in filters:
        cells = []
        for _, a, b in ranges:
            e = sample([r for r in rows if a <= r["m"] < b], lambda r: f(r) and vp(r))
            cells.append(f"{len(e)} / {fr(e, 'f60')}")
        e_all = sample([r for r in rows if r["m"] < 360], lambda r: f(r) and vp(r))
        py = " ".join(f"{y[2:]}:{statistics.mean([r['f60'] for r in e_all if r['y']==y] or [0]):+.0f}" for y in YEARS)
        print(f"| {name} | " + " | ".join(cells) + f" | {py} |")


def table_vpin(rows, filters):
    print("\n## D. VPIN variants (with SPY band): entries / fwd60 bps (hit) by clock range\n")
    ranges = [("09:30-10:00", 0, 30), ("10:00-11:30", 30, 120), ("11:30-13:00", 120, 210), ("13:00-15:30", 210, 360)]
    print("| VPIN rule | " + " | ".join(r[0] for r in ranges) + " |")
    print("|---|" + "---|" * len(ranges))
    for name, f in filters:
        cells = []
        for _, a, b in ranges:
            e = sample([r for r in rows if a <= r["m"] < b], lambda r: r["band"] and f(r))
            cells.append(f"{len(e)} / {fr(e, 'f60')}")
        print(f"| {name} | " + " | ".join(cells) + " |")


def table_sqm(rows):
    print("\n## E. time-scaled band k (|SPY session % / sqrt(min)| <= k, with VPIN >= 0.217) vs v18's band\n")
    print("entries / fwd60 bps (hit); last column: 10:00-11:30 fwd60 per year (n / bps)\n")
    ranges = [("09:30", 0, 30), ("10:00", 30, 60), ("10:30", 60, 90), ("11:00", 90, 120)]
    print("| filter | " + " | ".join(r[0] for r in ranges) + " | 10:00-11:30 by year |")
    print("|---|" + "---|" * (len(ranges) + 1))
    fl = [("band ±0.2 % (v18)", lambda r: r["band"])]
    fl += [(f"k = {k}", (lambda k: lambda r: r["sqm"] is not None and abs(r["sqm"]) <= k)(k)) for k in (0.03, 0.04, 0.05, 0.06)]
    fl += [("|SPY 15m| <= 0.10 %", lambda r: r["r15"] is not None and abs(r["r15"]) <= 0.10)]
    for name, f in fl:
        cells = []
        for _, a, b in ranges:
            e = sample([r for r in rows if a <= r["m"] < b], lambda r: f(r) and vp(r))
            cells.append(f"{len(e)} / {fr(e, 'f60')}")
        e = sample([r for r in rows if 30 <= r["m"] < 120], lambda r: f(r) and vp(r))
        py = " ".join(f"{len([r for r in e if r['y'] == y])}/{statistics.mean([r['f60'] for r in e if r['y'] == y] or [0]):+.0f}" for y in YEARS)
        print(f"| {name} | " + " | ".join(cells) + f" | {py} |")


def main():
    tag = sys.argv[1] if len(sys.argv) > 1 else "tf_am"
    seq = load(tag)
    sig = spy_sigma()
    rows = enrich(seq, sig)
    print(f"# diagnosis on {tag}: {len(seq)} ticker-days, {len(rows)} trigger bars (thrust or core)")
    table_block(rows)
    table_fwd(rows)
    ab = lambda k, x: (lambda r: r[k] is not None and abs(r[k]) <= x)
    filters = [("none", lambda r: True), ("session band ±0.2 % (v18)", lambda r: r["band"])]
    filters += [(f"|SPY 15m| <= {x} %", ab("r15", x)) for x in (0.10, 0.15, 0.20)]
    filters += [(f"|SPY 5m| <= {x} %", ab("r5", x)) for x in (0.05, 0.10)]
    filters += [(f"|z| <= {x}", ab("z", x)) for x in (0.25, 0.5, 0.75)]
    filters += [(f"|sess/sqrt(min)| <= {x}", ab("sqm", x)) for x in (0.05, 0.08, 0.12)]
    table_alt(rows, filters)
    vfl = [(f"raw >= {x}", (lambda x: lambda r: vp(r, x))(x)) for x in (0.15, 0.18, 0.217, 0.25)]
    vfl += [(f"pctile(1950) >= {p}", (lambda p: lambda r: r["pct"] is not None and r["pct"] >= p)(p)) for p in (0.6, 0.7, 0.8, 0.9)]
    vfl += [("none", lambda r: True)]
    table_vpin(rows, vfl)
    table_sqm(rows)


if __name__ == "__main__":
    main()
