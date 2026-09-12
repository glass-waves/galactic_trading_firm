"""pivot sim_results.csv into per-variant tables (engine cost model and adverse cost model)."""
import csv, collections, os
from common import OUT, YEARS
R = collections.defaultdict(dict)
order = []
with open(os.path.join(OUT, 'sim_results.csv')) as f:
    for r in csv.DictReader(f):
        if r['variant'] not in order: order.append(r['variant'])
        R[r['variant']][int(r['year'])] = r
base = R['base']
out = open(os.path.join(OUT, 'sim_tables.md'), 'w')
def P(s=''): print(s); out.write(s + '\n')
P("### simulator calibration (base variant, engine cost model) vs actual trade files\n")
P("| year | actual trades / pnl | sim trades / pnl | diff |")
P("|---|---|---|---|")
actual = {2022: (414, 3654), 2023: (209, 479), 2024: (212, 395), 2025: (244, 699), 2026: (207, 665)}
for y in YEARS:
    b = base[y]; P(f"| {y} | {actual[y][0]} / {actual[y][1]:+,} | {b['trades']} / {int(b['pnl_engine']):+,} | {100*(int(b['pnl_engine'])-actual[y][1])/actual[y][1]:+.1f}% |")
for mode, key in (('engine (direction-blind, favourable to shorts — matches the sweep numbers)', 'pnl_engine'), ('adverse (honest 3 bps + $0.005 against the short, each way)', 'pnl_adverse')):
    P(f"\n### per-variant P&L, cost model = {mode}\n")
    P("| variant | " + " | ".join(str(y) for y in YEARS) + " | 5y total | vs base | years better | trades 5y | WR 5y | PF (eng) 5y |")
    P("|---|" + "---|" * (len(YEARS) + 6))
    for v in order:
        cells = []; tot = 0; better = 0; btot = 0; ntr = 0; wins = 0
        for y in YEARS:
            r = R[v][y]; p = int(r[key]); bp = int(base[y][key]); tot += p; btot += bp; ntr += int(r['trades'])
            wins += float(r['win_rate']) * int(r['trades'])
            if p > bp: better += 1
            cells.append(f"{p:+,} ({p-bp:+,})" if v != 'base' else f"{p:+,}")
        P(f"| {v} | " + " | ".join(cells) + f" | {tot:+,} | {tot-btot:+,} | {better}/5 | {ntr} | {100*wins/ntr:.1f}% | - |")
P("\n### marginal trades: what each variant ADDS relative to base (engine cost model) — count / pnl of added trades, count / pnl of base trades it DROPS (displaced by an earlier entry)\n")
P("| variant | " + " | ".join(f"{y} added n/pnl ; dropped n/pnl" for y in YEARS) + " | added 5y n / pnl | added avg $/trade |")
P("|---|" + "---|" * (len(YEARS) + 2))
for v in order:
    if v == 'base': continue
    cells = []; an = 0; ap = 0
    for y in YEARS:
        r = R[v][y]; an += int(r['added_vs_base']); ap += int(r['added_pnl'])
        cells.append(f"{r['added_vs_base']} / {int(r['added_pnl']):+,} ; {r['dropped_vs_base']} / {int(r['dropped_pnl']):+,}")
    P(f"| {v} | " + " | ".join(cells) + f" | {an} / {ap:+,} | {ap/an if an else 0:+.1f} |")
P("\n### per-variant trade counts, win rate, PF, max drawdown (engine cost model)\n")
P("| variant | " + " | ".join(f"{y} n/WR/PF/DD" for y in YEARS) + " |")
P("|---|" + "---|" * len(YEARS))
for v in order:
    P(f"| {v} | " + " | ".join(f"{R[v][y]['trades']}/{100*float(R[v][y]['win_rate']):.0f}%/{R[v][y]['pf_engine']}/{R[v][y]['maxdd_engine']}" for y in YEARS) + " |")
out.close()
