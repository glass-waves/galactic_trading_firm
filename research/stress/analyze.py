#!/usr/bin/env python3
"""stress-mode study tables: every cell vs the baseline (iex_v18) on the five-year IEX replay.

usage: analyze.py [--base iex_v18] [--only TAG] TAG [TAG ...]
prints, per tag: 5y + per-year vs baseline, the stress windows' own subset, the tail-day table
(SPY session return < -1 %, < -2 %, the 25 worst SPY days), daily-P&L correlation with SPY,
ordinary-day P&L (SPY >= -1 %) vs baseline, false alarms (stress-trade days where SPY closed
> -0.25 %), the worst single day, pre-emption counts, and the `stress-mode` gate verdict.
`--only TAG` names the matching stress-only cell (stress windows without the morning windows)
used to count stress entries pre-empted by an open morning position.
SPY session return = open of the first RTH bar -> close of the last, from data/bars_iex/SPY.csv.
"""
import csv
import datetime
import glob
import math
import sys
import zoneinfo
from collections import defaultdict

YEARS = ["2022", "2023", "2024", "2025", "2026"]
ET = zoneinfo.ZoneInfo("US/Eastern")
LAST_DAY = "2026-09-10"


def spy_days():
    days = {}
    with open("data/bars_iex/SPY.csv") as f:
        for r in csv.DictReader(f):
            d = datetime.datetime.fromtimestamp(int(r["timestamp"]), ET).date().isoformat()
            c = float(r["close"])
            if d not in days:
                days[d] = [float(r["open"]), c, c]
            else:
                days[d][1] = c
                days[d][2] = min(days[d][2], c)
    ret = {d: (v[1] / v[0] - 1) * 100 for d, v in days.items() if d <= LAST_DAY}
    low = {d: (v[2] / v[0] - 1) * 100 for d, v in days.items() if d <= LAST_DAY}
    return ret, low


def load(tag):
    trades, dates = [], set()
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"] == "summary":
                    dates.add(r["date"])
                elif r["row_type"] == "trade":
                    r["pnl"] = float(r["pnl"])
                    r["year"] = r["date"][:4]
                    r["stress"] = "stress" in r["entry_reason"]
                    trades.append(r)
    return trades, dates


def pf(rows):
    w = sum(r["pnl"] for r in rows if r["pnl"] > 0)
    l = -sum(r["pnl"] for r in rows if r["pnl"] < 0)
    return w / l if l else float("inf")


def daily(rows):
    d = defaultdict(float)
    for r in rows:
        d[r["date"]] += r["pnl"]
    return d


def corr(xs, ys):
    n = len(xs)
    if n < 3:
        return float("nan")
    mx, my = sum(xs) / n, sum(ys) / n
    sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    sxx = sum((x - mx) ** 2 for x in xs)
    syy = sum((y - my) ** 2 for y in ys)
    return sxy / math.sqrt(sxx * syy) if sxx > 0 and syy > 0 else float("nan")


def fmt_stats(rows):
    if not rows:
        return "0 / 0 / —"
    return f"{sum(r['pnl'] for r in rows):+.0f} / {len(rows)} / {pf(rows):.2f}"


def key(r):
    return (r["ticker"], r["entry_time"])


def tail_line(label, days, dn, bn, n_days, tr, btr):
    ds = set(days)
    p = sum(dn.get(d, 0.0) for d in days)
    bp = sum(bn.get(d, 0.0) for d in days)
    n = sum(1 for r in tr if r["date"] in ds)
    ns = sum(1 for r in tr if r["date"] in ds and r["stress"])
    bn_ = sum(1 for r in btr if r["date"] in ds)
    worst = min((dn.get(d, 0.0) for d in days), default=0.0)
    bworst = min((bn.get(d, 0.0) for d in days), default=0.0)
    return (f"| {label} ({n_days} d) | {p:+.0f} | {p / n_days:+.1f} | {n} ({ns} stress) | {worst:+.0f} "
            f"| {bp:+.0f} | {bp / n_days:+.1f} | {bn_} | {bworst:+.0f} |")


