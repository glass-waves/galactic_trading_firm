"""entry-feature screen, round two (2026-09-12). python stdlib only.
reads screen/feat_<year>.csv (built by build_table.py) + data/bars/*.csv + research/entries/data/earnings_*.txt
runs the five analyses of screen_brief.md per year, writes s1..s5 csv + tables.md.
usage: python3 screen.py            (all years; ~2-3 min)
"""
import sys, csv, os, json, statistics, itertools
sys.path.insert(0, '/home/dylmet/Projects/galactic_trading_firm/scripts/analysis/missed')
from common import hm_et, et_date, load_bars, simulate_short, reject_fires, TICKERS, ROOT

SCREEN = os.path.dirname(os.path.abspath(__file__))
YEARS = [2022, 2023, 2024, 2025, 2026]
CS_PATTERNS = ('engulfing', 'star', 'three', 'pin', 'cloud', 'harami', 'inside', '3bar', 'first')
V16 = dict(cmax=-0.35, t_s5=-0.50, lag=0.10, t_1h=0.0, k_s5=-0.40, k_1h=-0.40)
EXIT_V16 = dict(stop_pct=0.025, max_hold=90, loss_red=50, profit_ext=0, breakeven=0.005, score_exit=0.30)
EXIT_LABEL = dict(stop_pct=0.025, max_hold=90, loss_red=15, profit_ext=30)   # the label's v15 stack
MIN_EPS = 30

# ---------------------------------------------------------------- bars (sim + cross features)
def group_days(rows):
    """rows sorted (ts,o,h,l,c) -> dict date -> (slice, ts->local idx). new day when gap > 4h."""
    days = {}; start = 0
    for i in range(1, len(rows) + 1):
        if i == len(rows) or rows[i][0] - rows[i - 1][0] > 4 * 3600:
            sl = rows[start:i]
            days[et_date(sl[0][0])] = (sl, {b[0]: k for k, b in enumerate(sl)})
            start = i
    return days

print('loading bars ...', flush=True)
DAYS = {t: group_days(load_bars(t)[0]) for t in TICKERS + ['SPY']}

# earnings: filing date (after close) -> reaction session = next trading date; 'pre' = filing date session
EARN = {}   # (ticker, date) -> 'react' | 'pre'
for t in TICKERS:
    dates = sorted(DAYS[t])
    with open(os.path.join(ROOT, 'research', 'entries', 'data', f'earnings_{t}.txt')) as f:
        for line in f:
            d = line.strip()
            if not d: continue
            pre = max((x for x in dates if x <= d), default=None)
            react = min((x for x in dates if x > d), default=None)
            if pre: EARN[(t, pre)] = 'pre'
            if react: EARN[(t, react)] = 'react'

# ---------------------------------------------------------------- pooled quintile edges (fixed across years)
QCOLS = ['ofi_1m', 'ofi_5m', 'vpin_raw', 'rvol_raw']
def quintile_edges():
    cache = os.path.join(SCREEN, 'quintile_edges.json')
    if os.path.exists(cache):
        return json.load(open(cache))
    vals = {c: [] for c in QCOLS}
    for y in YEARS:
        with open(os.path.join(SCREEN, f'feat_{y}.csv')) as f:
            r = csv.DictReader(f)
            for row in r:
                for c in QCOLS:
                    if row[c] != '': vals[c].append(float(row[c]))
    edges = {}
    for c in QCOLS:
        v = sorted(vals[c]); n = len(v)
        edges[c] = [v[int(n * q)] for q in (0.2, 0.4, 0.6, 0.8)]
    json.dump(edges, open(cache, 'w'), indent=1)
    return edges
print('quintile edges ...', flush=True)
EDGES = quintile_edges()
print(json.dumps(EDGES))

# ---------------------------------------------------------------- load one year (columnar)
def fnum(s):
    return float(s) if s != '' else None

