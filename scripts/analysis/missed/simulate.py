"""step 4+5: re-simulate the strategy with loosened / tightened window thresholds.
usage: python3 simulate.py [years...]   -> sim_results.csv, sim_calibration.txt, sim_trades_<variant>_<year>.csv (base only)"""
import sys, csv, os, collections
from common import *

years = [int(a) for a in sys.argv[1:]] or YEARS

VARIANTS = collections.OrderedDict()
def V(name, **kw):
    p = dict(BASE); p.update(kw); VARIANTS[name] = p
V('base')
# loosenings
V('cmax_-0.30', cmax=-0.30); V('cmax_-0.25', cmax=-0.25)
V('t_s5_-0.40', t_s5=-0.40); V('t_s5_-0.30', t_s5=-0.30)
V('k_s5_-0.30', k_s5=-0.30)
V('lag_0.05', lag=0.05); V('lag_0.00', lag=0.0)
V('t_1h_+0.10', t_1h=0.10); V('t_1h_+0.20', t_1h=0.20)
V('k_1h_-0.15', k_1h=-0.15); V('k_1h_0.00', k_1h=0.0)
V('no_s1h_both', t_1h=9.0, k_1h=9.0)
V('no_reject_gates', reject=False)
V('cmax_-0.30+k_1h_-0.15', cmax=-0.30, k_1h=-0.15)
# tightenings
V('cmax_-0.40', cmax=-0.40); V('cmax_-0.45', cmax=-0.45)
V('t_1h_-0.30_both', t_1h=-0.30)
V('t_s5_-0.60', t_s5=-0.60); V('k_s5_-0.50', k_s5=-0.50)
V('lag_0.15', lag=0.15); V('k_1h_-0.40', k_1h=-0.40)
# diagnostics: single windows
V('thrust_only', k_s5=-9.0); V('core_only', t_s5=-9.0)

def load_year(year):
    days = []
    for date, ticker, rows in stream_tick_days(year):
        days.append((date, ticker, rows))
    return days

def run_variant(days, p, mode):
    """returns list of trades: (date,ticker,sig_idx,e_ts,x_ts,reason,pnl,pnl_pct,size,win)"""
    trades = []
    for date, ticker, rows in days:
        day = [(r['ts'], r['o'], r['h'], r['l'], r['c']) for r in rows]
        comps = [r['comp'] for r in rows]
        i = 0; n = len(rows)
        while i < n:
            r = rows[i]
            if r['hm'] > ENTRY_LAST_HM: break
            if r['hm'] < 570 or r['s5m'] is None: i += 1; continue
            if p['reject'] and reject_fires(r['s1m'], r['s5m'], r['s1h']): i += 1; continue
            win = window_pass(r['comp'], r['s1m'], r['s5m'], r['s1h'], p)
            if not win: i += 1; continue
            t = simulate_short(day, i, comps=comps, mode=mode, score_exit=0.30, breakeven=None)  # engine's breakeven monitor never produced an exit in the trade files; modelling it broke calibration
            if t is None: break
            trades.append((date, ticker, i, day[t['e_idx']][0], day[t['x_idx']][0], t['reason'], t['pnl'], t['pnl_pct'], t['size'], win))
            i = t['x_idx'] + 1   # exit-fill bar is blocked by entry_cooldown; next bar is free
    return trades

def summarize(trades):
    n = len(trades); pnl = sum(t[6] for t in trades)
    gp = sum(t[6] for t in trades if t[6] > 0); gl = -sum(t[6] for t in trades if t[6] < 0)
    wr = sum(1 for t in trades if t[6] > 0) / n if n else 0
    cum = 0; peak = 0; dd = 0
    for t in sorted(trades, key=lambda t: t[4]):
        cum += t[6]; peak = max(peak, cum); dd = max(dd, peak - cum)
    return dict(n=n, pnl=pnl, wr=wr, pf=(gp / gl if gl else float('inf')), maxdd=dd,
                reasons=collections.Counter(t[5] for t in trades), wins=collections.Counter(t[9] for t in trades))

