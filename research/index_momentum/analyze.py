#!/usr/bin/env python3
"""index intraday-momentum study: per-cell report, decoupling vs the live book, combined book, LOYO.

usage (from the repo root):
  analyze.py card TAG[+TAG...] [--dm-min X] [--dm-max X]   full card for one cell (A+B = union of
                                                           two single-ticker tags = the two-ticker book)
  analyze.py grid TAG ...                                  one line per cell
  analyze.py decouple TAG[+TAG] [--dm-min X]               correlation / overlap / combined book vs v18
  analyze.py dmgrid TAG[+TAG]                              day-level vol filter grid (noise.day_move_pct
                                                           recomputed from the bars, see day_moves())
  analyze.py loyo TAG[+TAG]                                leave-one-year-out pick of the vol-filter threshold
the vol filter is a day-level condition (day_move_pct is constant within a day), so a filtered
cell = the base cell's trades on the days that pass: exact, no sweep needed (confirmed by a sweep).
"""
import csv
import datetime as dt
import math
import sys
import zoneinfo
from collections import defaultdict

YEARS = ["2022", "2023", "2024", "2025", "2026"]
ET = zoneinfo.ZoneInfo("America/New_York")
V18 = "iex_v18"
SQRT_BAND = "tf_c_sq05_1100"     # spy-sqrt-band (candidate 44) = this research cell, verified identical


def load(tag):
    trades, days = [], set()
    for t in tag.split("+"):
        for y in YEARS:
            try:
                fh = open(f"data/{t}_{y}_trades.csv")
            except FileNotFoundError:
                continue
            for r in csv.DictReader(fh):
                if r["row_type"] == "summary":
                    days.add(r["date"])
                elif r["row_type"] == "trade":
                    r["pnl"] = float(r["pnl"])
                    r["year"] = r["date"][:4]
                    trades.append(r)
    # the sweep writes a summary row for every weekday incl. holidays: keep days with SPY bars
    return trades, days & bar_days()


_BD = []


def bar_days():
    if not _BD:
        _BD.append({dt.datetime.fromtimestamp(int(r["timestamp"]), ET).date().isoformat()
                    for r in csv.DictReader(open("data/bars_iex/SPY.csv"))})
    return _BD[0]


_DM = {}


def day_moves(ticker, lookback=14):
    """noise.day_move_pct per date, as the indicator computes it: mean |16:00 close / open - 1| (the
    15:00 hourly candle's close vs the 09:30 open) over the previous `lookback` full sessions."""
    if (ticker, lookback) in _DM:
        return _DM[(ticker, lookback)]
    sess = {}
    for r in csv.DictReader(open(f"data/bars_iex/{ticker}.csv")):
        t = dt.datetime.fromtimestamp(int(r["timestamp"]), ET)
        d = t.date().isoformat()
        s = sess.setdefault(d, {"open": None, "first_h": t.hour, "close": None, "last_h": None})
        if s["open"] is None:
            s["open"] = float(r["open"])
        s["close"], s["last_h"] = float(r["close"]), t.hour
    dates = sorted(sess)
    out, hist = {}, []
    for d in dates:
        if len(hist) >= lookback:
            out[d] = 100 * sum(hist[-lookback:]) / lookback
        s = sess[d]
        if s["first_h"] == 9 and s["last_h"] == 15:
            hist.append(abs(s["close"] / s["open"] - 1))
    _DM[(ticker, lookback)] = out
    return out


def dm_filter(trades, lo=None, hi=None):
    if lo is None and hi is None:
        return trades
    keep = []
    for r in trades:
        v = day_moves(r["ticker"]).get(r["date"])
        if v is None:
            continue
        if (lo is None or v >= lo) and (hi is None or v < hi):
            keep.append(r)
    return keep


def pf(rows):
    w = sum(r["pnl"] for r in rows if r["pnl"] > 0)
    l = -sum(r["pnl"] for r in rows if r["pnl"] < 0)
    return w / l if l > 0 else float("inf")


def daily(rows):
    d = defaultdict(float)
    for r in rows:
        d[r["date"]] += r["pnl"]
    return d


