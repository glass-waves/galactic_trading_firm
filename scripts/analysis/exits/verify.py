import pickle, csv, sys
from collections import Counter, defaultdict
from exitsim import *
d = pickle.load(open(f'{OUT}/cache.pkl', 'rb')); trades, paths, state = d['trades'], d['paths'], d['state']
mism = []; reason_ok = 0; ts_ok = 0; price_ok = 0; real = defaultdict(float); sim = defaultdict(float)
conf = Counter()
for i, t in enumerate(trades):
    s = simulate(t, paths[i], CUR)
    conf[(t['reason'], s['reason'])] += 1
    real[t['year']] += t['pnl']; sim[t['year']] += s['pnl']
    dp = abs(s['fill'] - t['exit_price']) / t['exit_price']
    if s['reason'] == t['reason']: reason_ok += 1
    if s['ts'] == t['exit_ts']: ts_ok += 1
    if dp <= 0.002: price_ok += 1
    if s['ts'] != t['exit_ts'] or dp > 0.002 or s['reason'] != t['reason']:
        mism.append(dict(year=t['year'], date=t['date'], ticker=t['ticker'], real_reason=t['reason'], sim_reason=s['reason'],
                         real_hold=t['hold_min'], sim_hold=s['hold'], real_exit=t['exit_price'], sim_exit=round(s['fill'],4),
                         real_pnl=t['pnl'], sim_pnl=round(s['pnl'],2), dprice=round(dp,5)))
print('n', len(trades), 'reason ok', reason_ok, 'ts ok', ts_ok, 'price<=0.2% ok', price_ok)
print('confusion (real,sim):', sorted(conf.items(), key=lambda x: -x[1]))
for y in YEARS: print(y, 'real', round(real[y]), 'sim', round(sim[y]), 'diff', round(sim[y]-real[y]))
with open(f'{OUT}/repro_mismatches.csv', 'w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(mism[0].keys()) if mism else ['none']); w.writeheader(); w.writerows(mism)
print('mismatches', len(mism))
for m in mism[:25]: print(m)
# cross-check path vs engine state: ticks row at bar j+1 shows unrealized from close of bar j
bad = 0; checked = 0; worst = 0
for i, t in enumerate(trades):
    p = paths[i]
    for j in range(len(p) - 1):
        if p[j]['ts'] + 60 >= t['exit_ts']: break
        st = state.get((t['ticker'], p[j]['ts'] + 60))
        if not st or st[0] != 'short' or not st[1]: continue
        checked += 1
        diff = abs(float(st[1]) - p[j]['unreal'])
        worst = max(worst, diff)
        if diff > 0.0002: bad += 1
        if int(st[2]) != p[j]['hold'] + 1: bad += 1
print('state cross-check rows', checked, 'bad', bad, 'worst unreal diff', worst)
