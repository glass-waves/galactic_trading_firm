#!/usr/bin/env python3
"""split a cell's tier-2 trades into front-runners (a v18 trade on the same ticker would have
fired during their hold) and genuinely new trades. prints P&L / n / PF of each, and the v18 P&L of
the trades the front-runners replaced, plus the median lead time."""
import csv, glob, sys, statistics
from datetime import datetime
YEARS=["2022","2023","2024","2025","2026"]
def load(tag):
    rows=[]
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"]=="trade": r["pnl"]=float(r["pnl"]); r["year"]=y; rows.append(r)
    return rows
def st(rows):
    if not rows: return "     —  n=   0"
    p=sum(r["pnl"] for r in rows); w=[r["pnl"] for r in rows if r["pnl"]>0]; l=[-r["pnl"] for r in rows if r["pnl"]<0]
    return f"{p:+7.0f} n={len(rows):4d} win={len(w)/len(rows)*100:3.0f}% PF={(sum(w)/sum(l)) if l else float('inf'):4.2f}"
base=load("iex_v18"); bk={(r["ticker"],r["entry_time"]) for r in base}
ts=lambda s: datetime.fromisoformat(s)
for tag in sys.argv[1:]:
    cell=load(tag)
    t2=[r for r in cell if (r["ticker"],r["entry_time"]) not in bk]
    fr,new,repl,lead=[],[],[],[]
    for r in t2:
        hit=[b for b in base if b["ticker"]==r["ticker"] and b["date"]==r["date"] and r["entry_time"]<=b["entry_time"]<r["exit_time"]]
        if hit: fr.append(r); repl+=hit; lead.append((ts(hit[0]["entry_time"])-ts(r["entry_time"])).total_seconds()/60)
        else: new.append(r)
    print(f"== {tag}")
    print(f"   tier-2 front-runners {st(fr)}   replaced v18 trades {st(repl)}   median lead {statistics.median(lead) if lead else 0:.0f} min")
    print(f"   tier-2 genuinely new {st(new)}")
    for y in YEARS:
        print(f"      {y} front {st([r for r in fr if r['year']==y])}  new {st([r for r in new if r['year']==y])}")
