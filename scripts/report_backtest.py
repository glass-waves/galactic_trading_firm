#!/usr/bin/env python3
"""summarise data/<tag>_<year>_trades.csv files produced by backtest_range.sh.

usage: scripts/report_backtest.py <tag> [<tag> ...]        # one block per tag
       scripts/report_backtest.py --compare <tag> <tag>...  # side-by-side headline table

daily P&L comes from the summary rows (one per ticker per day), so drawdown is on
the day-to-day equity curve at fixed $CAPITAL per day (no compounding).
"""
import csv, glob, sys, math
from collections import defaultdict

def load(tag):
    trades, daily = [], defaultdict(float)
    days = set()
    for f in sorted(glob.glob(f"data/{tag}_*_trades.csv")):
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

def headline(tag):
    trades, daily, days = load(tag)
    pnls = [float(t["pnl"]) for t in trades]
    wins = [p for p in pnls if p > 0]
    mdd, longest = drawdown(daily)
    return dict(tag=tag, days=len(days), trades=len(trades), pnl=sum(pnls),
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
    if not args: print(__doc__); sys.exit(1)
    if args[0] == "--compare":
        print(f"{'tag':<14}{'days':>5}{'trades':>7}{'t/day':>6}{'P&L':>10}{'win%':>6}{'PF':>6}{'avg':>8}{'sharpe':>7}{'maxDD':>9}{'DDdays':>7}{'+days%':>7}")
        for tag in args[1:]:
            h = headline(tag)
            print(f"{h['tag']:<14}{h['days']:>5}{h['trades']:>7}{h['tpd']:>6.2f}{h['pnl']:>+10.2f}{h['win']:>6.1f}{h['pf']:>6.2f}"
                  f"{h['avg']:>+8.2f}{h['sharpe']:>7.2f}{h['mdd']:>9.2f}{h['dd_days']:>7}{h['pos_days']:>7.0f}")
    else:
        for tag in args: block(tag)
