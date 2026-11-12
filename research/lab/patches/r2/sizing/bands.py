import csv,collections,sys
f=sys.argv[1]; cap0=float(sys.argv[2]) if len(sys.argv)>2 else 100000
rows=[r for r in csv.DictReader(open(f)) if r['row_type']=='trade']
rows.sort(key=lambda r:r['exit_time'])
# reconstruct available capital per ticker at entry: cap0 + realized pnl of earlier-closed trades of the ticker
cum=collections.defaultdict(float); closed=collections.defaultdict(list)
for r in rows: closed[r['ticker']].append((r['exit_time'],float(r['pnl'])))
MULTS=[0.7,0.8,0.9,1.0,1.25,1.5,1.75]; NAMES=['<.20','.20-.22','.22-.24','.24-.26','.26-.30','.30-.35','>=.35']
out=[]
for r in sorted(rows,key=lambda r:r['entry_time']):
  avail=cap0+sum(p for t,p in closed[r['ticker']] if t<=r['entry_time'])
  frac=float(r['size'])*float(r['entry_price'])/avail; m=frac/0.36
  b=min(range(len(MULTS)),key=lambda i:abs(MULTS[i]-m)) if m<1.74 else 6
  # distinguish floor: true mult >= frac (floor reduces). choose smallest mult >= m - tolerance
  cands=[i for i in range(len(MULTS)) if MULTS[i]*0.36>=frac-1e-9]
  b=cands[0] if cands else 6
  r['band']=b; r['mult']=MULTS[b]; r['pct']=float(r['pnl_pct']); r['unit']=float(r['pnl'])/MULTS[b]
  out.append(r)
def pf(x):
  gw=sum(v for v in x if v>0); gl=-sum(v for v in x if v<0); return round(gw/gl,2) if gl else 99
print('band n  unit-pnl(per 1.0x, $100k)  PF  mean%  per-year n')
for b in range(7):
  s=[r for r in out if r['band']==b]
  if not s: continue
  u=[r['unit'] for r in s]
  yrs=collections.Counter(r['date'][:4] for r in s)
  print(NAMES[b],len(s),round(sum(u)),pf(u),round(100*sum(r['pct'] for r in s)/len(s),3),dict(sorted(yrs.items())))
import json; json.dump([{k:r[k] for k in ('date','ticker','entry_time','entry_reason','exit_reason','band','unit','pct')} for r in out],open(sys.argv[3],'w'))
