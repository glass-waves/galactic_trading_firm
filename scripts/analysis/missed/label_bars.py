"""step 1+2: label every 09:30-11:29 ET bar with engine category, failing window conditions and
the value of a hypothetical short. usage: python3 label_bars.py <year>  -> writes bars_<year>.csv"""
import sys, csv, os
from common import *

year = int(sys.argv[1])
bars = {t: load_bars(t) for t in TICKERS}

def day_slice(ticker, first_ts, n):
    rows, idx = bars[ticker]
    i0 = idx.get(first_ts)
    if i0 is None: return None
    return rows[i0:i0 + n]

FIELDS = ['year','date','ticker','ts','hm','comp','s1m','s5m','s1h','cat','gate','event','near_miss',
          'win','c_fail','t_s5_fail','t_lag_fail','t_1h_fail','k_s5_fail','k_1h_fail',
          'opp_pnl_pct','opp_reason','opp_full_pct','opp_full_reason','ret_1155','mfe90','mae90']
out = open(os.path.join(OUT, f'bars_{year}.csv'), 'w', newline='')
w = csv.writer(out); w.writerow(FIELDS)
n_days = 0; n_rows = 0; mismatched = 0
for date, ticker, rows in stream_tick_days(year):
    n_days += 1
    day = day_slice(ticker, rows[0]['ts'], len(rows))
    if day is None or len(day) != len(rows) or any(d[0] != r['ts'] for d, r in zip(day, rows)):
        mismatched += 1
        day = [(r['ts'], r['o'], r['h'], r['l'], r['c']) for r in rows]
    comps = [r['comp'] for r in rows]
    for i, r in enumerate(rows):
        if r['hm'] < 570 or r['hm'] > ENTRY_LAST_HM: continue
        g = gate_name(r['blocked_by'])
        if r['event'] == 'open': cat = 'TAKEN'
        elif r['position'] == 'short': cat = 'IN_POSITION'
        elif g: cat = 'GATED'
        elif r['s5m'] is None: cat = 'NOSCORE'
        else: cat = 'FREE'
        if r['s5m'] is None:
            flags = {k: '' for k in ('c_fail','t_s5_fail','t_lag_fail','t_1h_fail','k_s5_fail','k_1h_fail')}; win = ''
        else:
            flags = {k: int(v) for k, v in cond_flags(r['comp'], r['s1m'], r['s5m'], r['s1h']).items()}
            win = window_pass(r['comp'], r['s1m'], r['s5m'], r['s1h']) or ''
        spec = simulate_short(day, i, mode='adverse')
        full = simulate_short(day, i, comps=comps, mode='adverse', score_exit=0.30, breakeven=0.015)
        ps = path_stats(day, i)
        if spec is None or ps is None: continue
        w.writerow([year, date, ticker, r['ts'], r['hm'],
                    '' if r['comp'] is None else f"{r['comp']:.4f}", '' if r['s1m'] is None else f"{r['s1m']:.4f}",
                    '' if r['s5m'] is None else f"{r['s5m']:.4f}", '' if r['s1h'] is None else f"{r['s1h']:.4f}",
                    cat, g, r['event'], int(bool(r['near_miss'])), win,
                    flags['c_fail'], flags['t_s5_fail'], flags['t_lag_fail'], flags['t_1h_fail'], flags['k_s5_fail'], flags['k_1h_fail'],
                    f"{spec['pnl_pct']:.5f}", spec['reason'], f"{full['pnl_pct']:.5f}", full['reason'],
                    f"{ps['ret_1155']:.5f}", f"{ps['mfe']:.5f}", f"{ps['mae']:.5f}"])
        n_rows += 1
out.close()
print(f"{year}: ticker-days={n_days} labelled_bars={n_rows} bar/tick path mismatches={mismatched}")
