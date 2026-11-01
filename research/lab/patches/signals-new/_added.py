#!/usr/bin/env python3
"""signals-new: additivity check + added-population stats.
usage: _added.py CELL WINDOW [--base base__v18] [--new-prefix REASONPREFIX ...]
compares research/lab/runs/signals-new__CELL.WINDOW.trades.csv against the base run on the same window:
baseline trades kept / displaced, added trades (entry_reason not a v18 window) by year, worst day, combined book."""
import csv, sys
from collections import defaultdict
from pathlib import Path

RUNS = Path(__file__).resolve().parents[2] / "runs"
V18 = {"window:5m thrust short", "window:strong core short"}


def load(p):
    with open(p) as f:
        return [r for r in csv.DictReader(f) if r.get("row_type", "trade") == "trade"]


def pf(x):
    g = sum(v for v in x if v > 0); l = -sum(v for v in x if v < 0)
    return g / l if l > 0 else (99.0 if g > 0 else 0.0)


def block(rows):
    p = [float(r["pnl"]) for r in rows]
    days = defaultdict(float)
    for r in rows: days[r["date"]] += float(r["pnl"])
    return f"{sum(p):+8.0f} / {len(p):4d} / PF {pf(p):5.2f} / worst day {min(days.values()) if days else 0:+6.0f}"


def by_year(rows):
    d = defaultdict(list)
    for r in rows: d[r["date"][:4]].append(float(r["pnl"]))
    return "  ".join(f"{y}:{sum(v):+.0f}/{len(v)}/{pf(v):.2f}" for y, v in sorted(d.items()))


def main():
    cell, window = sys.argv[1], sys.argv[2]
    base = sys.argv[3] if len(sys.argv) > 3 else "base__v18"
    rows = load(RUNS / f"signals-new__{cell}.{window}.trades.csv")
    brows = load(RUNS / f"{base}.{window}.trades.csv")
    key = lambda r: (r["date"], r["ticker"], r["entry_time"], r["exit_time"], r["pnl"])
    bset = {key(r) for r in brows}
    old = [r for r in rows if r["entry_reason"] in V18]
    new = [r for r in rows if r["entry_reason"] not in V18]
    kept = [r for r in old if key(r) in bset]
    oset = {key(r) for r in old}
    lost = [r for r in brows if key(r) not in oset]
    print(f"== {cell} on {window}")
    print(f"  base       {block(brows)}")
    print(f"  baseline kept {len(kept)}/{len(brows)}; base trades missing/changed {len(lost)} ({sum(float(r['pnl']) for r in lost):+.0f}); "
          f"v18-window trades new/changed {len(old) - len(kept)} ({sum(float(r['pnl']) for r in old if key(r) not in bset):+.0f})")
    print(f"  ADDED      {block(new)}")
    print(f"     by year {by_year(new)}")
    for d in sorted({r['direction'] for r in new}):
        print(f"     {d:6s} {block([r for r in new if r['direction'] == d])}")
    tick = defaultdict(list)
    for r in new: tick[r["ticker"]].append(float(r["pnl"]))
    top = sorted(tick.items(), key=lambda kv: -abs(sum(kv[1])))[:8]
    print("     tickers " + "  ".join(f"{t}:{sum(v):+.0f}/{len(v)}" for t, v in top))
    ex = defaultdict(list)
    for r in new: ex[r["exit_reason"]].append(float(r["pnl"]))
    print("     exits   " + "  ".join(f"{t}:{sum(v):+.0f}/{len(v)}" for t, v in sorted(ex.items())))
    print(f"  COMBINED   {block(rows)}")
    print(f"     by year {by_year(rows)}")


if __name__ == "__main__":
    main()
