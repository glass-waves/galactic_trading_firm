"""screen 4: pairs of the top conditions on the v16 windows (and both-firing standalone for require-type
pairs), plus threshold sensitivity for the two strongest single conditions. writes s4_pairs.csv, s4_sens.csv."""
import csv, os, json, itertools, statistics
import screen as S
SCREEN = S.SCREEN; YEARS = S.YEARS

# (label, feature name in screen.features(), mode)
TOP = [('SPYflat', 'SPY session (-0.2,0.2)%', 'req'),
       ('noPDLext', 'pdl dist_low < -1%', 'excl'),
       ('noGapDn1', 'gap <= -1%', 'excl'),
       ('noRvolQ5', 'rvol_raw Q5 (>=1.13)', 'excl'),
       ('noPDL0_05', 'pdl dist_low [0,0.5)%', 'excl'),
       ('noOR15low', 'or15 lower half of range', 'excl'),
       ('vpinQ5', 'vpin_raw Q5 (>=0.217)', 'req')]

def sens_conditions(cols, n):
    """threshold sweeps on the two strongest conditions."""
    X = cols['x_sess']; P = cols['pdl_dlow']; G = cols['gap_pct']
    out = {}
    for b in (0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5):
        out[f'SPY |session| < {b}%'] = [x is not None and abs(x) < b for x in X]
    for t in (-0.5, -1.0, -1.5, -2.0):
        out[f'exclude pdl dist_low < {t}%'] = [not (x is not None and x < t) for x in P]
    for t in (-0.5, -1.0, -1.5, -2.0):
        out[f'exclude gap <= {t}%'] = [not (x is not None and x <= t) for x in G]
    return out

rows4 = []; rows_s = []
for year in YEARS:
    print(f'== {year}', flush=True)
    cols, n, _ = S.load_year(year)
    keys = list(zip(cols['ticker'], cols['date'])); opp = cols['opp_pnl_pct']
    top_thr = sorted(opp)[int(0.95 * n)]; base_mean = statistics.mean(opp)
    win, _, _ = S.win_flags(cols, n)
    F = S.features(cols, n)
    cond = {}
    for lab, name, mode in TOP:
        m = F[name][1]
        cond[lab] = (m if mode == 'req' else S.NOT(m), mode)
    def rec(label, kind, mask):
        st = S.stats(mask, opp, keys, top_thr, base_mean)
        sm = S.sim_summary(S.simulate(mask, cols, n, S.EXIT_V16))
        return dict(label=label, kind=kind, year=year, bars=st['bars'], eps=st['eps'], bar_mean=st['mean'], ep_mean=st['ep_mean'], hit=st['hit'], **sm)
    rows4.append(rec('v16', 'base', win))
    for lab, (m, mode) in cond.items():
        rows4.append(rec(lab, 'single-on-window', S.AND(win, m)))
    for (la, (ma, moa)), (lb, (mb, mob)) in itertools.combinations(cond.items(), 2):
        rows4.append(rec(f'{la} + {lb}', 'pair-on-window', S.AND(win, S.AND(ma, mb))))
        if moa == 'req' and mob == 'req':
            rows4.append(rec(f'{la} + {lb}', 'pair-standalone', S.AND(ma, mb)))
    for lab, m in sens_conditions(cols, n).items():
        rows_s.append(rec(lab, 'sens-on-window', S.AND(win, m)))
    del F, cols

for name, rows in (('s4_pairs.csv', rows4), ('s4_sens.csv', rows_s)):
    with open(os.path.join(SCREEN, name), 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)

# ---- summary
def by(rows):
    d = {}
    for r in rows: d.setdefault((r['label'], r['kind']), {})[r['year']] = r
    return d
def line(lab, yrs):
    tot = sum(yrs[y]['pnl'] for y in YEARS); tr = sum(yrs[y]['trades'] for y in YEARS)
    gw = gl = 0.0
    for y in YEARS:
        p, pf = yrs[y]['pnl'], yrs[y]['pf']
        if pf is None or pf == 1 or yrs[y]['trades'] == 0: continue
        if pf == float('inf'): gw += p; continue
        g = p / (pf - 1); gl += g; gw += g * pf
    py = ' '.join(f"{100*(yrs[y]['mean_pct'] or 0):+.2f}({yrs[y]['pnl']:+,.0f}/{yrs[y]['trades']})" for y in YEARS)
    return f"{lab:34s} 5y {tot:+7,.0f} tr {tr:5d} PF {gw/gl if gl else 0:.2f} | {py}"
D = by(rows4)
print('\n=== pairs on window (beats-both = years per-trade mean > both parents)')
print(line('v16', D[('v16', 'base')]))
for lab, _, _ in TOP: print(line(lab, D[(lab, 'single-on-window')]))
print()
for (lab, kind), yrs in D.items():
    if kind != 'pair-on-window': continue
    a, b = lab.split(' + ')
    pa, pb = D[(a, 'single-on-window')], D[(b, 'single-on-window')]
    beats = sum(1 for y in YEARS if (yrs[y]['mean_pct'] or -9) > (pa[y]['mean_pct'] or -9) and (yrs[y]['mean_pct'] or -9) > (pb[y]['mean_pct'] or -9))
    pos = sum(1 for y in YEARS if yrs[y]['pnl'] > 0)
    print(f"beats-both {beats}/5 pos {pos}/5 " + line(lab, yrs))
print('\n=== standalone both-firing (require-type pairs)')
for (lab, kind), yrs in D.items():
    if kind == 'pair-standalone': print(line(lab, yrs))
print('\n=== sensitivity (on window)')
for (lab, kind), yrs in by(rows_s).items():
    print(line(lab, yrs))
