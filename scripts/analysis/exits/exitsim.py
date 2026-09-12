"""offline exit-rule simulator for v15c short-only trades. stdlib only.

usage: python3 exitsim.py  (from anywhere; paths are absolute)
loads trades, bars, tick composites; reconstructs paths; verifies reproduction; runs sweeps.
writes tables.md + paths.csv + repro_mismatches.csv into this directory.
"""
import csv, os, sys, statistics, pickle
from collections import defaultdict
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

REPO = '/home/dylmet/Projects/galactic_trading_firm'
OUT = os.path.dirname(os.path.abspath(__file__))
NY = ZoneInfo('America/New_York')
YEARS = [2022, 2023, 2024, 2025, 2026]
TICKERS = ['AMZN', 'AAPL', 'NVDA', 'MSFT']
SLIP = 0.0003
HALF = 0.005
CLOSE_MIN = 11 * 60 + 55   # force_exit_by 11:55 ET
CUR = dict(stop=0.025, thr=0.30, max_hold=90, loss_red=15, prof_ext=30, be=None, tstop=None)

def entry_fill(o): return o * (1 + SLIP) + HALF
def exit_fill(o): return o * (1 - SLIP) - HALF

# ---------------------------------------------------------------- loading
def load_trades():
    trades = []
    for y in YEARS:
        with open(f'{REPO}/data/v15c_{y}_trades.csv') as f:
            for r in csv.DictReader(f):
                if r['row_type'] != 'trade':
                    continue
                t = dict(year=y, date=r['date'], ticker=r['ticker'],
                         entry_ts=int(datetime.fromisoformat(r['entry_time']).timestamp()),
                         exit_ts=int(datetime.fromisoformat(r['exit_time']).timestamp()),
                         entry_price=float(r['entry_price']), exit_price=float(r['exit_price']),
                         size=float(r['size']), pnl=float(r['pnl']), pnl_pct=float(r['pnl_pct']),
                         hold_min=int(r['hold_duration_ms']) // 60000, reason=r['exit_reason'],
                         entry_reason=r['entry_reason'])
                trades.append(t)
    trades.sort(key=lambda t: (t['ticker'], t['entry_ts']))
    return trades

def et_minutes(ts):
    d = datetime.fromtimestamp(ts, NY)
    return d.hour * 60 + d.minute

def load_bars(days_needed):
    """bars[ticker][date] -> list of (ts, o, h, l, c) for RTH bars of that ET date."""
    bars = {tk: defaultdict(list) for tk in TICKERS}
    for tk in TICKERS:
        need = days_needed[tk]
        with open(f'{REPO}/data/bars/{tk}.csv') as f:
            rd = csv.reader(f); next(rd)
            cur_day = None; cur_key = None
            for r in rd:
                ts = int(r[0])
                day = ts // 86400  # UTC day; RTH never crosses UTC midnight
                if day != cur_day:
                    cur_day = day
                    cur_key = datetime.fromtimestamp(ts, NY).strftime('%Y-%m-%d')
                if cur_key in need:
                    bars[tk][cur_key].append((ts, float(r[1]), float(r[2]), float(r[3]), float(r[4])))
    return bars

def load_ticks(days_needed):
    """comp[(ticker, ts)] -> composite (float|None); also engine state for cross-check."""
    comp = {}
    state = {}
    for y in YEARS:
        with open(f'{REPO}/data/v15c_{y}_ticks.csv') as f:
            rd = csv.reader(f); hdr = next(rd)
            ix = {h: i for i, h in enumerate(hdr)}
            for r in rd:
                tk = r[ix['ticker']]
                if r[ix['date']] not in days_needed[tk]:
                    continue
                ts = int(datetime.fromisoformat(r[ix['ts']]).timestamp())
                c = r[ix['composite']]
                comp[(tk, ts)] = float(c) if c else None
                state[(tk, ts)] = (r[ix['position']], r[ix['unrealized_pct']], r[ix['hold_min']], r[ix['event']])
    return comp, state

# ---------------------------------------------------------------- paths
def build_path(t, bars, comp):
    """per-bar path from entry fill bar to the 11:55 bar (inclusive) + fill open of the bar after each.
    returns list of dicts: hold, ts, o, h, l, c, comp, unreal (close-based, +=profit), fill_pnl (if exit signalled here)."""
    day = bars[t['ticker']].get(t['date'], [])
    idx = next((i for i, b in enumerate(day) if b[0] == t['entry_ts']), None)
    if idx is None:
        return None
    ep = t['entry_price']
    path = []
    for j in range(idx, len(day)):
        ts, o, h, l, c = day[j]
        nxt = day[j + 1] if j + 1 < len(day) else None
        fill = exit_fill(nxt[1]) if nxt else c
        path.append(dict(hold=(ts - t['entry_ts']) // 60, ts=ts, o=o, h=h, l=l, c=c,
                         comp=comp.get((t['ticker'], ts)),
                         unreal=(ep - c) / ep, fill_pnl=(ep - fill) / ep, fill=fill,
                         etm=et_minutes(ts)))
        if et_minutes(ts) >= CLOSE_MIN:
            break
    return path

