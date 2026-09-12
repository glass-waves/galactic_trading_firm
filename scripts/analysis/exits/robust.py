"""robustness of L=30/W=90 (and L=30/W=120) vs current by ticker / entry window / year; what the rule does to cut trades. writes tables_robust.md"""
import pickle
from collections import defaultdict
from exitsim import *
d = pickle.load(open(f'{OUT}/cache.pkl', 'rb')); trades, paths = d['trades'], d['paths']
def run(**kw):
    p = dict(CUR); p.update(kw); return run_scenario(trades, paths, p)
cur = run(); a90 = run(max_hold=90, loss_red=60, prof_ext=0); a120 = run(max_hold=90, loss_red=60, prof_ext=30)
b15 = run(stop=0.015)
out = []
def table(title, keyf, keys):
    out.append(f'\n## {title}\n')
    out.append('| year | ' + ' | '.join(f'{k} cur / L30W90 / L30W120 / stop1.5' for k in keys) + ' |'); out.append('|---|' + '---|' * len(keys))
    for y in YEARS + ['ALL']:
        cells = []
        for k in keys:
            vals = []
            for m in (cur, a90, a120, b15):
                s = sum(m['sims'][i]['pnl'] for i in m['kept'] if (y == 'ALL' or trades[i]['year'] == y) and keyf(trades[i]) == k)
                vals.append(f'{s:+.0f}')
            cells.append(' / '.join(vals))
        out.append(f'| {y} | ' + ' | '.join(cells) + ' |')
table('R1. P&L by ticker: current / L30 W90 / L30 W120 / stop 1.5% (current L/W)', lambda t: t['ticker'], TICKERS)
wins = sorted(set(t['entry_reason'] for t in trades))
table('R2. P&L by entry window', lambda t: t['entry_reason'], wins)
# what happens to trades cut by the 30-min rule
out.append('\n## R3. trades the L=30 rule cuts earlier than the current exit (per year): count, their P&L under current rules vs under L30/W90, split by whether the trade was an original loser/BE/winner\n')
out.append('| year | cut trades | of which orig loss / BE / win | $ current | $ L30W90 | delta | orig-loser delta | orig-winner delta | cut at exactly 30-31 min |'); out.append('|---|---|---|---|---|---|---|---|---|')
for y in YEARS + ['ALL']:
    idx = [i for i in a90['kept'] if (y == 'ALL' or trades[i]['year'] == y) and a90['sims'][i]['ts'] < trades[i]['exit_ts']]
    bk = defaultdict(list)
    for i in idx: bk[bucket(trades[i]['pnl_pct'])].append(i)
    dc = sum(trades[i]['pnl'] for i in idx); da = sum(a90['sims'][i]['pnl'] for i in idx)
    dl = sum(a90['sims'][i]['pnl'] - trades[i]['pnl'] for i in bk['loss']); dw = sum(a90['sims'][i]['pnl'] - trades[i]['pnl'] for i in bk['win'])
    early = sum(1 for i in idx if a90['sims'][i]['hold'] <= 31)
    out.append(f"| {y} | {len(idx)} | {len(bk['loss'])} / {len(bk['breakeven'])} / {len(bk['win'])} | {dc:+.0f} | {da:+.0f} | {da-dc:+.0f} | {dl:+.0f} | {dw:+.0f} | {early} |")
# distribution of sim exit hold under L30W90
out.append('\n## R4. hold-time distribution under L30/W90 vs current (share of trades)\n')
def hist(m):
    c = defaultdict(int)
    for i in m['kept']:
        h = m['sims'][i]['hold']; b = '<=31' if h <= 31 else ('32-60' if h <= 60 else ('61-91' if h <= 91 else '>91')); c[b] += 1
    n = len(m['kept']); return {k: f'{v/n:.0%}' for k, v in c.items()}
out.append(f"- current: {dict(hist(cur))}"); out.append(f"- L30 W90: {dict(hist(a90))}")
# tolerance variant (information only, not a config knob): exit at >=30 min only if unreal < -0.1% / -0.2%
out.append('\n## R5. (information only, NOT a config knob) losing-limit 30 with a tolerance: exit at hold>=30 only if unrealized < -x\n')
out.append(SCN_HDR)
def sim_tol(x):
    rows = []
    for i, t in enumerate(trades):
        ep = t['entry_price']; res = None
        for b in paths[i]:
            c = b['c']; unreal = b['unreal']; r = None
            if b['etm'] >= CLOSE_MIN: r = 'SessionClose'
            elif b['comp'] is not None and b['comp'] >= 0.30: r = 'ScoreExit'
            elif (c - ep) / ep >= 0.025: r = 'HardStop'
            elif b['hold'] >= 30 and unreal < -x: r = 'MaxHoldTimeout'
            elif b['hold'] >= 90 and unreal <= 0: r = 'MaxHoldTimeout'
            elif b['hold'] >= 90 and unreal > 0: r = 'MaxHoldTimeout'
            if r: res = dict(reason=r, ts=b['ts'] + 60, pnl=b['fill_pnl'] * ep * t['size']); break
        if res is None: b = paths[i][-1]; res = dict(reason='End', ts=b['ts'], pnl=b['unreal'] * ep * t['size'])
        rows.append(res)
    kept = apply_overlap(trades, rows); m = metrics([(trades[i]['year'], rows[i]['ts'], rows[i]['pnl']) for i in kept]); m['dropped'] = len(trades) - len(kept); m['total'] = sum(m[y]['pnl'] for y in YEARS); return m
for x in (0.0, 0.001, 0.002, 0.003): out.append(fmt_scn(f'L30 W90 tol {x*100:.1f}%', sim_tol(x)))
with open(f'{OUT}/tables_robust.md', 'w') as f: f.write('\n'.join(out) + '\n')
print('\n'.join(out))