def max_dd(dpnl, dates=None):
    eq = peak = dd = 0.0
    for d in sorted(dates or dpnl):
        eq += dpnl.get(d, 0.0)
        peak = max(peak, eq)
        dd = min(dd, eq - peak)
    return dd


def line(rows, sessions):
    n = len(rows)
    p = sum(r["pnl"] for r in rows)
    dp = daily(rows)
    wins = sum(1 for r in rows if r["pnl"] > 0)
    yrs = {y: [r for r in rows if r["year"] == y] for y in YEARS}
    ypos = sum(1 for y in YEARS if sum(r["pnl"] for r in yrs[y]) > 0)
    nyears = len(sessions) / 252 if sessions else 4.69
    return (f"{n:5d} tr {n / nyears:5.0f}/yr  P&L {p:+8.0f}  PF {pf(rows):4.2f}  win {100 * wins / max(n, 1):3.0f}%  "
            f"DD {max_dd(dp):+6.0f}  worst day {min(dp.values(), default=0):+5.0f}  "
            f"active {100 * len(dp) / max(len(sessions), 1):3.0f}%  years+ {ypos}/5")


def card(tag, lo=None, hi=None):
    rows, sessions = load(tag)
    rows = dm_filter(rows, lo, hi)
    print(f"== {tag}" + (f"  day_move in [{lo}, {hi})" if lo or hi else ""))
    print("  all    " + line(rows, sessions))
    for y in YEARS:
        yr = [r for r in rows if r["year"] == y]
        ys = {d for d in sessions if d.startswith(y)}
        print(f"  {y}   {len(yr):4d} tr  P&L {sum(r['pnl'] for r in yr):+7.0f}  PF {pf(yr):4.2f}  "
              f"DD {max_dd(daily(yr)):+5.0f}  active {100 * len(daily(yr)) / max(len(ys), 1):3.0f}%")
    for side in ("Long", "Short"):
        s = [r for r in rows if r["direction"] == side]
        print(f"  {side:5s}  {len(s):4d} tr  P&L {sum(r['pnl'] for r in s):+7.0f}  PF {pf(s):4.2f}  "
              f"win {100 * sum(1 for r in s if r['pnl'] > 0) / max(len(s), 1):3.0f}%")
    ex = defaultdict(list)
    for r in rows:
        ex[r["exit_reason"]].append(r)
    print("  exits  " + "  ".join(f"{k} {len(v)} ({sum(r['pnl'] for r in v):+.0f})" for k, v in sorted(ex.items())))
    if len({r["ticker"] for r in rows}) > 1:
        for t in sorted({r["ticker"] for r in rows}):
            s = [r for r in rows if r["ticker"] == t]
            print(f"  {t:5s}  {len(s):4d} tr  P&L {sum(r['pnl'] for r in s):+7.0f}  PF {pf(s):4.2f}")
    bps = [1e4 * float(r["pnl_pct"]) for r in rows]
    if bps:
        print(f"  mean trade {sum(bps) / len(bps):+.1f} bps net; trades/active day {len(rows) / len(daily(rows)):.2f}")


def corr(a, b):
    n = len(a)
    ma, mb = sum(a) / n, sum(b) / n
    va = sum((x - ma) ** 2 for x in a)
    vb = sum((y - mb) ** 2 for y in b)
    return sum((x - ma) * (y - mb) for x, y in zip(a, b)) / math.sqrt(va * vb) if va and vb else float("nan")