# ---------------------------------------------------------------- simulator
def simulate(t, path, p):
    """replicates the engine exit stack on a reconstructed path.
    p: stop, thr (None=never), max_hold, loss_red, prof_ext (minutes), be (breakeven trigger or None),
       tstop (N minutes: exit if not profitable at hold>=N, or None).
    returns dict(reason, ts, fill, pnl_pct, hold)."""
    ep = t['entry_price']
    lwm = ep
    be_armed = False
    for b in path:
        c = b['c']
        if c < lwm: lwm = c
        unreal = b['unreal']
        reason = None
        if b['etm'] >= CLOSE_MIN:
            reason = 'SessionClose'
        elif p['thr'] is not None and b['comp'] is not None and b['comp'] >= p['thr']:
            reason = 'ScoreExit'
        elif (c - ep) / ep >= p['stop']:
            reason = 'HardStop'
        else:
            if p['be'] is not None and (ep - lwm) / ep >= p['be']:
                be_armed = True
            if be_armed and c >= ep:
                reason = 'BreakevenStop'
            elif p['tstop'] is not None and b['hold'] >= p['tstop'] and unreal <= 0:
                reason = 'TimeStop'
            else:
                if unreal > 0: eff = p['max_hold'] + p['prof_ext']
                elif unreal < 0: eff = max(p['max_hold'] - p['loss_red'], 0)
                else: eff = p['max_hold']
                if b['hold'] >= eff:
                    reason = 'MaxHoldTimeout'
        if reason:
            return dict(reason=reason, ts=b['ts'] + 60, fill=b['fill'], pnl_pct=b['fill_pnl'],
                        hold=b['hold'] + 1, pnl=b['fill_pnl'] * ep * t['size'])
    b = path[-1]
    return dict(reason='EndOfData', ts=b['ts'], fill=b['c'], pnl_pct=b['unreal'], hold=b['hold'],
                pnl=b['unreal'] * ep * t['size'])

def bucket(pct):
    return 'loss' if pct < -0.003 else ('win' if pct > 0.003 else 'breakeven')

# ---------------------------------------------------------------- metrics
def metrics(rows):
    """rows: list of (year, exit_ts, pnl). returns dict per year + total."""
    out = {}
    by = defaultdict(list)
    for y, ts, pnl in rows: by[y].append((ts, pnl))
    for y, lst in by.items():
        lst.sort()
        pnls = [p for _, p in lst]
        gw = sum(p for p in pnls if p > 0); gl = -sum(p for p in pnls if p < 0)
        eq = 0; peak = 0; dd = 0
        for p in pnls:
            eq += p; peak = max(peak, eq); dd = max(dd, peak - eq)
        out[y] = dict(n=len(pnls), pnl=sum(pnls), wr=sum(1 for p in pnls if p > 0) / len(pnls) if pnls else 0,
                      pf=(gw / gl) if gl > 0 else float('inf'), dd=dd)
    return out

def apply_overlap(trades, sims):
    """drop trades whose entry-signal bar is not after the simulated exit fill of the previous trade
    on the same ticker/date (they could not have been entered). returns list of kept indices."""
    kept = []
    last_exit = {}
    for i, t in enumerate(trades):
        key = (t['ticker'], t['date'])
        le = last_exit.get(key)
        if le is not None and t['entry_ts'] - 60 <= le:
            continue  # shadowed
        kept.append(i)
        last_exit[key] = sims[i]['ts']
    return kept

def run_scenario(trades, paths, p):
    sims = [simulate(t, paths[i], p) for i, t in enumerate(trades)]
    kept = apply_overlap(trades, sims)
    rows = [(trades[i]['year'], sims[i]['ts'], sims[i]['pnl']) for i in kept]
    m = metrics(rows)
    m['dropped'] = len(trades) - len(kept)
    m['total'] = sum(m[y]['pnl'] for y in YEARS if y in m)
    m['sims'] = sims; m['kept'] = kept
    return m

def fmt_scn(name, m):
    cells = [name]
    for y in YEARS:
        cells.append(f"{m[y]['pnl']:+.0f}" if y in m else '-')
    pos = sum(1 for y in YEARS if y in m and m[y]['pnl'] >= 0)
    n = sum(m[y]['n'] for y in YEARS if y in m)
    pnls_all = []
    cells += [f"{m['total']:+.0f}", str(pos), str(n), str(m['dropped'])]
    wr = sum(m[y]['wr'] * m[y]['n'] for y in YEARS if y in m) / max(n, 1)
    cells.append(f"{wr:.1%}")
    cells.append(' / '.join(f"{m[y]['pf']:.2f}" for y in YEARS if y in m))
    cells.append(' / '.join(f"{m[y]['dd']:.0f}" for y in YEARS if y in m))
    return '| ' + ' | '.join(cells) + ' |'

SCN_HDR = ('| scenario | 2022 | 2023 | 2024 | 2025 | 2026 | 5y total | yrs>=0 | n | dropped | win% | PF 22/23/24/25/26 | maxDD$ 22/23/24/25/26 |\n'
           '|---|---|---|---|---|---|---|---|---|---|---|---|---|')

if __name__ == '__main__':
    trades = load_trades()
    days_needed = {tk: set() for tk in TICKERS}
    for t in trades: days_needed[t['ticker']].add(t['date'])
    print('trades', len(trades), file=sys.stderr)
    bars = load_bars(days_needed); print('bars loaded', file=sys.stderr)
    comp, state = load_ticks(days_needed); print('ticks loaded', len(comp), file=sys.stderr)
    paths = [build_path(t, bars, comp) for t in trades]
    missing = [i for i, p in enumerate(paths) if p is None]
    print('paths missing', len(missing), file=sys.stderr)
    with open(f'{OUT}/cache.pkl', 'wb') as f:
        pickle.dump(dict(trades=trades, paths=paths, state=state), f)
    print('cached', file=sys.stderr)