res_path = os.path.join(OUT, 'sim_results.csv')
write_hdr = not os.path.exists(res_path) or years == YEARS
res = open(res_path, 'a' if not write_hdr else 'w', newline=''); rw = csv.writer(res)
if write_hdr: rw.writerow(['variant','year','trades','pnl_engine','pnl_adverse','win_rate','pf_engine','maxdd_engine','thrust','core','added_vs_base','added_pnl','dropped_vs_base','dropped_pnl','exit_mix'])
cal = open(os.path.join(OUT, 'sim_calibration.txt'), 'a' if not write_hdr else 'w')

for year in years:
    days = load_year(year)
    actual = load_trades(year)
    base_engine = run_variant(days, VARIANTS['base'], 'engine')
    # ---- calibration against the actual trade file ----
    act = {(t['date'], t['ticker'], t['entry_ts']): t for t in actual}
    sim = {(t[0], t[1], t[3]): t for t in base_engine}
    both = set(act) & set(sim)
    exit_ok = sum(1 for k in both if act[k]['exit_ts'] == sim[k][4])
    reason_ok = sum(1 for k in both if act[k]['exit_reason'] == sim[k][5])
    pnl_err = sum(abs(act[k]['pnl'] - sim[k][6]) for k in both)
    s = summarize(base_engine)
    msg = (f"{year}: actual trades={len(actual)} pnl={sum(t['pnl'] for t in actual):.0f} | sim base trades={s['n']} pnl={s['pnl']:.0f} "
           f"| entry-bar matches={len(both)} exit-bar matches={exit_ok} reason matches={reason_ok} sum|pnl err| on matched={pnl_err:.1f}\n")
    unmatched_a = [k for k in act if k not in sim][:5]; unmatched_s = [k for k in sim if k not in act][:5]
    msg += f"   actual-not-sim (first 5): {[(k[0],k[1],datetime.fromtimestamp(k[2],timezone.utc).strftime('%H:%M')) for k in unmatched_a]}\n"
    msg += f"   sim-not-actual (first 5): {[(k[0],k[1],datetime.fromtimestamp(k[2],timezone.utc).strftime('%H:%M')) for k in unmatched_s]}\n"
    mism = [(k, act[k]['exit_reason'], sim[k][5], datetime.fromtimestamp(act[k]['exit_ts'],timezone.utc).strftime('%H:%M'), datetime.fromtimestamp(sim[k][4],timezone.utc).strftime('%H:%M')) for k in both if act[k]['exit_ts'] != sim[k][4]][:6]
    msg += f"   exit mismatches (first 6): {mism}\n"
    print(msg); cal.write(msg)
    base_keys = set(sim)
    with open(os.path.join(OUT, f'sim_trades_base_{year}.csv'), 'w', newline='') as f:
        ww = csv.writer(f); ww.writerow(['date','ticker','sig_idx','entry_ts','exit_ts','reason','pnl','pnl_pct','size','window'])
        for t in base_engine: ww.writerow(t)
    for name, p in VARIANTS.items():
        te = base_engine if name == 'base' else run_variant(days, p, 'engine')
        ta = run_variant(days, p, 'adverse')
        s = summarize(te); sa = summarize(ta)
        keys = {(t[0], t[1], t[3]): t for t in te}
        added = [keys[k] for k in keys if k not in base_keys]; dropped = [sim[k] for k in base_keys if k not in keys]
        rw.writerow([name, year, s['n'], f"{s['pnl']:.0f}", f"{sa['pnl']:.0f}", f"{s['wr']:.3f}", f"{s['pf']:.2f}", f"{s['maxdd']:.0f}",
                     s['wins'].get('thrust', 0), s['wins'].get('core', 0), len(added), f"{sum(t[6] for t in added):.0f}",
                     len(dropped), f"{sum(t[6] for t in dropped):.0f}", ' '.join(f"{k}:{v}" for k, v in sorted(s['reasons'].items()))])
        res.flush()
    print(f"{year}: {len(VARIANTS)} variants done")
res.close(); cal.close()
