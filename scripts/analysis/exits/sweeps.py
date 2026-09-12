"""analysis 3: exit-rule sweeps (single knob, pairs, time-stop). writes tables_sweeps.md"""
import pickle, sys
from exitsim import *
d = pickle.load(open(f'{OUT}/cache.pkl', 'rb')); trades, paths = d['trades'], d['paths']
out = []
def sect(s): out.append('\n' + s + '\n'); out.append(SCN_HDR)
def scn(name, **kw):
    p = dict(CUR); p.update(kw)
    m = run_scenario(trades, paths, p); out.append(fmt_scn(name, m)); return m
base = None
sect('## S0. baseline + reference')
base = scn('current (90/15/30, stop 2.5%, thr 0.30)')
scn('hold to 11:55 only (no score/stop/maxhold)', thr=None, stop=9.0, max_hold=9999)
scn('no ScoreExit', thr=None)
scn('no HardStop', stop=9.0)
scn('no MaxHold', max_hold=9999)
sect('## S1. max_hold_ms (loss_red 15 / prof_ext 30 kept)')
for mh in (45, 60, 75, 90, 105, 120, 150): scn(f'max_hold {mh}', max_hold=mh)
sect('## S2. loss_reduction_ms')
for lr in (0, 15, 30, 45): scn(f'loss_red {lr}', loss_red=lr)
sect('## S3. profit_extension_ms')
for pe in (0, 30, 60, 90): scn(f'prof_ext {pe}', prof_ext=pe)
sect('## S4. stop_loss_pct')
for s in (0.01, 0.015, 0.02, 0.025, 0.035): scn(f'stop {s*100:.1f}%', stop=s)
sect('## S5. exit_threshold (ScoreExit fires when composite >= T)')
for t in (0.10, 0.20, 0.30, 0.45, None): scn(f'thr {t}' if t is not None else 'thr never', thr=t)
sect('## S6. breakeven trigger_pct (NOTE: engine currently ignores ModifyStop — this simulates a WORKING breakeven stop; needs code)')
for be in (0.005, 0.01, 0.015, None): scn(f'breakeven {be}' if be else 'breakeven never (= engine today)', be=be)
sect('## S7. NEW time-stop: exit if fill_pnl<=0 at hold>=N (needs code)')
for n in (20, 30, 45): scn(f'timestop {n}', tstop=n)
sect('## S8. pairs: max_hold x profit_extension')
for mh in (45, 60, 75, 90):
    for pe in (30, 60, 90): scn(f'max_hold {mh} + prof_ext {pe}', max_hold=mh, prof_ext=pe)
sect('## S9. pairs: max_hold x loss_reduction')
for mh in (60, 75, 90):
    for lr in (0, 30, 45): scn(f'max_hold {mh} + loss_red {lr}', max_hold=mh, loss_red=lr)
sect('## S10. pairs: exit_threshold x max_hold / prof_ext')
for t in (0.20, 0.45, None):
    for mh, pe in ((60, 30), (75, 30), (90, 60), (120, 30)):
        scn(f'thr {t if t is not None else "never"} + max_hold {mh} + prof_ext {pe}', thr=t, max_hold=mh, prof_ext=pe)
sect('## S11. pairs: stop x threshold')
for s in (0.015, 0.02, 0.035):
    for t in (0.20, 0.45): scn(f'stop {s*100:.1f}% + thr {t}', stop=s, thr=t)
sect('## S12. loss-side only: losing limit L = max_hold - loss_red (winning limit fixed at 120)')
for L in (30, 45, 60, 75, 90, 120): scn(f'losing limit {L}, winning 120', max_hold=90, loss_red=90 - L, prof_ext=30)
sect('## S13. winning limit W (losing limit fixed at 75)')
for W in (75, 90, 105, 120, 150, 999): scn(f'losing 75, winning {W}', max_hold=90, loss_red=15, prof_ext=W - 90)
with open(f'{OUT}/tables_sweeps.md', 'w') as f: f.write('\n'.join(out) + '\n')
print('\n'.join(out))
