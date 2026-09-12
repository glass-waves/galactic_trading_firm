"""extra checks after the pair test: gap band union, wider SPY band in the best pair, and the triple."""
import screen as S, statistics, csv, os
YEARS = S.YEARS; rows = []
for year in YEARS:
    cols, n, _ = S.load_year(year)
    win, _, _ = S.win_flags(cols, n); F = S.features(cols, n)
    G, X, V, P = cols['gap_pct'], cols['x_sess'], cols['vpin_raw'], cols['pdl_dlow']
    C = {
      'gap in (-1,+0.3)% require': [g is not None and -1 < g < 0.3 for g in G],
      'SPY|sess|<0.2 + vpinQ5': S.AND([x is not None and abs(x) < 0.2 for x in X], F['vpin_raw Q5 (>=0.217)'][1]),
      'SPY|sess|<0.3 + vpinQ5': S.AND([x is not None and abs(x) < 0.3 for x in X], F['vpin_raw Q5 (>=0.217)'][1]),
      'SPY|sess|<0.2 + vpin_raw>=0.18': S.AND([x is not None and abs(x) < 0.2 for x in X], [v is not None and v >= 0.18 for v in V]),
      'SPY|sess|<0.2 + vpin_raw>=0.26': S.AND([x is not None and abs(x) < 0.2 for x in X], [v is not None and v >= 0.26 for v in V]),
      'SPY|sess|<0.2 + vpinQ5 + noPDLext (triple)': S.AND(S.AND([x is not None and abs(x) < 0.2 for x in X], F['vpin_raw Q5 (>=0.217)'][1]), [not (p is not None and p < -1) for p in P]),
      'SPY|sess|<0.2 + gap in (-1,+0.3)': S.AND([x is not None and abs(x) < 0.2 for x in X], [g is not None and -1 < g < 0.3 for g in G]),
    }
    for lab, m in C.items():
        sm = S.sim_summary(S.simulate(S.AND(win, m), cols, n, S.EXIT_V16))
        rows.append(dict(label=lab, year=year, **sm))
with open(os.path.join(S.SCREEN, 's4_extras.csv'), 'w', newline='') as f:
    w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
D = {}
for r in rows: D.setdefault(r['label'], {})[r['year']] = r
for lab, yrs in D.items():
    gw = gl = 0.0
    for y in YEARS:
        p, pf = yrs[y]['pnl'], yrs[y]['pf']
        if pf and pf != float('inf') and pf != 1: g = p / (pf - 1); gl += g; gw += g * pf
    print(f"{lab:46s} 5y {sum(yrs[y]['pnl'] for y in YEARS):+7,.0f} tr {sum(yrs[y]['trades'] for y in YEARS):4d} PF {gw/gl:.2f} | " + ' '.join(f"{100*(yrs[y]['mean_pct'] or 0):+.2f}({yrs[y]['pnl']:+,.0f}/{yrs[y]['trades']})" for y in YEARS))