def load_year(year):
    cols = None
    with open(os.path.join(SCREEN, f'feat_{year}.csv')) as f:
        r = csv.reader(f); hdr = next(r)
        cols = {h: [] for h in hdr}
        numeric = set(hdr) - {'date', 'ticker', 'event', 'position', 'opp_reason'}
        for rec in r:
            for h, v in zip(hdr, rec):
                cols[h].append(fnum(v) if h in numeric else v)
    cols['ts'] = [int(x) for x in cols['ts']]
    cols['hm'] = [int(x) for x in cols['hm']]
    n = len(cols['ts'])
    # derived cross features from the bar cache (indicator was null on every bar in the dump)
    spy = DAYS['SPY']
    x_sess = [None] * n; x_r5 = [None] * n; x_r15 = [None] * n; peers = [None] * n
    for i in range(n):
        d = cols['date'][i]; ts = cols['ts'][i]
        sd = spy.get(d)
        if sd:
            rows, idx = sd; j = idx.get(ts)
            if j is not None:
                c = rows[j][4]
                x_sess[i] = 100 * (c / rows[0][1] - 1)
                if j >= 5: x_r5[i] = 100 * (c / rows[j - 5][4] - 1)
                if j >= 15: x_r15[i] = 100 * (c / rows[j - 15][4] - 1)
        red = 0; tot = 0
        for p in TICKERS:
            if p == cols['ticker'][i]: continue
            pd = DAYS[p].get(d)
            if not pd: continue
            rows, idx = pd; j = idx.get(ts)
            if j is None: continue
            tot += 1
            if rows[j][4] < rows[0][1]: red += 1
        if tot == 3: peers[i] = red / 3
    cols['x_sess'] = x_sess; cols['x_r5'] = x_r5; cols['x_r15'] = x_r15; cols['x_peers_red'] = peers
    # split-day artefacts (AMZN 2022-06-06 20:1, NVDA 2024-06-10 10:1): |gap| > 15 % -> null gap/pdl for the day
    bad_days = {(cols['ticker'][i], cols['date'][i]) for i in range(n) if cols['gap_pct'][i] is not None and abs(cols['gap_pct'][i]) > 15}
    for i in range(n):
        if (cols['ticker'][i], cols['date'][i]) in bad_days:
            for c in ('gap', 'gap_pct', 'above_open', 'pdl', 'pdl_dlow', 'pdl_dhigh', 'pdl_dclose'):
                cols[c][i] = None
    cols['earn'] = [EARN.get((cols['ticker'][i], cols['date'][i]), '') for i in range(n)]
    return cols, n, bad_days

# ---------------------------------------------------------------- feature definitions
def win_flags(cols, n):
    """v16 windows recomputed from the scores (not `event`): returns (win, thrust, core) bool lists."""
    win = [False] * n; thr = [False] * n; core = [False] * n
    C, A, B, H = cols['comp'], cols['s1m'], cols['s5m'], cols['s1h']
    for i in range(n):
        c, a, b, h = C[i], A[i], B[i], H[i]
        if c is None or a is None or b is None or h is None: continue
        if c > V16['cmax']: continue
        if reject_fires(a, b, h): continue
        t = b <= V16['t_s5'] and b <= a - V16['lag'] and b <= h - V16['lag'] and h <= V16['t_1h']
        k = b <= V16['k_s5'] and h <= V16['k_1h']
        thr[i] = t; core[i] = k; win[i] = t or k
    return win, thr, core

def gt(x, a): return x is not None and x > a
def ge(x, a): return x is not None and x >= a
def lt(x, a): return x is not None and x < a
def le(x, a): return x is not None and x <= a
def between(x, a, b): return x is not None and a <= x < b