def grid(tags, base):
    """one row per tag: the report's grid table."""
    ret, low = spy_days()
    btr, bdates = load(base)
    bday = daily(btr)
    tail1 = [d for d in ret if ret[d] < -1.0 and d in bdates]
    tail2 = [d for d in tail1 if ret[d] < -2.0]
    ordinary = [d for d in bdates if d in ret and ret[d] >= -1.0]
    bt1 = sum(bday.get(d, 0.0) for d in tail1); bop = sum(bday.get(d, 0.0) for d in ordinary)
    print("| cell | 5y P&L / n / PF | +yrs | stress subset | SPY<-1 % (117 d) | SPY<-2 % (25 d) | ordinary Δ | corr | worst day | false alarms | gate |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for tag in [base] + tags:
        tr, dates = load(tag)
        if not tr:
            continue
        dn = daily(tr)
        st = [r for r in tr if r["stress"]]
        pos = sum(1 for y in YEARS if sum(r["pnl"] for r in tr if r["year"] == y) > 0)
        t1 = sum(dn.get(d, 0.0) for d in tail1); t2 = sum(dn.get(d, 0.0) for d in tail2)
        op = sum(dn.get(d, 0.0) for d in ordinary)
        ds = sorted(d for d in dates if d in ret)
        c = corr([ret[d] for d in ds], [dn.get(d, 0.0) for d in ds])
        wd = min(dn.values()) if dn else 0.0
        sd = defaultdict(list)
        for r in st:
            sd[r["date"]].append(r)
        fa = [r for d, v in sd.items() if ret.get(d, 0) > -0.25 for r in v]
        checks = [pf(tr) >= 1.3, pos >= 4, len(tr) >= 0.9 * len(btr), t1 >= 3 * bt1,
                  t1 / len(tail1) >= 40, wd >= -300, bop and abs(op / bop - 1) <= 0.10]
        n1 = sum(1 for r in st if r["date"] in set(tail1)); n2 = sum(1 for r in st if r["date"] in set(tail2))
        print(f"| {tag} | {fmt_stats(tr)} | {pos} | {fmt_stats(st)} | {t1:+.0f} ({n1} st) | {t2:+.0f} ({n2} st) "
              f"| {(op / bop - 1) * 100:+.0f} % | {c:+.2f} | {wd:+.0f} | {len({r['date'] for r in fa})} d {sum(r['pnl'] for r in fa):+.0f} "
              f"| {'PASS' if all(checks) else 'fail ' + ''.join('x' if not k else '.' for k in checks)} |")


def main():
    args = sys.argv[1:]
    base = "iex_v18"
    only = None
    if "--grid" in args:
        args.remove("--grid")
        if "--base" in args:
            i = args.index("--base"); base = args[i + 1]; del args[i:i + 2]
        grid(args, base)
        return
    if "--base" in args:
        i = args.index("--base"); base = args[i + 1]; del args[i:i + 2]
    if "--only" in args:
        i = args.index("--only"); only = args[i + 1]; del args[i:i + 2]
    ret, low = spy_days()
    btr, bdates = load(base)
    bday = daily(btr)
    tail1 = sorted(d for d in ret if d < LAST_DAY + "z" and ret[d] < -1.0 and d in bdates)
    tail2 = [d for d in tail1 if ret[d] < -2.0]
    worst25 = sorted((d for d in ret if d in bdates), key=lambda d: ret[d])[:25]
    ordinary = [d for d in bdates if d in ret and ret[d] >= -1.0]
    bkeys = {key(r) for r in btr}
    only_tr = load(only)[0] if only else None
    for tag in args:
        tr, dates = load(tag)
        if not tr:
            print(f"== {tag}: no trades on disk"); continue
        dn = daily(tr)
        st = [r for r in tr if r["stress"]]
        am = [r for r in tr if not r["stress"]]
        print(f"\n## {tag}  (baseline {base})")
        print(f"5y: {fmt_stats(tr)}   baseline {fmt_stats(btr)}   (P&L / trades / PF)")
        print("| year | cell P&L / n / PF | stress subset | baseline |")
        print("|---|---|---|---|")
        pos = 0
        for y in YEARS:
            yr = [r for r in tr if r["year"] == y]
            pos += sum(r["pnl"] for r in yr) > 0
            print(f"| {y} | {fmt_stats(yr)} | {fmt_stats([r for r in yr if r['stress']])} "
                  f"| {fmt_stats([r for r in btr if r['year'] == y])} |")
        print(f"positive years: {pos}/5   stress subset 5y: {fmt_stats(st)}  "
              f"win {sum(1 for r in st if r['pnl'] > 0) / len(st) * 100 if st else 0:.0f} %")
        if st:
            by_w = defaultdict(list)
            for r in st:
                by_w[r["entry_reason"]].append(r)
            print("  by window: " + "; ".join(f"{w}: {fmt_stats(v)}" for w, v in sorted(by_w.items())))
            by_x = defaultdict(list)
            for r in st:
                by_x[r["exit_reason"]].append(r)
            print("  by exit: " + "; ".join(f"{x}: {fmt_stats(v)}" for x, v in sorted(by_x.items())))
            hold = sorted(int(r["hold_duration_ms"]) / 60000 for r in st)
            print(f"  hold min: median {hold[len(hold) // 2]:.0f}, p90 {hold[int(len(hold) * .9)]:.0f}; "
                  f"days with a stress trade: {len({r['date'] for r in st})}")
        # tail-day table
        print("| day set | P&L | per day | trades | worst day | base P&L | base/day | base trades | base worst |")
        print("|---|---|---|---|---|---|---|---|---|")
        print(tail_line("SPY < -1 %", tail1, dn, bday, len(tail1), tr, btr))
        print(tail_line("SPY < -2 %", tail2, dn, bday, len(tail2), tr, btr))
        print(tail_line("25 worst SPY", worst25, dn, bday, 25, tr, btr))
        print(tail_line("ordinary (SPY >= -1 %)", ordinary, dn, bday, len(ordinary), tr, btr))
        # correlation
        ds = sorted(d for d in dates if d in ret)
        c_all = corr([ret[d] for d in ds], [dn.get(d, 0.0) for d in ds])
        b_all = corr([ret[d] for d in ds], [bday.get(d, 0.0) for d in ds])
        dt = [d for d in ds if d in dn]
        c_tr = corr([ret[d] for d in dt], [dn[d] for d in dt])
        print(f"corr(daily P&L, SPY ret): all days {c_all:+.3f} (baseline {b_all:+.3f}); traded days {c_tr:+.3f}")
        # ordinary-day delta, worst day
        op = sum(dn.get(d, 0.0) for d in ordinary); bop = sum(bday.get(d, 0.0) for d in ordinary)
        nost = [d for d in ds if d not in {r["date"] for r in st}]
        print(f"ordinary-day P&L {op:+.0f} vs baseline {bop:+.0f} ({(op / bop - 1) * 100:+.1f} %); "
              f"days without a stress trade: {sum(dn.get(d, 0) for d in nost):+.0f} vs {sum(bday.get(d, 0) for d in nost):+.0f}")
        wd = min(dn.items(), key=lambda kv: kv[1]) if dn else ("-", 0)
        bwd = min(bday.items(), key=lambda kv: kv[1]) if bday else ("-", 0)
        print(f"worst single day: {wd[1]:+.0f} on {wd[0]} (SPY {ret.get(wd[0], float('nan')):+.2f} %); "
              f"baseline {bwd[1]:+.0f} on {bwd[0]}")
        # false alarms: stress-trade days where SPY closed > -0.25 %
        if st:
            sd = defaultdict(list)
            for r in st:
                sd[r["date"]].append(r)
            fa = {d: v for d, v in sd.items() if ret.get(d, 0) > -0.25}
            held = {d: v for d, v in sd.items() if ret.get(d, 0) <= -1.0}
            mid = {d: v for d, v in sd.items() if -1.0 < ret.get(d, 0) <= -0.25}
            f_rows = [r for v in fa.values() for r in v]
            h_rows = [r for v in held.values() for r in v]
            m_rows = [r for v in mid.values() for r in v]
            print(f"false alarms (stress day, SPY closed > -0.25 %): {len(fa)} days, {fmt_stats(f_rows)}; "
                  f"SPY closed <= -1 %: {len(held)} days, {fmt_stats(h_rows)}; between: {len(mid)} days, {fmt_stats(m_rows)}")
        # pre-emption
        ck = {key(r) for r in am}
        lost = [r for r in btr if key(r) not in ck]
        new = [r for r in am if key(r) not in bkeys]
        print(f"morning book vs baseline: {len(am)} trades, {fmt_stats(am)}; baseline entries lost {len(lost)} "
              f"({sum(r['pnl'] for r in lost):+.0f}), new morning entries {len(new)} ({sum(r['pnl'] for r in new):+.0f})")
        if only_tr is not None:
            ok = {key(r) for r in st}
            pre = [r for r in only_tr if key(r) not in ok]
            print(f"stress-only cell {only}: {fmt_stats(only_tr)}; stress entries pre-empted by a morning "
                  f"position/cooldown: {len(pre)} ({sum(r['pnl'] for r in pre):+.0f})")
        # gate
        t1 = sum(dn.get(d, 0.0) for d in tail1); bt1 = sum(bday.get(d, 0.0) for d in tail1)
        checks = {
            "PF >= 1.3": pf(tr) >= 1.3,
            ">= 4/5 years positive": pos >= 4,
            "trades >= 0.9 x baseline": len(tr) >= 0.9 * len(btr),
            "SPY<-1% P&L >= 3 x baseline": t1 >= 3 * bt1,
            "SPY<-1% >= +40/day": t1 / len(tail1) >= 40,
            "no day < -300": wd[1] >= -300,
            "ordinary within +-10 %": abs(op / bop - 1) <= 0.10 if bop else False,
        }
        print("gate stress-mode: " + ("PASS" if all(checks.values()) else "FAIL") + " — " +
              ", ".join(f"{k}: {'ok' if v else 'NO'}" for k, v in checks.items()))


if __name__ == "__main__":
    main()
