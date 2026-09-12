#!/usr/bin/env python3
"""summarise data/<tag>_<year>_trades.csv files produced by backtest_range.sh.

usage: scripts/report_backtest.py <tag> [<tag> ...]        # one block per tag
       scripts/report_backtest.py --compare <tag> <tag>...  # side-by-side headline table

daily P&L comes from the summary rows (one per ticker per day), so drawdown is on
the day-to-day equity curve at fixed $CAPITAL per day (no compounding).
"""
import csv, glob, sys, math, re
from collections import defaultdict

YEAR = None  # set via --year YYYY to restrict to one calendar-year file

def load(tag):
    trades, daily = [], defaultdict(float)
    days = set()
    for f in sorted(f for f in glob.glob(f"data/{tag}_*_trades.csv")
                    if re.fullmatch(rf"data/{re.escape(tag)}_\d{{4}}_trades\.csv", f)
                    and (YEAR is None or f"_{YEAR}_" in f)):
        with open(f) as fh:
            for row in csv.DictReader(fh):
                if row.get("row_type") == "trade":
                    trades.append(row)
                elif row.get("row_type") == "summary":
                    daily[row["date"]] += float(row["pnl"] or 0)
                    days.add(row["date"])
    return trades, dict(sorted(daily.items())), sorted(days)

def pf(pnls):
    g = sum(p for p in pnls if p > 0); l = -sum(p for p in pnls if p < 0)
    return g / l if l > 0 else float("inf")

def drawdown(daily):
    eq = peak = 0.0; mdd = 0.0; cur = 0
    longest = 0
    for _, p in daily.items():
        eq += p
        if eq > peak: peak = eq; cur = 0
        else: cur += 1; longest = max(longest, cur)
        mdd = min(mdd, eq - peak)
    return mdd, longest

def sharpe(daily):
    xs = list(daily.values())
    if len(xs) < 2: return float("nan")
    m = sum(xs) / len(xs); v = sum((x - m) ** 2 for x in xs) / (len(xs) - 1)
    return (m / math.sqrt(v)) * math.sqrt(252) if v > 0 else float("nan")

HOLDOUT_START = "2026-07-01"

def headline(tag):
    trades, daily, days = load(tag)
    pnls = [float(t["pnl"]) for t in trades]
    wins = [p for p in pnls if p > 0]
    mdd, longest = drawdown(daily)
    h1 = sum(float(t["pnl"]) for t in trades if t["date"] < HOLDOUT_START)
    h2 = sum(float(t["pnl"]) for t in trades if t["date"] >= HOLDOUT_START)
    n2 = sum(1 for t in trades if t["date"] >= HOLDOUT_START)
    return dict(tag=tag, days=len(days), trades=len(trades), pnl=sum(pnls), h1=h1, h2=h2, n2=n2,
                win=100 * len(wins) / len(pnls) if pnls else 0, pf=pf(pnls),
                avg=sum(pnls) / len(pnls) if pnls else 0, tpd=len(trades) / len(days) if days else 0,
                mdd=mdd, dd_days=longest, sharpe=sharpe(daily),
                pos_days=100 * sum(1 for p in daily.values() if p > 0) / len(daily) if daily else 0)

def block(tag):
    trades, daily, days = load(tag)
    h = headline(tag)
    print(f"=== {tag}: {h['days']} days, {h['trades']} trades ({h['tpd']:.2f}/day) ===")
    print(f"  P&L ${h['pnl']:+.2f}  win {h['win']:.1f}%  PF {h['pf']:.2f}  avg ${h['avg']:+.2f}/trade  "
          f"sharpe(daily) {h['sharpe']:.2f}  maxDD ${h['mdd']:.2f}  longest DD {h['dd_days']}d  positive days {h['pos_days']:.0f}%")
    def table(title, key):
        agg = defaultdict(list)
        for t in trades: agg[key(t)].append(float(t["pnl"]))
        print(f"  -- by {title}")
        for k, v in sorted(agg.items(), key=lambda kv: -sum(kv[1])):
            w = 100 * sum(1 for p in v if p > 0) / len(v)
            print(f"     {str(k):<28} n={len(v):<4} pnl ${sum(v):+9.2f}  win {w:5.1f}%  PF {pf(v):5.2f}")
    table("month", lambda t: t["date"][:7])
    table("ticker", lambda t: t["ticker"])
    table("entry window", lambda t: t.get("entry_reason") or "?")
    table("exit reason", lambda t: t["exit_reason"])
    hold = [int(t["hold_duration_ms"]) / 60000 for t in trades]
    if hold: print(f"  hold minutes: mean {sum(hold)/len(hold):.0f}, max {max(hold):.0f}")
    print()

if __name__ == "__main__":
    args = sys.argv[1:]
    if "--year" in args:
        i = args.index("--year"); YEAR = args[i + 1]; del args[i:i + 2]
    if not args: print(__doc__); sys.exit(1)
    if args[0] == "--compare":
        print(f"{'tag':<16}{'days':>5}{'trades':>7}{'t/day':>6}{'P&L':>9}{'win%':>6}{'PF':>6}{'avg':>7}{'sharpe':>7}{'maxDD':>8}{'DDd':>5}{'tune':>8}{'holdout':>9}{'n_ho':>5}")
        for tag in args[1:]:
            h = headline(tag)
            print(f"{h['tag']:<16}{h['days']:>5}{h['trades']:>7}{h['tpd']:>6.2f}{h['pnl']:>+9.0f}{h['win']:>6.1f}{h['pf']:>6.2f}"
                  f"{h['avg']:>+7.2f}{h['sharpe']:>7.2f}{h['mdd']:>8.0f}{h['dd_days']:>5}{h['h1']:>+8.0f}{h['h2']:>+9.0f}{h['n2']:>5}")
    else:
        for tag in args: block(tag)
