"""step 0: one streaming pass over data/scr16_<year>_ticks.csv -> compact per-bar feature table
joined to data/labels/bars_<year>.csv on (date, ticker, epoch ts). usage: python3 build_table.py <year>
writes screen/feat_<year>.csv and prints the join rate. python stdlib only."""
import sys, csv, json, os
from datetime import datetime
sys.path.insert(0, '/home/dylmet/Projects/galactic_trading_firm/scripts/analysis/missed')
from common import hm_et, ROOT

SCREEN = os.path.dirname(os.path.abspath(__file__))
year = int(sys.argv[1])

# (output column, json key)
FEATS = []
for p in ('engulfing','star','three','pin','cloud','harami','inside','3bar','first'):
    for ts in ('1m','5m'):
        FEATS.append((f'cs_{p}_{ts}', f'cs_{p}_{ts}'))
FEATS += [('pdl','pdl_5m'), ('pdl_dlow','pdl_5m.dist_low_pct'), ('pdl_dhigh','pdl_5m.dist_high_pct'),
          ('pdl_dclose','pdl_5m.dist_close_pct'),
          ('or15','or15_1m'), ('or30','or30_1m'),
          ('gap','gap_5m'), ('gap_pct','gap_5m.gap_pct'), ('above_open','gap_5m.above_open'),
          ('cross','cross_1m'), ('x_sess','cross_1m.index_session_ret'), ('x_r5','cross_1m.index_ret_5m'),
          ('x_r15','cross_1m.index_ret_15m'), ('x_peers_red','cross_1m.peers_red_frac'),
          ('ofi_1m','ofi_1m'), ('ofi_5m','ofi_5m'), ('vpin','vpin_1m'), ('vpin_raw','vpin_1m.raw_vpin'),
          ('rvol','rvol_1m'), ('rvol_raw','rvol_1m.rvol'), ('fomc','cal_fomc')]
LABEL_COLS = ['opp_pnl_pct','opp_reason','opp_full_pct','ret_1155','mfe90','mae90']

# labels keyed on (date, ticker, ts)
labels = {}
with open(os.path.join(ROOT, 'data', 'labels', f'bars_{year}.csv')) as f:
    r = csv.DictReader(f)
    for row in r:
        labels[(row['date'], row['ticker'], int(row['ts']))] = [row[c] for c in LABEL_COLS]

def fmt(v):
    if v is None: return ''
    if isinstance(v, float): return f'{v:.4f}'
    return str(v)

out_path = os.path.join(SCREEN, f'feat_{year}.csv')
out = open(out_path, 'w', newline='')
w = csv.writer(out)
w.writerow(['date','ticker','ts','hm','open','comp','s1m','s5m','s1h','event','position'] + [c for c, _ in FEATS] + LABEL_COLS)
n_window = n_join = n_null_json = 0
with open(os.path.join(ROOT, 'data', f'scr16_{year}_ticks.csv')) as f:
    r = csv.reader(f); hdr = next(r); col = {n: i for i, n in enumerate(hdr)}
    ci = [col[k] for k in ('date','ticker','ts','open','composite','s1m','s5m','s1h','position','event','indicators')]
    for rec in r:
        date, ticker, ts, o, comp, s1m, s5m, s1h, pos, ev, ind = (rec[i] for i in ci)
        tse = int(datetime.fromisoformat(ts).timestamp())
        hm = hm_et(tse)
        if hm < 570 or hm > 689: continue
        n_window += 1
        lab = labels.get((date, ticker, tse))
        if lab is None: continue
        n_join += 1
        try:
            d = json.loads(ind)
        except Exception:
            d = {}; n_null_json += 1
        w.writerow([date, ticker, tse, hm, o, comp, s1m, s5m, s1h, ev, pos] + [fmt(d.get(k)) for _, k in FEATS] + lab)
out.close()
print(f'{year}: tick bars 09:30-11:29 = {n_window}, label rows = {len(labels)}, joined = {n_join} '
      f'({100.0*n_join/max(1,n_window):.2f}% of tick bars, {100.0*n_join/max(1,len(labels)):.2f}% of label rows), bad json = {n_null_json}')
