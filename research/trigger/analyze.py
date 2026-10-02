#!/usr/bin/env python3
"""entry-trigger study tables. stdlib + scripts/pipeline/{metrics,gates}.py (the gates the nightly
runner applies, imported so the verdicts here are the pipeline's own).

usage: analyze.py grid TAG [TAG ...]          one row per cell: 5y, per-year, Δ vs iex_v18, gates
       analyze.py windows TAG [TAG ...]        per-window subsets (5y and per year)
       analyze.py diff TAG                     kept / removed / added trades vs iex_v18 (matched on
                                               date, ticker, entry_time, direction), per year
       analyze.py loyo TAG [TAG ...]           leave-one-year-out over the given cells (+ iex_v18):
                                               choose on four years (pooled PF; P&L as a second
                                               selector), report the held-out year
       analyze.py bars TAG                     entry-bar profile of a cell's trades (scores at entry,
                                               minutes after the open, exit reasons, hold)
"""
import datetime
import sys
import zoneinfo
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "pipeline"))
import gates  # noqa: E402
import metrics  # noqa: E402

BASE = "iex_v18"
YEARS = metrics.YEARS
ET = zoneinfo.ZoneInfo("America/New_York")


def s(rows):
    return metrics.stats(rows)


def f(st):
    if not st or st["n"] == 0:
        return "—"
    return f"{st['pnl']:+.0f} / {st['n']} / {st['pf']:.2f}"


