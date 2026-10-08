#!/usr/bin/env python3
"""time-consistent-filter study tables (stdlib + scripts/pipeline/{metrics,gates}.py).

usage: analyze.py grid TAG [TAG ...]       5y, per year, gates vs iex_v18 (research/trigger/analyze.py)
       analyze.py loyo TAG [TAG ...]       leave-one-year-out (PF and P&L selectors)
       analyze.py windows|diff TAG ...     per-window subsets / kept-removed-added vs iex_v18
       analyze.py loyo_vol TAG [TAG ...]   LOYO restricted to cells with >= iex_v18's 4-year trade count
       analyze.py card TAG [TAG ...]       one card per cell: 5y, maxDD, worst day, trades and P&L per
                                           year, per half-hour of entry, per window, per clock range
"""
import datetime
import os
import sys
import zoneinfo
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "trigger"))
import analyze as tr  # noqa: E402  (entry-trigger study: grid / windows / diff / loyo)

metrics = tr.metrics
YEARS = metrics.YEARS
ET = zoneinfo.ZoneInfo("America/New_York")


def mins(r):
    t = datetime.datetime.fromisoformat(r["entry_time"]).astimezone(ET)
    return t.hour * 60 + t.minute - 570


def f(st):
    return "—" if not st or st["n"] == 0 else f"{st['pnl']:+.0f} / {st['n']} / {st['pf']:.2f}"


def card(tag):
    rows = metrics.load_trades(tag)
    st = metrics.stats(rows)
    print(f"\n### {tag}\n")
    print(f"5y {f(st)}, maxDD {st['dd']:.0f}, worst day {metrics.worst_day(rows):.0f}, "
          f"sessions with a trade {len({r['date'] for r in rows})}")
    print("\n| | " + " | ".join(YEARS) + " |")
    print("|---|" + "---|" * len(YEARS))
    print("| P&L / n / PF | " + " | ".join(f(metrics.stats([r for r in rows if r['year'] == y])) for y in YEARS) + " |")
    by = defaultdict(list)
    for r in rows:
        by[min(mins(r) // 30, 12)].append(r)
    print("\n| entry half-hour | P&L / n / PF | " + " | ".join(YEARS) + " |")
    print("|---|---|" + "---|" * len(YEARS))
    for h in sorted(by):
        lab = f"{(570 + 30 * h) // 60:02d}:{(570 + 30 * h) % 60:02d}"
        rs = by[h]
        print(f"| {lab} | {f(metrics.stats(rs))} | " + " | ".join(f"{sum(r['pnl'] for r in rs if r['year'] == y):+.0f}" for y in YEARS) + " |")
    byw = defaultdict(list)
    for r in rows:
        byw[r["entry_reason"].replace("window:", "")].append(r)
    print("\n| window | 5y | " + " | ".join(YEARS) + " |")
    print("|---|---|" + "---|" * len(YEARS))
    for w, rs in sorted(byw.items()):
        print(f"| {w} | {f(metrics.stats(rs))} | " + " | ".join(f(metrics.stats([r for r in rs if r['year'] == y])) for y in YEARS) + " |")
    for name, a, b in (("before 11:30", 0, 120), ("11:30 and later", 120, 999)):
        rs = [r for r in rows if a <= mins(r) < b]
        print(f"\n{name}: {f(metrics.stats(rs))}; per year " + " ".join(f"{sum(r['pnl'] for r in rs if r['year'] == y):+.0f}" for y in YEARS))


def loyo_vol(tags):
    """leave-one-year-out under the owner's volume constraint: on four years, among the cells with at
    least iex_v18's four-year trade count, pick the best pooled PF; report its held-out year."""
    cand = [tr.BASE] + tags
    data = {t: metrics.load_trades(t) for t in cand}
    print("| held out | chosen (4-yr PF, n) | held-out year | iex_v18 held-out year |")
    print("|---|---|---|---|")
    tot = defaultdict(float)
    for y in YEARS:
        four = {t: metrics.stats([r for r in data[t] if r["year"] != y]) for t in cand}
        nb = four[tr.BASE]["n"]
        ok = [t for t in cand if four[t]["n"] >= nb]
        best = max(ok, key=lambda t: four[t]["pf"])
        h = metrics.stats([r for r in data[best] if r["year"] == y])
        hb = metrics.stats([r for r in data[tr.BASE] if r["year"] == y])
        tot["c"] += h["pnl"]; tot["b"] += hb["pnl"]; tot["nc"] += h["n"]; tot["nb"] += hb["n"]
        print(f"| {y} | {best} ({four[best]['pf']:.2f}, {four[best]['n']}) | {f(h)} | {f(hb)} |")
    print(f"| sum | | {tot['c']:+.0f} / {tot['nc']:.0f} | {tot['b']:+.0f} / {tot['nb']:.0f} |")


def main():
    cmd, tags = sys.argv[1], sys.argv[2:]
    if cmd == "card":
        for t in tags:
            card(t)
    elif cmd == "loyo_vol":
        loyo_vol(tags)
    elif cmd == "diff":
        for t in tags:
            tr.diff(t)
    else:
        getattr(tr, cmd)(tags)


if __name__ == "__main__":
    main()
