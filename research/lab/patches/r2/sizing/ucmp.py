# paired day bootstrap A-B where A/B may be unions "x.json+y.json" (judge.rows_for); train window only
import sys, collections
sys.path.insert(0,'scripts/lab'); sys.path.insert(0,'scripts/pipeline')
import lab, judge, metrics as pm
w=sys.argv[3] if len(sys.argv)>3 else 'train'
start,end=lab.window_bounds(w); lab.check_reveal(w,start,end)
days=[d.isoformat() for d in lab.sessions(lab.ROOT/'data'/'bars_iex',start,end)]
na,ra,_=judge.rows_for(sys.argv[1],start,end); nb,rb,_=judge.rows_for(sys.argv[2],start,end)
def pf(x):
  gw=sum(v for v in x if v>0); gl=-sum(v for v in x if v<0); return round(gw/gl,2) if gl else 99
def blk(rows):
  p=[r['pnl'] for r in rows]; return f"{sum(p):+.0f}/{len(p)}/PF {pf(p)}"
p=pm.paired_day_bootstrap(ra,rb,session_list=days,n=5000,seed=0)
corr=pm.pearson_corr(pm.daily_series(ra,days),pm.daily_series(rb,days))
s=lab.summarize(ra,lab.sessions(lab.ROOT/'data'/'bars_iex',start,end),start,end)
print(f"A {na}: {blk(ra)}  dd {s['total']['dd']:.0f} worst day {s.get('worst_day',0):.0f} ex-best PF {s.get('ex_best_year',{}).get('pf','?')}")
print(f"B {nb}: {blk(rb)}")
print(f"paired A-B: diff {p['pnl_diff']:+.0f}, PF diff {p['pf_diff']:+.3f}, P(diff<=0) {p['p_le_0']:.3f}, corr {corr:+.2f}")
for key,f in (('year',lambda r:r['entry_time'][:4]),('ticker',lambda r:r['ticker'])):
  ga=collections.defaultdict(list); gb=collections.defaultdict(list)
  for r in ra: ga[f(r)].append(r['pnl'])
  for r in rb: gb[f(r)].append(r['pnl'])
  print(' ',key,' | '.join(f"{k}: {sum(ga[k]):+.0f}/{len(ga[k])}/{pf(ga[k])} vs {sum(gb[k]):+.0f}/{len(gb[k])}" for k in sorted(set(ga)|set(gb))))