def grid(tags):
    b = metrics.summarize(BASE)
    print("| cell | 5y P&L | n | PF | maxDD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | Δn | ΔP&L | volume | quality | additive |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for tag in [BASE] + tags:
        rows = metrics.load_trades(tag)
        if not rows:
            print(f"| {tag} | (no trades on disk) |")
            continue
        m = metrics.summarize(tag)
        m["marginal"] = metrics.marginal_metrics(tag, BASE)
        yrs = " | ".join(f"{m['years'][y]['pnl']:+.0f}" for y in YEARS)
        res = []
        for g in ("volume-config", "quality-config", "additive-config"):
            r = gates.run_gate(g, m, b)
            res.append("PASS" if r["pass"] else "fail: " + ",".join(gates.failed_names(r)))
        print(f"| {tag} | {m['pnl']:+.0f} | {m['n']} | {m['pf']:.2f} | {m['dd']:.0f} | {yrs} | {m['years_positive']} "
              f"| {m['n'] - b['n']:+d} | {m['pnl'] - b['pnl']:+.0f} | {' | '.join(res)} |")


def windows(tags):
    for tag in tags:
        rows = metrics.load_trades(tag)
        by = defaultdict(list)
        for r in rows:
            by[r["entry_reason"].replace("window:", "")].append(r)
        print(f"\n{tag}")
        print("| window | 5y | " + " | ".join(YEARS) + " |")
        print("|---|---|" + "---|" * len(YEARS))
        for w, rs in sorted(by.items()):
            print(f"| {w} | {f(s(rs))} | " + " | ".join(f(s([r for r in rs if r['year'] == y])) for y in YEARS) + " |")


def diff(tag):
    rows, brows = metrics.load_trades(tag), metrics.load_trades(BASE)
    k = metrics.trade_key
    bk, ck = {k(r) for r in brows}, {k(r) for r in rows}
    kept = [r for r in rows if k(r) in bk]
    added = [r for r in rows if k(r) not in bk]
    removed = [r for r in brows if k(r) not in ck]
    # an added trade that sits on the same ticker-day as a removed one = a re-timed entry
    rem_days = {(r["date"], r["ticker"]) for r in removed}
    retimed = [r for r in added if (r["date"], r["ticker"]) in rem_days]
    new = [r for r in added if (r["date"], r["ticker"]) not in rem_days]
    print(f"\n{tag} vs {BASE}")
    print("| set | 5y | " + " | ".join(YEARS) + " |")
    print("|---|---|" + "---|" * len(YEARS))
    for name, rs in (("kept (in both)", kept), ("removed (base only)", removed),
                     ("added: re-timed (same ticker-day as a removed one)", retimed),
                     ("added: new ticker-days", new)):
        print(f"| {name} | {f(s(rs))} | " + " | ".join(f(s([r for r in rs if r['year'] == y])) for y in YEARS) + " |")
    # paired comparison of re-timed entries: minutes earlier/later than the removed base trade
    if retimed:
        bmap = defaultdict(list)
        for r in removed:
            bmap[(r["date"], r["ticker"])].append(r)
        dts, dp = [], 0.0
        for r in retimed:
            o = min(bmap[(r["date"], r["ticker"])], key=lambda x: abs(_t(x) - _t(r)))
            dts.append((_t(r) - _t(o)) / 60)
        dts.sort()
        print(f"re-timed entries vs the base entry they replaced: median {dts[len(dts) // 2]:+.0f} min "
              f"(p10 {dts[int(len(dts) * .1)]:+.0f}, p90 {dts[int(len(dts) * .9)]:+.0f})")


def _t(r):
    return datetime.datetime.fromisoformat(r["entry_time"]).timestamp()


def loyo(tags):
    cand = [BASE] + tags
    data = {t: metrics.load_trades(t) for t in cand}
    cand = [t for t in cand if data[t]]
    print("| held out | chosen by PF (4 yrs) | its held-out year | chosen by P&L (4 yrs) | its held-out year | iex_v18 held-out year |")
    print("|---|---|---|---|---|---|")
    tot = defaultdict(float)
    for y in YEARS:
        def four(t):
            return s([r for r in data[t] if r["year"] != y])
        by_pf = max(cand, key=lambda t: four(t)["pf"])
        by_pnl = max(cand, key=lambda t: four(t)["pnl"])
        hp, hq = s([r for r in data[by_pf] if r["year"] == y]), s([r for r in data[by_pnl] if r["year"] == y])
        hb = s([r for r in data[BASE] if r["year"] == y])
        tot["pf"] += hp["pnl"]; tot["pnl"] += hq["pnl"]; tot["base"] += hb["pnl"]
        print(f"| {y} | {by_pf} (PF {four(by_pf)['pf']:.2f}) | {f(hp)} | {by_pnl} ({four(by_pnl)['pnl']:+.0f}) | {f(hq)} | {f(hb)} |")
    print(f"| sum of held-out years | | {tot['pf']:+.0f} | | {tot['pnl']:+.0f} | {tot['base']:+.0f} |")


def bars(tag):
    rows = metrics.load_trades(tag)
    def mins(r):
        t = datetime.datetime.fromisoformat(r["entry_time"]).astimezone(ET)
        return t.hour * 60 + t.minute - 570
    print(f"\n{tag}: entry-bar profile (means; minutes after 09:30 ET)")
    by = defaultdict(list)
    for r in rows:
        by[r["entry_reason"].replace("window:", "")].append(r)
    by["ALL"] = rows
    print("| window | n | composite | 1m | 5m | 1h | min after open (median) | hold min (mean) | top exits |")
    print("|---|---|---|---|---|---|---|---|---|")
    for w, rs in by.items():
        def mean(key):
            v = [float(r[key]) for r in rs if r.get(key) not in (None, "")]
            return sum(v) / len(v) if v else float("nan")
        ms = sorted(mins(r) for r in rs)
        ex = defaultdict(int)
        for r in rs:
            ex[r["exit_reason"]] += 1
        top = ", ".join(f"{k} {v * 100 // len(rs)} %" for k, v in sorted(ex.items(), key=lambda kv: -kv[1])[:3])
        hold = sum(int(r["hold_duration_ms"]) for r in rs) / len(rs) / 60000
        print(f"| {w} | {len(rs)} | {mean('entry_composite'):+.2f} | {mean('entry_1m'):+.2f} | {mean('entry_5m'):+.2f} "
              f"| {mean('entry_1h'):+.2f} | {ms[len(ms) // 2]} | {hold:.0f} | {top} |")


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return
    cmd, tags = sys.argv[1], sys.argv[2:]
    {"grid": lambda: grid(tags), "windows": lambda: windows(tags), "diff": lambda: [diff(t) for t in tags],
     "loyo": lambda: loyo(tags), "bars": lambda: [bars(t) for t in tags]}[cmd]()


if __name__ == "__main__":
    main()
