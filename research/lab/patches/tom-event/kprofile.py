# open(09:31)->15:56 open net return (3 bps/leg + $0.005/sh/leg ignored as price-relative ~1bp) per session k of month,
# train window only (<= 2025-06-30). bars only, no engine: a calendar profile to read k=2 against its neighbours.
import csv, datetime as dt, zoneinfo, sys, random
from collections import defaultdict
ET = zoneinfo.ZoneInfo("America/New_York")
sys.path.insert(0, __import__('os').path.dirname(__file__))
from subsets import SUBS
END = dt.date(2025, 6, 30)
def oc(t):
    o = {}; c = {}
    for line in open(f"/home/dylmet/Projects/galactic_trading_firm/data/bars_iex/{t}.csv").readlines()[1:]:
        ts, op = line.split(",")[:2]
        x = dt.datetime.fromtimestamp(int(ts), ET)
        d = x.date()
        if d > END: break
        hm = x.hour * 60 + x.minute
        if hm >= 571 and d not in o: o[d] = float(op)
        if hm >= 956 and d not in c: c[d] = float(op)
    return {d: 1e4 * (c[d] / o[d] - 1) - 6 - 1e4 * 0.01 / o[d] for d in o if d in c}
R = {t: oc(t) for t in SUBS["ALL41"]}
days = sorted(R["SPY"])
by = defaultdict(list)
for d in days: by[(d.year, d.month)].append(d)
kmap = {}
for v in by.values():
    for i, d in enumerate(v):
        kmap[d] = (i + 1, i - len(v))
def basket(tk, d):
    xs = [R[t][d] for t in tk if d in R[t]]
    return sum(xs) / len(xs) if xs else None
for name in ["SPY", "QQQ", "IDX3", "MEGA4", "POOL34", "ALL41"]:
    tk = SUBS[name]
    allm = [basket(tk, d) for d in days]; allm = [x for x in allm if x is not None]
    line = f"{name:<6} all {sum(allm)/len(allm):+5.1f} | "
    for k in [1, 2, 3, 4, 5, 6, 8, 10, 12, 15]:
        xs = [basket(tk, d) for d in days if kmap[d][0] == k]
        line += f"k{k} {sum(xs)/len(xs):+5.1f} "
    for k in [-1, -2, -3]:
        xs = [basket(tk, d) for d in days if kmap[d][1] == k]
        line += f"k{k} {sum(xs)/len(xs):+5.1f} "
    print(line)
# random-month-session control for k=2 on ALL41: distribution of mean over 42 random sessions (one per month)
tk = SUBS["ALL41"]; rng = random.Random(0)
k2 = [basket(tk, d) for d in days if kmap[d][0] == 2]; obs = sum(k2) / len(k2)
sims = []
for _ in range(5000):
    xs = [basket(tk, rng.choice(v)) for v in by.values()]
    sims.append(sum(xs) / len(xs))
print(f"ALL41 k=2 {obs:+.1f} bps; random one-session-per-month p = {sum(s >= obs for s in sims)/len(sims):.3f}; sim mean {sum(sims)/len(sims):+.1f}")
for name in ["SPY","QQQ","MEGA4"]:
    tk=SUBS[name]; k2=[basket(tk,d) for d in days if kmap[d][0]==2]; obs=sum(k2)/len(k2)
    sims=[]
    for _ in range(5000):
        xs=[basket(tk,rng.choice(v)) for v in by.values()]; sims.append(sum(xs)/len(xs))
    print(f"{name} k=2 {obs:+.1f}; random p {sum(s>=obs for s in sims)/len(sims):.3f}")
import statistics as st
print("--- z-profile ALL41 (mean bps, z vs all-session mean, n)")
tk = SUBS["ALL41"]; allm = [basket(tk, d) for d in days]; mu = sum(allm) / len(allm)
zs = []
for k in list(range(1, 22)) + [-1, -2, -3, -4, -5]:
    xs = [basket(tk, d) for d in days if (kmap[d][0] if k > 0 else kmap[d][1]) == k]
    if len(xs) < 30: continue
    z = (sum(xs) / len(xs) - mu) / (st.pstdev(allm) / len(xs) ** 0.5)
    zs.append((k, z)); print(f"k{k:+d} {sum(xs)/len(xs):+6.1f} z {z:+.2f} n {len(xs)}")
print("sd of z across k (1.0 = pure noise):", round(st.pstdev([z for _, z in zs]), 2), "count |z|>=1.8:", sum(abs(z) >= 1.8 for _, z in zs), "of", len(zs))
