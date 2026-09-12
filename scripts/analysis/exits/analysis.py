"""analyses 1 & 2: path reconstruction, MFE/MAE, horizons, regret/forgone. writes tables_paths.md + paths.csv"""
import pickle, csv, statistics as st
from collections import defaultdict
from exitsim import *
d = pickle.load(open(f'{OUT}/cache.pkl', 'rb')); trades, paths = d['trades'], d['paths']
H = [15, 30, 45, 60, 75, 90, 120]
REASONS = ['HardStop', 'ScoreExit', 'MaxHoldTimeout', 'SessionClose']
BK = ['loss', 'breakeven', 'win']

def pnl_at(p, n):
    best = None
    for b in p:
        if b['hold'] <= n: best = b
        else: break
    return best['fill_pnl'] if best else p[-1]['fill_pnl']

rows = []
for i, t in enumerate(trades):
    p = paths[i]
    if t['reason'] == 'TrailingStop': continue
    sig_hold = t['hold_min'] - 1              # exit signalled on this bar
    pre = [b for b in p if b['hold'] <= sig_hold]
    post = [b for b in p if b['hold'] >= sig_hold]  # from signal bar on (fill_pnl of signal bar == actual)
    mfe_b = max(p, key=lambda b: b['unreal']); mae_b = min(p, key=lambda b: b['unreal'])
    mfe_pre = max(b['unreal'] for b in pre) if pre else 0.0
    mae_pre = min(b['unreal'] for b in pre) if pre else 0.0
    best_after = max(b['fill_pnl'] for b in post) if post else t['pnl_pct']
    r = dict(year=t['year'], date=t['date'], ticker=t['ticker'], reason=t['reason'], bucket=bucket(t['pnl_pct']),
             actual=t['pnl_pct'], hold=t['hold_min'], mfe=mfe_b['unreal'], t_mfe=mfe_b['hold'], mae=mae_b['unreal'], t_mae=mae_b['hold'],
             mfe_pre=mfe_pre, mae_pre=mae_pre, p1155=p[-1]['fill_pnl'], best_after=best_after,
             regret=max(0.0, best_after - t['pnl_pct']), forgone=p[-1]['fill_pnl'] - t['pnl_pct'],
             entry_reason=t['entry_reason'], size=t['size'], entry_price=t['entry_price'])
    for n in H: r[f'p{n}'] = pnl_at(p, n)
    rows.append(r)
