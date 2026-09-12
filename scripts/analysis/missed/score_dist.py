"""score distributions of missed winners vs taken / all-free bars (supports section 3 of the report)."""
import csv, statistics as st
from common import OUT, YEARS
print("| year | set | n | median comp | median s5m | median s1h | s5m<=-0.40 | s5m<=-0.20 | s5m>0 | comp<=-0.35 | s1h<=0 |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for y in YEARS:
    rows=[r for r in csv.DictReader(open(f'{OUT}/bars_{y}.csv')) if r['s5m'] and r['cat']!='NOSCORE']
    for r in rows:
        for k in ('comp','s5m','s1h','opp_pnl_pct'): r[k]=float(r[k])
    thr=sorted(r['opp_pnl_pct'] for r in rows)[int(0.95*(len(rows)-1))]
    def single(r): return sum(int(r[c]) for c in ('c_fail','t_s5_fail','t_lag_fail','t_1h_fail'))==1 or sum(int(r[c]) for c in ('c_fail','k_s5_fail','k_1h_fail'))==1
    sets={'FREE top5 winners':[r for r in rows if r['cat']=='FREE' and r['opp_pnl_pct']>=thr],
          '  of which need>=2 changes':[r for r in rows if r['cat']=='FREE' and r['opp_pnl_pct']>=thr and not single(r)],
          'GATED(reject) top5 winners':[r for r in rows if r['cat']=='GATED' and r['opp_pnl_pct']>=thr],
          'ALL FREE':[r for r in rows if r['cat']=='FREE'],
          'TAKEN':[r for r in rows if r['cat']=='TAKEN']}
    for name,v in sets.items():
        n=len(v)
        print(f"| {y} | {name} | {n} | {st.median(r['comp'] for r in v):+.2f} | {st.median(r['s5m'] for r in v):+.2f} | {st.median(r['s1h'] for r in v):+.2f} | {100*sum(1 for r in v if r['s5m']<=-0.40)/n:.0f}% | {100*sum(1 for r in v if r['s5m']<=-0.20)/n:.0f}% | {100*sum(1 for r in v if r['s5m']>0)/n:.0f}% | {100*sum(1 for r in v if r['comp']<=-0.35)/n:.0f}% | {100*sum(1 for r in v if r['s1h']<=0)/n:.0f}% |")
