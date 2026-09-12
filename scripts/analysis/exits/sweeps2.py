"""finer losing-limit x winning-limit grid, robustness (leave-one-year-out), and combos with stop/threshold. writes tables_sweeps2.md"""
import pickle
from exitsim import *
d = pickle.load(open(f'{OUT}/cache.pkl', 'rb')); trades, paths = d['trades'], d['paths']
out = []
def run(**kw):
    p = dict(CUR); p.update(kw); return run_scenario(trades, paths, p)
def lw(L, W, **kw):  # losing limit L, winning limit W via max_hold=W-? : use max_hold=90, loss_red=90-L, prof_ext=W-90 (prof_ext may be negative -> use max_hold=L+... )
    mh = max(L, min(W, 90)); return run(max_hold=mh, loss_red=mh - L, prof_ext=W - mh, **kw)
Ls = [15, 20, 25, 30, 35, 40, 45, 50, 60, 75]
Ws = [60, 75, 90, 105, 120, 150]
out.append('## T1. surface: five-year total P&L ($) by losing limit L (rows) x winning limit W (cols); cell = total / #years>=0 / min-year')
out.append('| L \\ W | ' + ' | '.join(map(str, Ws)) + ' |'); out.append('|---|' + '---|' * len(Ws))
grid = {}
for L in Ls:
    cells = []
    for W in Ws:
        m = lw(L, W); grid[(L, W)] = m
        pos = sum(1 for y in YEARS if m[y]['pnl'] >= 0); mn = min(m[y]['pnl'] for y in YEARS)
        cells.append(f"{m['total']:+.0f} /{pos}/ {mn:+.0f}")
    out.append(f'| **{L}** | ' + ' | '.join(cells) + ' |')
out.append('\n## T2. same grid, per-year P&L for W=90 and W=120 (current W=120, L=75)')
out.append(SCN_HDR)
for W in (90, 120):
    for L in Ls: out.append(fmt_scn(f'L={L} W={W}', grid[(L, W)]))
# leave-one-year-out: choose best (L,W) on 4 years, report held-out year vs current
out.append('\n## T3. leave-one-year-out: (L,W) chosen on the other four years, applied to the held-out year')
out.append('| held-out year | chosen (L,W) | held-out P&L with choice | held-out P&L current | delta |'); out.append('|---|---|---|---|---|')
for y in YEARS:
    best = max(grid.items(), key=lambda kv: sum(kv[1][z]['pnl'] for z in YEARS if z != y))
    (L, W), m = best
    out.append(f"| {y} | ({L},{W}) | {m[y]['pnl']:+.0f} | {grid[(75,120)][y]['pnl']:+.0f} | {m[y]['pnl']-grid[(75,120)][y]['pnl']:+.0f} |")
# combos
out.append('\n## T4. combos on top of L=30/W=90 (max_hold 90, loss_red 60, prof_ext 0) and L=30/W=120')
out.append(SCN_HDR)
for name, kw in [('L30 W90', {}), ('L30 W90 + stop 1.5%', dict(stop=0.015)), ('L30 W90 + stop 1.0%', dict(stop=0.01)), ('L30 W90 + stop 2.0%', dict(stop=0.02)),
                 ('L30 W90 + thr 0.2', dict(thr=0.2)), ('L30 W90 + thr 0.1', dict(thr=0.1)), ('L30 W90 + thr never', dict(thr=None)),
                 ('L30 W90 + stop 1.5% + thr 0.2', dict(stop=0.015, thr=0.2)), ('L30 W90 + breakeven 0.5% (code)', dict(be=0.005))]:
    out.append(fmt_scn(name, lw(30, 90, **kw)))
for name, kw in [('L30 W120', {}), ('L30 W120 + stop 1.5%', dict(stop=0.015)), ('L45 W120 + stop 1.5%', dict(stop=0.015)), ('L45 W90 + stop 1.5%', dict(stop=0.015))]:
    L = 45 if 'L45' in name else 30; W = 120 if 'W120' in name else 90
    out.append(fmt_scn(name, lw(L, W, **kw)))
# stop finer
out.append('\n## T5. stop_loss_pct finer (current L/W)')
out.append(SCN_HDR)
for s in (0.008, 0.01, 0.0125, 0.015, 0.0175, 0.02, 0.025): out.append(fmt_scn(f'stop {s*100:.2f}%', run(stop=s)))
out.append('\n## T6. leave-one-year-out for stop_loss_pct alone')
out.append('| held-out year | chosen stop | held-out P&L with choice | held-out P&L current | delta |'); out.append('|---|---|---|---|---|')
stops = {s: run(stop=s) for s in (0.008, 0.01, 0.0125, 0.015, 0.0175, 0.02, 0.025, 0.03, 0.035)}
for y in YEARS:
    s, m = max(stops.items(), key=lambda kv: sum(kv[1][z]['pnl'] for z in YEARS if z != y))
    out.append(f"| {y} | {s*100:.2f}% | {m[y]['pnl']:+.0f} | {stops[0.025][y]['pnl']:+.0f} | {m[y]['pnl']-stops[0.025][y]['pnl']:+.0f} |")
# per exit-reason mix and per-bucket P&L under L30W90 vs current
out.append('\n## T7. what changes under L=30/W=90: exit-reason mix and P&L by original outcome bucket')
cur = run(); alt = lw(30, 90)
from collections import Counter, defaultdict
for nm, m in (('current', cur), ('L30 W90', alt)):
    mix = Counter(m['sims'][i]['reason'] for i in m['kept'])
    byb = defaultdict(float); n = defaultdict(int)
    for i in m['kept']:
        b = bucket(trades[i]['pnl_pct']); byb[b] += m['sims'][i]['pnl']; n[b] += 1
    out.append(f"- {nm}: mix {dict(mix)}; P&L by ORIGINAL bucket: " + ', '.join(f"{b} n={n[b]} ${byb[b]:+.0f}" for b in ('loss', 'breakeven', 'win')))
# how many original winners are cut before their exit by L=30 and what they lose
lost_w = [(trades[i], alt['sims'][i]) for i in alt['kept'] if trades[i]['pnl_pct'] > 0.003 and alt['sims'][i]['ts'] < trades[i]['exit_ts']]
out.append(f"- original winners exited earlier under L30/W90: {len(lost_w)}; their original $ {sum(t['pnl'] for t,_ in lost_w):+.0f} -> simulated $ {sum(s['pnl'] for _,s in lost_w):+.0f}")
lost_l = [(trades[i], alt['sims'][i]) for i in alt['kept'] if trades[i]['pnl_pct'] < -0.003 and alt['sims'][i]['ts'] < trades[i]['exit_ts']]
out.append(f"- original losers exited earlier under L30/W90: {len(lost_l)}; their original $ {sum(t['pnl'] for t,_ in lost_l):+.0f} -> simulated $ {sum(s['pnl'] for _,s in lost_l):+.0f}")
with open(f'{OUT}/tables_sweeps2.md', 'w') as f: f.write('\n'.join(out) + '\n')
print('\n'.join(out))