def decouple(tag, lo=None, hi=None):
    rows, sessions = load(tag)
    rows = dm_filter(rows, lo, hi)
    mine = daily(rows)
    print(f"== decoupling: {tag}" + (f"  day_move in [{lo}, {hi})" if lo or hi else ""))
    for other in (V18, SQRT_BAND):
        orows, osess = load(other)
        od = daily(orows)
        days = sorted(sessions & osess)
        a = [mine.get(d, 0.0) for d in days]
        b = [od.get(d, 0.0) for d in days]
        both = [d for d in days if d in mine and d in od]
        cb = corr([mine[d] for d in both], [od[d] for d in both]) if len(both) > 2 else float("nan")
        act_m = sum(1 for d in days if d in mine)
        act_o = sum(1 for d in days if d in od)
        union = sum(1 for d in days if d in mine or d in od)
        comb = orows + rows
        cd = daily(comb)
        oneg = sum(1 for d in both if od[d] < 0)
        hedge = sum(1 for d in both if od[d] < 0 and mine[d] > 0)
        print(f"  vs {other}: {len(days)} common sessions; daily corr (all, 0-filled) {corr(a, b):+.3f}, "
              f"on {len(both)} shared days {cb:+.3f}")
        print(f"    active days: {other} {act_o} ({100 * act_o / len(days):.0f}%), this {act_m} ({100 * act_m / len(days):.0f}%), "
              f"overlap {len(both)}, union {union} ({100 * union / len(days):.0f}%); "
              f"on {oneg} shared days {other} lost, this book won on {hedge}")
        print(f"    {other:15s} alone: P&L {sum(r['pnl'] for r in orows):+7.0f}  PF {pf(orows):4.2f}  "
              f"DD {max_dd(od, days):+6.0f}  worst day {min(od.values()):+5.0f}")
        print(f"    combined book     : P&L {sum(r['pnl'] for r in comb):+7.0f}  PF {pf(comb):4.2f}  "
              f"DD {max_dd(cd, days):+6.0f}  worst day {min(cd.values()):+5.0f}  "
              f"years+ {sum(1 for y in YEARS if sum(v for d, v in cd.items() if d.startswith(y)) > 0)}/5  "
              + " ".join(f"{y}:{sum(v for d, v in cd.items() if d.startswith(y)):+.0f}" for y in YEARS))


THRESH = [None, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.2]


def dmgrid(tag):
    rows, sessions = load(tag)
    print(f"== day_move_pct floor grid: {tag}")
    for t in THRESH:
        r = dm_filter(rows, t)
        ys = " ".join(f"{sum(x['pnl'] for x in r if x['year'] == y):+5.0f}" for y in YEARS)
        print(f"  >= {str(t):5s} " + line(r, sessions) + f"  | {ys}")


def loyo(tag):
    """hold out each year, pick the floor maximising PF (>= 40 trades/yr) on the other four, score it."""
    rows, sessions = load(tag)
    print(f"== LOYO day_move floor: {tag}")
    tot = []
    for hold in YEARS:
        best, bpf = None, -1
        for t in THRESH:
            tr = [r for r in dm_filter(rows, t) if r["year"] != hold]
            if len(tr) < 40 * 3.7:
                continue
            if pf(tr) > bpf:
                best, bpf = t, pf(tr)
        test = [r for r in dm_filter(rows, best) if r["year"] == hold]
        base = [r for r in rows if r["year"] == hold]
        tot += test
        print(f"  hold {hold}: pick >= {best} (train PF {bpf:.2f}) -> held-out {len(test)} tr P&L "
              f"{sum(r['pnl'] for r in test):+.0f} PF {pf(test):.2f}  (unfiltered {sum(r['pnl'] for r in base):+.0f} PF {pf(base):.2f})")
    print(f"  stitched held-out: {len(tot)} tr P&L {sum(r['pnl'] for r in tot):+.0f} PF {pf(tot):.2f}")


def main():
    a = sys.argv[1:]
    cmd = a[0]

    def opt(name):
        return float(a[a.index(name) + 1]) if name in a else None
    tags = [x for x in a[1:] if not x.startswith("--") and not x.replace(".", "").isdigit()]
    lo, hi = opt("--dm-min"), opt("--dm-max")
    for t in tags:
        if cmd == "card":
            card(t, lo, hi)
        elif cmd == "grid":
            r, s = load(t)
            print(f"{t:28s} " + line(dm_filter(r, lo, hi), s))
        elif cmd == "decouple":
            decouple(t, lo, hi)
        elif cmd == "dmgrid":
            dmgrid(t)
        elif cmd == "loyo":
            loyo(t)


if __name__ == "__main__":
    main()