def features(cols, n):
    """ordered dict name -> (group, bool list). group 'ref' = reference only (not a candidate)."""
    F = {}
    def add(name, group, fn, col=None):
        src = cols[col] if col else None
        F[name] = (group, [fn(src[i]) if col else fn(i) for i in range(n)])
    for ts in ('1m', '5m'):
        for p in CS_PATTERNS:
            add(f'cs_{p}_{ts} bear', 'candle', lambda x: x is not None and x <= -0.5, f'cs_{p}_{ts}')
    for p in CS_PATTERNS:
        add(f'cs_{p}_5m BULL (ref)', 'ref', lambda x: x is not None and x >= 0.5, f'cs_{p}_5m')
    # prior-day levels
    add('pdl below prior low (pdl<=-0.5)', 'pdl', lambda x: le(x, -0.5), 'pdl')
    add('pdl above prior high (ref)', 'ref', lambda x: ge(x, 0.5), 'pdl')
    for lo, hi, nm in ((-99, -1, 'pdl dist_low < -1%'), (-1, -0.5, 'pdl dist_low [-1,-0.5)%'), (-0.5, 0, 'pdl dist_low [-0.5,0)%'),
                       (0, 0.5, 'pdl dist_low [0,0.5)%'), (0.5, 1, 'pdl dist_low [0.5,1)%'), (1, 2, 'pdl dist_low [1,2)%'), (2, 99, 'pdl dist_low >= 2%')):
        add(nm, 'pdl', lambda x, lo=lo, hi=hi: between(x, lo, hi), 'pdl_dlow')
    add('pdl below prior close', 'pdl', lambda x: lt(x, 0), 'pdl_dclose')
    add('pdl above prior close (ref)', 'ref', lambda x: ge(x, 0), 'pdl_dclose')
    # opening range
    add('or15 below range low', 'or', lambda x: le(x, -0.5), 'or15')
    add('or30 below range low', 'or', lambda x: le(x, -0.5), 'or30')
    add('or15 above range high (ref)', 'ref', lambda x: ge(x, 0.5), 'or15')
    add('or30 above range high (ref)', 'ref', lambda x: ge(x, 0.5), 'or30')
    add('or15 lower half of range', 'or', lambda x: between(x, -0.5, -0.15), 'or15')
    # gap
    for lo, hi, nm in ((-99, -1, 'gap <= -1%'), (-1, -0.3, 'gap (-1,-0.3]%'), (-0.3, 0.3, 'gap (-0.3,0.3)%'), (0.3, 1, 'gap [0.3,1)%'), (1, 99, 'gap >= 1%')):
        add(nm, 'gap', lambda x, lo=lo, hi=hi: x is not None and (lo < x <= hi if hi <= -0.3 else (lo < x < hi if hi == 0.3 else lo <= x < hi)), 'gap_pct')
    G, AO = cols['gap_pct'], cols['above_open']
    add('gap up >=0.3% & fading (below open)', 'gap', lambda i: ge(G[i], 0.3) and lt(AO[i], 0))
    add('gap up >=0.3% & holding (above open)', 'gap', lambda i: ge(G[i], 0.3) and ge(AO[i], 0))
    add('gap down <=-0.3% & extending (below open)', 'gap', lambda i: le(G[i], -0.3) and lt(AO[i], 0))
    add('gap down <=-0.3% & filling (above open)', 'gap', lambda i: le(G[i], -0.3) and ge(AO[i], 0))
    add('below today open (above_open<0)', 'gap', lambda x: lt(x, 0), 'above_open')
    # cross (derived offline from data/bars)
    for lo, hi, nm in ((-99, -0.5, 'SPY session <= -0.5%'), (-0.5, -0.2, 'SPY session (-0.5,-0.2]%'), (-0.2, 0.2, 'SPY session (-0.2,0.2)%'),
                       (0.2, 0.5, 'SPY session [0.2,0.5)%'), (0.5, 99, 'SPY session >= 0.5%')):
        add(nm, 'cross', lambda x, lo=lo, hi=hi: x is not None and (lo < x <= hi if hi <= -0.2 else (lo < x < hi if hi == 0.2 else lo <= x < hi)), 'x_sess')
    add('SPY 5m ret <= -0.15%', 'cross', lambda x: le(x, -0.15), 'x_r5')
    add('SPY 5m ret < 0', 'cross', lambda x: lt(x, 0), 'x_r5')
    add('SPY 5m ret >= +0.15% (ref)', 'ref', lambda x: ge(x, 0.15), 'x_r5')
    add('SPY 15m ret <= -0.3%', 'cross', lambda x: le(x, -0.3), 'x_r15')
    add('SPY 15m ret < 0', 'cross', lambda x: lt(x, 0), 'x_r15')
    add('SPY 15m ret >= +0.3% (ref)', 'ref', lambda x: ge(x, 0.3), 'x_r15')
    for k in range(4):
        add(f'peers red frac = {k}/3', 'cross', lambda x, k=k: x is not None and abs(x - k / 3) < 0.01, 'x_peers_red')
    # quintiles (pooled edges)
    for c in QCOLS:
        e = EDGES[c]
        for q in range(5):
            lo = -1e18 if q == 0 else e[q - 1]; hi = 1e18 if q == 4 else e[q]
            add(f'{c} Q{q + 1} [{lo:.3g},{hi:.3g})' if q not in (0, 4) else (f'{c} Q1 (<{hi:.3g})' if q == 0 else f'{c} Q5 (>={lo:.3g})'),
                'flow', lambda x, lo=lo, hi=hi: between(x, lo, hi), c)
    # calendar
    add('FOMC decision day', 'cal', lambda x: le(x, -0.5), 'fomc')
    add('earnings reaction day', 'cal', lambda x: x == 'react', 'earn')
    add('earnings day-before (filing session)', 'cal', lambda x: x == 'pre', 'earn')
    # time of day reference
    HM = cols['hm']
    for k in range(12):
        h0 = 570 + 10 * k
        add(f'time {h0 // 60:02d}:{h0 % 60:02d}-{(h0 + 9) // 60:02d}:{(h0 + 9) % 60:02d} (ref)', 'ref', lambda i, h0=h0: h0 <= HM[i] < h0 + 10)
    return F

