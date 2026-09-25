#!/usr/bin/env python3
"""why do iex_v18 trades go missing in a tiered cell? classifies each missing v18 trade by
what the cell had open at that entry time: same ticker in position (tier-2 or tier-1),
3 positions open (concurrency cap), or neither (loss breaker / cooldown / other)."""
import csv, glob, sys
from collections import Counter
YEARS = ["2022","2023","2024","2025","2026"]
def load(tag):
    rows=[]
    for y in YEARS:
        for f in glob.glob(f"data/{tag}_{y}_trades.csv"):
            for r in csv.DictReader(open(f)):
                if r["row_type"]=="trade": r["pnl"]=float(r["pnl"]); rows.append(r)
    return rows
base=load("iex_v18"); bk={(r["ticker"],r["entry_time"]) for r in base}
for tag in sys.argv[1:]:
    cell=load(tag); ck={(r["ticker"],r["entry_time"]) for r in cell}
    missing=[r for r in base if (r["ticker"],r["entry_time"]) not in ck]
    c=Counter(); pnl=Counter()
    for m in missing:
        t=m["entry_time"]; d=m["date"]
        same=[x for x in cell if x["ticker"]==m["ticker"] and x["date"]==d and x["entry_time"]<=t<x["exit_time"]]
        openn=[x for x in cell if x["date"]==d and x["entry_time"]<=t<x["exit_time"]]
        earlier_same=[x for x in cell if x["ticker"]==m["ticker"] and x["date"]==d and x["entry_time"]<t]
        if same: k="same-ticker open (t2)" if (same[0]["ticker"],same[0]["entry_time"]) not in bk else "same-ticker open (t1)"
        elif len(openn)>=3: k="concurrency cap"
        elif earlier_same: k="earlier same-ticker trade closed (one-per-ticker/day or cooldown)"
        else: k="other (loss breaker/cooldown/no position on ticker)"
        c[k]+=1; pnl[k]+=m["pnl"]
    print(f"== {tag}: {len(missing)} v18 trades missing")
    for k,v in c.most_common(): print(f"   {v:3d}  v18 P&L {pnl[k]:+6.0f}  {k}")