with open(f'{OUT}/paths.csv', 'w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0].keys())); w.writeheader(); w.writerows(rows)

def mean(xs): return st.mean(xs) if xs else float('nan')
def med(xs): return st.median(xs) if xs else float('nan')
def pc(x): return f'{x*100:+.2f}'
out = []
def sect(s): out.append('\n' + s + '\n')
def tbl(hdr, lines): out.append('| ' + ' | '.join(hdr) + ' |'); out.append('|' + '---|' * len(hdr)); out.extend(lines)

# ---- table A: bucket x reason x year summary (n, mean actual, mfe, mae, t_mfe, horizons, 11:55)
sect('## A. path reconstruction: per year x exit_reason x outcome bucket (all values mean % of entry price, +=profit; t_mfe median minutes)')
hdr = ['year', 'reason', 'bucket', 'n', 'actual', 'hold', 'MFE', 'MFE_pre', 't_mfe', 'MAE', 'MAE_pre'] + [f'@{n}' for n in H] + ['@11:55']
lines = []
for y in YEARS + ['ALL']:
    for rs in REASONS:
        for bk in BK:
            g = [r for r in rows if (y == 'ALL' or r['year'] == y) and r['reason'] == rs and r['bucket'] == bk]
            if not g: continue
            lines.append('| ' + ' | '.join([str(y), rs, bk, str(len(g)), pc(mean([r['actual'] for r in g])), f"{med([r['hold'] for r in g]):.0f}",
                pc(mean([r['mfe'] for r in g])), pc(mean([r['mfe_pre'] for r in g])), f"{med([r['t_mfe'] for r in g]):.0f}",
                pc(mean([r['mae'] for r in g])), pc(mean([r['mae_pre'] for r in g]))] +
                [pc(mean([r[f'p{n}'] for r in g])) for n in H] + [pc(mean([r['p1155'] for r in g]))]) + ' |')
tbl(hdr, lines)

# ---- table B: bucket x year (all reasons)
sect('## B. per year x bucket, all exit reasons (mean %)')
hdr = ['year', 'bucket', 'n', 'share', 'actual', 'MFE_pre', 'MAE_pre', '@15', '@30', '@45', '@60', '@90', '@11:55', 'MFE_pre>=0.5%', 'MFE_pre>=1.0%', 'neg@15', 'neg@30', 'neg@45']
lines = []
for y in YEARS + ['ALL']:
    ally = [r for r in rows if y == 'ALL' or r['year'] == y]
    for bk in BK:
        g = [r for r in ally if r['bucket'] == bk]
        if not g: continue
        lines.append('| ' + ' | '.join([str(y), bk, str(len(g)), f'{len(g)/len(ally):.0%}', pc(mean([r['actual'] for r in g])),
            pc(mean([r['mfe_pre'] for r in g])), pc(mean([r['mae_pre'] for r in g]))] +
            [pc(mean([r[f'p{n}'] for r in g])) for n in (15, 30, 45, 60, 90)] + [pc(mean([r['p1155'] for r in g])),
            f"{sum(1 for r in g if r['mfe_pre']>=0.005)/len(g):.0%}", f"{sum(1 for r in g if r['mfe_pre']>=0.01)/len(g):.0%}"] +
            [f"{sum(1 for r in g if r[f'p{n}']<0)/len(g):.0%}" for n in (15, 30, 45)]) + ' |')
tbl(hdr, lines)

# ---- table C: losers: share of eventual loss present at 15/30/45 (mean pnl@N / mean actual), and $-weighted
sect('## C. losers + breakevens: how much of the final result was already there at 15/30/45 min (per year x reason). ratio = mean(@N)/mean(actual); "$@N" = dollar P&L if every trade in the group had exited at N (size-weighted)')
hdr = ['year', 'reason', 'n', 'actual $', '$@15', '$@30', '$@45', '$@60', '$@11:55', 'ratio@15', 'ratio@30', 'ratio@45', 'gave back >=0.5%', '>=1.0%']
lines = []
for y in YEARS + ['ALL']:
    for rs in REASONS + ['ALL']:
        g = [r for r in rows if (y == 'ALL' or r['year'] == y) and (rs == 'ALL' or r['reason'] == rs) and r['bucket'] in ('loss', 'breakeven')]
        if not g: continue
        dol = lambda k: sum(r[k] * r['entry_price'] * r['size'] for r in g)
        a = mean([r['actual'] for r in g])
        lines.append('| ' + ' | '.join([str(y), rs, str(len(g)), f"{dol('actual'):+.0f}", f"{dol('p15'):+.0f}", f"{dol('p30'):+.0f}", f"{dol('p45'):+.0f}", f"{dol('p60'):+.0f}", f"{dol('p1155'):+.0f}",
            f"{mean([r['p15'] for r in g])/a:.2f}", f"{mean([r['p30'] for r in g])/a:.2f}", f"{mean([r['p45'] for r in g])/a:.2f}",
            f"{sum(1 for r in g if r['mfe_pre']>=0.005)/len(g):.0%}", f"{sum(1 for r in g if r['mfe_pre']>=0.01)/len(g):.0%}"]) + ' |')
tbl(hdr, lines)

# ---- table D: regret & forgone per year x reason x bucket
sect('## D. regret (best fill available after the exit signal bar up to 11:55, minus actual; floored at 0) and forgone (11:55 fill minus actual, signed). mean / median %, plus dollar sums (size-weighted)')
hdr = ['year', 'reason', 'bucket', 'n', 'actual', 'regret mean', 'regret med', 'regret>0.3%', 'forgone mean', 'forgone med', 'forgone $', 'capture=actual/MFE']
lines = []
for y in YEARS + ['ALL']:
    for rs in REASONS:
        for bk in BK + ['ALL']:
            g = [r for r in rows if (y == 'ALL' or r['year'] == y) and r['reason'] == rs and (bk == 'ALL' or r['bucket'] == bk)]
            if not g: continue
            mfe = mean([r['mfe'] for r in g])
            lines.append('| ' + ' | '.join([str(y), rs, bk, str(len(g)), pc(mean([r['actual'] for r in g])), pc(mean([r['regret'] for r in g])), pc(med([r['regret'] for r in g])),
                f"{sum(1 for r in g if r['regret']>0.003)/len(g):.0%}", pc(mean([r['forgone'] for r in g])), pc(med([r['forgone'] for r in g])),
                f"{sum(r['forgone']*r['entry_price']*r['size'] for r in g):+.0f}", f"{mean([r['actual'] for r in g])/mfe:.2f}" if mfe > 0 else '-']) + ' |')
tbl(hdr, lines)

# ---- table E: winners: where was the exit relative to the path best
sect('## E. winners: exit timing vs path (share exiting within 0.2% of the path-best fill; share where price kept falling to 11:55)')
hdr = ['year', 'reason', 'n', 'actual', 'MFE', 'capture', 'near-best (<=0.2% below best)', 'better at 11:55', 'worse at 11:55 by >0.3%', 'forgone $']
lines = []
for y in YEARS + ['ALL']:
    for rs in REASONS:
        g = [r for r in rows if (y == 'ALL' or r['year'] == y) and r['reason'] == rs and r['bucket'] == 'win']
        if not g: continue
        lines.append('| ' + ' | '.join([str(y), rs, str(len(g)), pc(mean([r['actual'] for r in g])), pc(mean([r['mfe'] for r in g])),
            f"{mean([r['actual'] for r in g])/mean([r['mfe'] for r in g]):.2f}",
            f"{sum(1 for r in g if r['mfe']-r['actual']<=0.002)/len(g):.0%}", f"{sum(1 for r in g if r['forgone']>0)/len(g):.0%}",
            f"{sum(1 for r in g if r['forgone']<-0.003)/len(g):.0%}", f"{sum(r['forgone']*r['entry_price']*r['size'] for r in g):+.0f}"]) + ' |')
tbl(hdr, lines)

# ---- table F: MaxHold losers by hold length bucket (75 vs 120 path), and P&L trajectory
sect('## F. MaxHoldTimeout exits by fired limit (75 = losing at check, 121 = winning at check, other = flipped after 75): per year')
hdr = ['year', 'limit', 'n', 'actual $', 'mean actual', 'mean @45', 'mean @60', 'mean @11:55', 'forgone $', 'regret mean', 'win%']
lines = []
for y in YEARS + ['ALL']:
    for lim in ('76', '121', 'other'):
        g = [r for r in rows if (y == 'ALL' or r['year'] == y) and r['reason'] == 'MaxHoldTimeout' and
             ((lim == '76' and r['hold'] == 76) or (lim == '121' and r['hold'] == 121) or (lim == 'other' and r['hold'] not in (76, 121)))]
        if not g: continue
        lines.append('| ' + ' | '.join([str(y), lim, str(len(g)), f"{sum(r['actual']*r['entry_price']*r['size'] for r in g):+.0f}", pc(mean([r['actual'] for r in g])),
            pc(mean([r['p45'] for r in g])), pc(mean([r['p60'] for r in g])), pc(mean([r['p1155'] for r in g])),
            f"{sum(r['forgone']*r['entry_price']*r['size'] for r in g):+.0f}", pc(mean([r['regret'] for r in g])), f"{sum(1 for r in g if r['actual']>0)/len(g):.0%}"]) + ' |')
tbl(hdr, lines)

# ---- table G: profitable-at-N conditional outcome (time-stop evidence): trades not profitable at N: what did they end at?
sect('## G. time-stop evidence: trades NOT profitable (fill_pnl<=0) at N minutes vs profitable: eventual actual $ and mean %, per year')
hdr = ['year', 'N', 'n not-prof@N', 'their actual $', 'their $ if exited @N', 'delta $', 'mean actual', 'mean @N', 'n prof@N', 'prof actual $']
lines = []
for y in YEARS + ['ALL']:
    for n in (20, 30, 45):
        g = [r for r in rows if (y == 'ALL' or r['year'] == y) and r['hold'] > n]
        np_ = [r for r in g if r[f'p{n}' if n != 20 else 'p20'] <= 0] if n != 20 else None
        # p20 not precomputed: compute on the fly
        if n == 20:
            pass
        lines.append(None)
tbl(hdr, [])
out.pop(); out.pop(); out.pop()  # remove empty table G header; computed in sweeps instead
with open(f'{OUT}/tables_paths.md', 'w') as f: f.write('\n'.join(out) + '\n')
print('\n'.join(out))