# ---------------------------------------------------------------- stats
def episodes(mask, keys):
    """runs of consecutive firing bars within a ticker-day. returns list of first-bar indices."""
    out = []; prev = False; pk = None
    for i, m in enumerate(mask):
        k = keys[i]
        if m and not (prev and pk == k): out.append(i)
        prev = m; pk = k
    return out

def stats(mask, opp, keys, top_thr, base_mean):
    idx = [i for i, m in enumerate(mask) if m]
    if not idx:
        return dict(bars=0, eps=0, mean=None, median=None, hit=None, top5=None, lift=None, ep_mean=None)
    v = [opp[i] for i in idx]
    eps = episodes(mask, keys)
    ev = [opp[i] for i in eps]
    return dict(bars=len(idx), eps=len(eps), mean=statistics.mean(v), median=statistics.median(v),
                hit=sum(1 for x in v if x > 0) / len(v), top5=sum(1 for x in v if x >= top_thr) / len(v),
                lift=statistics.mean(v) - base_mean, ep_mean=statistics.mean(ev), ep_hit=sum(1 for x in ev if x > 0) / len(ev))

def AND(a, b): return [x and y for x, y in zip(a, b)]
def NOT(a): return [not x for x in a]

# ---------------------------------------------------------------- simulation
def simulate(mask, cols, n, exit_params):
    """one position per ticker, take the first firing bar, nothing until the exit-fill bar (cooldown
    blocks exactly that bar). returns list of (pnl$, pnl_pct, reason)."""
    trades = []; cur = None; busy = -1; day = idx = comps = None
    T, D, TS, C = cols['ticker'], cols['date'], cols['ts'], cols['comp']
    for i in range(n):
        key = (T[i], D[i])
        if key != cur:
            cur = key; busy = -1
            dd = DAYS[T[i]].get(D[i])
            if dd is None: day = None; continue
            day, idx = dd
            comps = [None] * len(day)
            j = i
            while j < n and T[j] == T[i] and D[j] == D[i]:
                k = idx.get(TS[j])
                if k is not None: comps[k] = C[j]
                j += 1
        if day is None or not mask[i]: continue
        j = idx.get(TS[i])
        if j is None or j <= busy: continue
        r = simulate_short(day, j, comps=comps, mode='adverse', **exit_params)
        if r is None: continue
        trades.append((r['pnl'], r['pnl_pct'], r['reason']))
        busy = r['x_idx']
    return trades

def sim_summary(trades):
    if not trades: return dict(trades=0, pnl=0.0, win=None, pf=None, mean_pct=None)
    p = [t[0] for t in trades]
    gw = sum(x for x in p if x > 0); gl = -sum(x for x in p if x < 0)
    return dict(trades=len(p), pnl=sum(p), win=sum(1 for x in p if x > 0) / len(p),
                pf=(gw / gl if gl > 0 else float('inf')), mean_pct=statistics.mean(t[1] for t in trades))

# ---------------------------------------------------------------- main
def pct(x, d=3):
    return '' if x is None else f'{100 * x:.{d}f}'

