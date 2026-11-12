# leverage-matched paired test: scale B's P&L by total-notional ratio A/B, then paired day bootstrap
import sys
sys.path.insert(0,'scripts/lab'); sys.path.insert(0,'scripts/pipeline')
import lab, judge, metrics as pm
start,end=lab.window_bounds('train')
days=[d.isoformat() for d in lab.sessions(lab.ROOT/'data'/'bars_iex',start,end)]
na,ra,_=judge.rows_for(sys.argv[1],start,end); nb,rb,_=judge.rows_for(sys.argv[2],start,end)
no=lambda rows: sum(float(r['size'])*float(r['entry_price']) for r in rows)
k=no(ra)/no(rb)
rb2=[dict(r,pnl=r['pnl']*k) for r in rb]
p=pm.paired_day_bootstrap(ra,rb2,session_list=days,n=5000,seed=0)
print(f"{na} vs {nb} x{k:.3f} (notional-matched): A {sum(r['pnl'] for r in ra):+.0f}  B' {sum(r['pnl'] for r in rb2):+.0f}  diff {p['pnl_diff']:+.0f}  P(diff<=0) {p['p_le_0']:.3f}")
