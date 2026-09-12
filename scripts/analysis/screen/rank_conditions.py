"""rank every (feature, require|exclude) condition on the v16 windows from the calibrated simulation
(s3_sim.csv): trade retention vs v16, years the per-trade mean beats v16, years P&L beats v16."""
import csv, os, json
SCREEN = os.path.dirname(os.path.abspath(__file__))
YEARS = [2022, 2023, 2024, 2025, 2026]
s3 = {}
for r in csv.DictReader(open(os.path.join(SCREEN, 's3_sim.csv'))):
    s3.setdefault((r['trigger'], r['mode']), {})[int(r['year'])] = r
v16 = s3[('v16 window (any)', 'standalone')]
v16_tr = sum(int(v16[y]['trades']) for y in YEARS)
out = []
for (name, mode), yrs in s3.items():
    if mode == 'standalone': continue
    tr = sum(int(yrs[y]['trades']) for y in YEARS)
    keep = tr / v16_tr
    mean_up = sum(1 for y in YEARS if yrs[y]['mean_pct'] and float(yrs[y]['mean_pct']) > float(v16[y]['mean_pct']))
    pnl_up = sum(1 for y in YEARS if float(yrs[y]['pnl']) > float(v16[y]['pnl']))
    pf_up = sum(1 for y in YEARS if yrs[y]['pf'] and float(yrs[y]['pf']) > float(v16[y]['pf']))
    pos = sum(1 for y in YEARS if float(yrs[y]['pnl']) > 0)
    total = sum(float(yrs[y]['pnl']) for y in YEARS)
    gw = gl = 0.0
    for y in YEARS:
        p = float(yrs[y]['pnl']); pf = float(yrs[y]['pf']) if yrs[y]['pf'] else None
        if pf is None or pf == 1: continue
        if pf == float('inf'): gw += p; continue
        g = p / (pf - 1); gl += g; gw += g * pf
    out.append(dict(feature=name, mode=mode, keep=keep, trades=tr, mean_up=mean_up, pf_up=pf_up, pnl_up=pnl_up, pos=pos, total=total,
                    pf=(gw / gl if gl else None), per_year=[(float(yrs[y]['pnl']), int(yrs[y]['trades']), float(yrs[y]['pf']) if yrs[y]['pf'] else None,
                                                             float(yrs[y]['mean_pct']) if yrs[y]['mean_pct'] else None) for y in YEARS]))
out.sort(key=lambda r: (-(r['keep'] >= 0.4), -r['mean_up'], -(r['pf'] or 0)))
print(f"v16: trades {v16_tr}, per-year mean pct " + ' '.join(f"{100*float(v16[y]['mean_pct']):+.3f}" for y in YEARS))
print(f"{'feature':44s} {'mode':11s} keep  trades meanUp pfUp pnlUp pos  5yPnL  PF   | per-year mean% (pnl)")
for r in out:
    if r['keep'] < 0.4: continue
    py = ' '.join(f"{100*m:+.2f}({p:+,.0f})" if m is not None else '-' for p, t, f, m in r['per_year'])
    print(f"{r['feature'][:44]:44s} {r['mode']:11s} {r['keep']:.2f} {r['trades']:6d} {r['mean_up']:6d} {r['pf_up']:4d} {r['pnl_up']:5d} {r['pos']:3d} {r['total']:+7,.0f} {r['pf']:.2f} | {py}")
json.dump(out, open(os.path.join(SCREEN, 'rank_conditions.json'), 'w'), indent=1)
