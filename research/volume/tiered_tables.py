#!/usr/bin/env python3
"""markdown tables for research/volume/tiered.md. usage: tiered_tables.py TAG[:L:M] ..."""
import csv, glob, sys
from collections import defaultdict
YEARS=["2022","2023","2024","2025","2026"]
def load(tag):
    rows=[]
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"]=="trade": r["pnl"]=float(r["pnl"]); r["year"]=y; rows.append(r)
    return rows
def m(rows):
    if not rows: return dict(pnl=0,n=0,win=0,pf=0,dd=0)
    p=sum(r["pnl"] for r in rows); w=[r["pnl"] for r in rows if r["pnl"]>0]; l=[-r["pnl"] for r in rows if r["pnl"]<0]
    daily=defaultdict(float)
    for r in rows: daily[r["date"]]+=r["pnl"]
    eq=peak=dd=0.0
    for d in sorted(daily):
        eq+=daily[d]; peak=max(peak,eq); dd=min(dd,eq-peak)
    return dict(pnl=p,n=len(rows),win=len(w)/len(rows)*100,pf=(sum(w)/sum(l)) if l else 99,dd=dd)
base=load("iex_v18"); bk={(r["ticker"],r["entry_time"]) for r in base}; B=m(base)
print("| tag | L | M | 5y P&L | trades | win % | PF | max DD | 2022 | 2023 | 2024 | 2025 | 2026 | +yrs | Δ trades | Δ P&L |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
specs=[a.split(":") for a in sys.argv[1:]]
for s in specs:
    tag=s[0]; L=s[1] if len(s)>1 else "—"; M=s[2] if len(s)>2 else "—"
    rows=load(tag); a=m(rows)
    yrs=[m([r for r in rows if r["year"]==y]) for y in YEARS]
    pos=sum(1 for y in yrs if y["pnl"]>0)
    ycols=" | ".join(f"{y['pnl']:+.0f} ({y['pf']:.2f})" for y in yrs)
    print(f"| {tag} | {L} | {M} | {a['pnl']:+.0f} | {a['n']} | {a['win']:.0f} | {a['pf']:.2f} | {a['dd']:.0f} | {ycols} | {pos} | {a['n']-B['n']:+d} | {a['pnl']-B['pnl']:+.0f} |")
print()
print("| tag | tier-2 n | share | tier-2 P&L | tier-2 win % | tier-2 PF | tier-2 mean sh | tier-1 n | tier-1 P&L | tier-1 Δ vs v18 | v18 trades displaced (their v18 P&L) |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for s in specs:
    tag=s[0]; rows=load(tag)
    t1=[r for r in rows if (r["ticker"],r["entry_time"]) in bk]; t2=[r for r in rows if (r["ticker"],r["entry_time"]) not in bk]
    miss=bk-{(r["ticker"],r["entry_time"]) for r in rows}; mp=sum(r["pnl"] for r in base if (r["ticker"],r["entry_time"]) in miss)
    A=m(t2); C=m(t1); sh=sum(float(r["size"]) for r in t2)/max(1,len(t2))
    print(f"| {tag} | {A['n']} | {A['n']/max(1,len(rows))*100:.0f}% | {A['pnl']:+.0f} | {A['win']:.0f} | {A['pf']:.2f} | {sh:.1f} | {C['n']} | {C['pnl']:+.0f} | {C['pnl']-B['pnl']:+.0f} | {len(miss)} ({mp:+.0f}) |")