def main():
    s1 = []; s2 = []; s3 = []; s5 = []; base = {}
    feat_names = None; feat_groups = {}
    for year in YEARS:
        print(f'== {year}: loading', flush=True)
        cols, n, bad_days = load_year(year)
        keys = list(zip(cols['ticker'], cols['date']))
        opp = cols['opp_pnl_pct']
        top_thr = sorted(opp)[int(0.95 * n)]
        base_mean = statistics.mean(opp)
        base[year] = dict(bars=n, mean=base_mean, hit=sum(1 for x in opp if x > 0) / n, top_thr=top_thr,
                          median=statistics.median(opp), split_days=sorted(bad_days))
        win, thr, core = win_flags(cols, n)
        F = features(cols, n)
        if feat_names is None:
            feat_names = list(F); feat_groups = {k: v[0] for k, v in F.items()}
        F['v16 window (any)'] = ('window', win); F['v16 thrust'] = ('window', thr); F['v16 core'] = ('window', core)
        feat_groups.update({'v16 window (any)': 'window', 'v16 thrust': 'window', 'v16 core': 'window'})
        S5, S1M = cols['s5m'], cols['s1m']
        # --- 1. feature alone
        print(f'== {year}: screen 1', flush=True)
        for name, (grp, m) in F.items():
            st = stats(m, opp, keys, top_thr, base_mean)
            s1.append(dict(feature=name, group=grp, year=year, **st))
        # --- 2. conditional on the window
        print(f'== {year}: screen 2', flush=True)
        wst = stats(win, opp, keys, top_thr, base_mean)
        for name, (grp, m) in F.items():
            if grp == 'window': continue
            a = stats(AND(win, m), opp, keys, top_thr, base_mean)
            b = stats(AND(win, NOT(m)), opp, keys, top_thr, base_mean)
            s2.append(dict(feature=name, group=grp, year=year, win_bars=wst['bars'], win_eps=wst['eps'], win_mean=wst['mean'], win_hit=wst['hit'],
                           req_bars=a['bars'], req_eps=a['eps'], req_mean=a['mean'], req_hit=a['hit'], req_ep_mean=a['ep_mean'],
                           req_keep=(a['eps'] / wst['eps'] if wst['eps'] else None),
                           excl_bars=b['bars'], excl_eps=b['eps'], excl_mean=b['mean'], excl_hit=b['hit'], excl_ep_mean=b['ep_mean'],
                           excl_keep=(b['eps'] / wst['eps'] if wst['eps'] else None)))
        # --- 3. simulation: v16 (calibration), every candidate standalone and as added condition
        print(f'== {year}: screen 3', flush=True)
        for name, (grp, m) in F.items():
            if grp == 'ref': continue
            for mode, mm in (('standalone', m), ('v16+require', AND(win, m)), ('v16+exclude', AND(win, NOT(m)))):
                if grp == 'window' and mode != 'standalone': continue
                if grp == 'window' and name != 'v16 window (any)' and mode == 'standalone': continue
                t16 = simulate(mm, cols, n, EXIT_V16); tl = simulate(mm, cols, n, EXIT_LABEL)
                a = sim_summary(t16); b = sim_summary(tl)
                s3.append(dict(trigger=name, mode=mode, group=grp, year=year, trades=a['trades'], pnl=a['pnl'], win=a['win'], pf=a['pf'], mean_pct=a['mean_pct'],
                               lab_trades=b['trades'], lab_pnl=b['pnl'], lab_pf=b['pf']))
        # --- 5. what the bearish candle patterns catch
        print(f'== {year}: screen 5', flush=True)
        for ts in ('1m', '5m'):
            for p in CS_PATTERNS:
                m = F[f'cs_{p}_{ts} bear'][1]
                bear_ok = AND(m, [le(x, -0.4) for x in S5]); bull = AND(m, [gt(x, 0) for x in S5])
                a = stats(bear_ok, opp, keys, top_thr, base_mean); b = stats(bull, opp, keys, top_thr, base_mean)
                c = stats(AND(m, [gt(x, 0) for x in S1M]), opp, keys, top_thr, base_mean)
                w = stats(AND(m, win), opp, keys, top_thr, base_mean)
                s5.append(dict(pattern=f'cs_{p}_{ts}', year=year, s5le40_bars=a['bars'], s5le40_eps=a['eps'], s5le40_mean=a['mean'], s5le40_hit=a['hit'],
                               s5gt0_bars=b['bars'], s5gt0_eps=b['eps'], s5gt0_mean=b['mean'], s5gt0_hit=b['hit'],
                               s1gt0_eps=c['eps'], s1gt0_mean=c['mean'], win_eps=w['eps'], win_mean=w['mean']))
        del F, cols
    # ---- write per-year csvs
    def dump(rows, name):
        if not rows: return
        with open(os.path.join(SCREEN, name), 'w', newline='') as f:
            w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
    dump(s1, 's1_feature_alone.csv'); dump(s2, 's2_conditional.csv'); dump(s3, 's3_sim.csv'); dump(s5, 's5_patterns.csv')
    json.dump({'base': base, 'groups': feat_groups}, open(os.path.join(SCREEN, 'meta.json'), 'w'), indent=1, default=str)
    print('wrote s1/s2/s3/s5; run summarize.py for the rankings and pairs', flush=True)

if __name__ == '__main__':
    main()
