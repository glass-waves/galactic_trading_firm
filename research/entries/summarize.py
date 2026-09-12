#!/usr/bin/env python3
"""per-year, per-entry-window summary of sweep tags (entry research round two).

usage: summarize.py TAG [TAG ...] [--by-window] [--min-trades N]
reads data/<TAG>_<year>_trades.csv. prints one line per tag-year (and per window with
--by-window): P&L, trades, win %, profit factor, max drawdown (on the daily curve).
"""
import csv
import glob
import re
import sys
from collections import defaultdict

YEARS = ["2022", "2023", "2024", "2025", "2026"]


def load(tag):
    rows = []
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"] == "trade":
                    r["year"] = y
                    r["pnl"] = float(r["pnl"])
                    rows.append(r)
    return rows


def stats(rows):
    if not rows:
        return "        —"
    pnl = sum(r["pnl"] for r in rows)
    wins = [r["pnl"] for r in rows if r["pnl"] > 0]
    losses = [-r["pnl"] for r in rows if r["pnl"] < 0]
    pf = (sum(wins) / sum(losses)) if losses else float("inf")
    daily = defaultdict(float)
    for r in rows:
        daily[r["date"]] += r["pnl"]
    eq, peak, dd = 0.0, 0.0, 0.0
    for d in sorted(daily):
        eq += daily[d]
        peak = max(peak, eq)
        dd = min(dd, eq - peak)
    return f"{pnl:+8.0f} n={len(rows):4d} win={len(wins)/len(rows)*100:4.0f}% PF={pf:4.2f} dd={dd:6.0f}"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    by_window = "--by-window" in sys.argv
    for tag in args:
        rows = load(tag)
        print(f"== {tag}: 5y {stats(rows)}")
        for y in YEARS:
            yr = [r for r in rows if r["year"] == y]
            print(f"   {y} {stats(yr)}")
            if by_window:
                for w in sorted({r["entry_reason"] for r in yr}):
                    wr = [r for r in yr if r["entry_reason"] == w]
                    print(f"        {w[:40]:40s} {stats(wr)}")


if __name__ == "__main__":
    main()
