#!/usr/bin/env python3
"""tier-2 breakdown for study B (tiered sizing below the VPIN floor).

usage: tier2_breakdown.py TAG [TAG ...]
for each tag: splits its trades into tier-1 (ticker+entry_time present in iex_v18's trade set)
and tier-2 (not present), prints P&L / n / win% / PF for the full run, the two subsets, and
the tier-1 subset's delta vs iex_v18. also prints per-year tier-2 P&L.
"""
import csv, glob, sys
from collections import defaultdict

YEARS = ["2022", "2023", "2024", "2025", "2026"]

def load(tag):
    rows = []
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"] == "trade":
                    r["year"] = y; r["pnl"] = float(r["pnl"]); r["size"] = float(r["size"])
                    rows.append(r)
    return rows

def st(rows):
    if not rows: return "     —  n=   0"
    pnl = sum(r["pnl"] for r in rows)
    w = [r["pnl"] for r in rows if r["pnl"] > 0]; l = [-r["pnl"] for r in rows if r["pnl"] < 0]
    pf = sum(w)/sum(l) if l else float("inf")
    return f"{pnl:+7.0f} n={len(rows):4d} win={len(w)/len(rows)*100:3.0f}% PF={pf:4.2f}"

base = load("iex_v18")
base_keys = {(r["ticker"], r["entry_time"]) for r in base}
base_pnl = sum(r["pnl"] for r in base)
print(f"iex_v18 reference: {st(base)}")
for tag in sys.argv[1:]:
    rows = load(tag)
    t1 = [r for r in rows if (r["ticker"], r["entry_time"]) in base_keys]
    t2 = [r for r in rows if (r["ticker"], r["entry_time"]) not in base_keys]
    missing = base_keys - {(r["ticker"], r["entry_time"]) for r in rows}
    miss_pnl = sum(r["pnl"] for r in base if (r["ticker"], r["entry_time"]) in missing)
    t1_pnl = sum(r["pnl"] for r in t1)
    print(f"== {tag}")
    print(f"   all    {st(rows)}")
    print(f"   tier-1 {st(t1)}   Δ vs iex_v18 {t1_pnl-base_pnl:+.0f}; v18 trades missing here: {len(missing)} (their v18 P&L {miss_pnl:+.0f})")
    print(f"   tier-2 {st(t2)}   share of trades {len(t2)/len(rows)*100:.0f}%  mean size t2/t1 = {sum(r['size'] for r in t2)/max(1,len(t2)):.1f}/{sum(r['size'] for r in t1)/max(1,len(t1)):.1f} sh")
    for y in YEARS:
        print(f"      {y} tier-2 {st([r for r in t2 if r['year']==y])}   tier-1 {st([r for r in t1 if r['year']==y])}")
