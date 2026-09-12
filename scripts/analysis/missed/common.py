"""shared helpers for the 'missed setups' analysis. python stdlib only.

data sources (relative to the repo root):
  data/bars/<TICKER>.csv          raw RTH 1-min bars, epoch seconds
  data/v15c_<year>_ticks.csv      per-bar engine diagnostics
  data/v15c_<year>_trades.csv     actual trades

engine mechanics reproduced here (verified against the tick/trade files and crates/ source):
  * entry signalled on bar i -> filled at bar i+1 OPEN; exit signalled on bar j -> filled at j+1 OPEN
  * backtest cost model is direction-blind: entry fill = open + 3bps*open + 0.005, exit fill =
    open - 3bps*open - 0.005 (favourable to a short!). 'adverse' mode flips the signs.
  * marks (unrealized, stop checks) use the bar CLOSE
  * size = floor(3600 / entry_fill) shares (36% of $10,000, no compounding)
  * entry_cooldown blocks exactly the exit-fill bar; 'position' is short from the entry-fill bar
  * reject gates (either fires -> no entry that bar):
      long  noise: s1m >= s5m+0.15 and s1m >= s1h+0.15 and s5m <= +0.35
      short noise: s1m <= s5m-0.15 and s1m <= s1h-0.15 and s5m >= -0.35
  * windows: thrust = comp<=cmax, s5m<=t_s5, s5m<=s1m-lag, s5m<=s1h-lag, s1h<=t_1h
             core   = comp<=cmax, s5m<=k_s5, s1h<=k_1h
"""
import csv, os
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

ROOT = '/home/dylmet/Projects/galactic_trading_firm'
OUT = '/tmp/claude-1000/-home-dylmet-Projects-galactic-trading-firm/48685858-bad9-4000-a57f-b038fffcc726/scratchpad/missed'
ET = ZoneInfo('America/New_York')
TICKERS = ['AMZN', 'AAPL', 'NVDA', 'MSFT']
YEARS = [2022, 2023, 2024, 2025, 2026]
CAPITAL_PER_POS = 3600.0          # 36% of $10,000
SLIP_BPS = 3.0
HALF_SPREAD = 0.005
ENTRY_LAST_HM = 11 * 60 + 29      # last bar on which an entry may be signalled (11:29)
FORCE_EXIT_HM = 11 * 60 + 55      # session close signal bar (11:55) -> fill 11:56

BASE = dict(cmax=-0.35, t_s5=-0.50, lag=0.10, t_1h=0.0, k_s5=-0.40, k_1h=-0.30, reject=True)

_hm_cache = {}
def hm_et(ts):
    """minutes since midnight ET for an epoch-second timestamp (cached per day-offset)."""
    day = ts // 86400
    off = _hm_cache.get(day)
    if off is None:
        d = datetime.fromtimestamp(ts, timezone.utc).astimezone(ET)
        off = d.utcoffset().total_seconds()
        _hm_cache[day] = off
    local = ts + off
    return int((local % 86400) // 60)

def et_date(ts):
    return datetime.fromtimestamp(ts, timezone.utc).astimezone(ET).date().isoformat()

def load_bars(ticker):
    """returns (list of (ts,o,h,l,c), dict ts->index)."""
    rows = []
    with open(os.path.join(ROOT, 'data', 'bars', f'{ticker}.csv')) as f:
        r = csv.reader(f); next(r)
        for ts, o, h, l, c, v in r:
            rows.append((int(ts), float(o), float(h), float(l), float(c)))
    rows.sort()
    idx = {b[0]: i for i, b in enumerate(rows)}
    return rows, idx

def fnum(s):
    return float(s) if s not in ('', None) else None

def stream_tick_days(year):
    """yield (date, ticker, rows) for each ticker-day in file order. each row is a dict with
    ts (epoch), hm (ET minutes), comp/s1m/s5m/s1h (float or None), position, event, blocked_by,
    near_miss, entry_reason, o/h/l/c."""
    path = os.path.join(ROOT, 'data', f'v15c_{year}_ticks.csv')
    cur_key = None; buf = []
    with open(path) as f:
        r = csv.reader(f); hdr = next(r)
        col = {n: i for i, n in enumerate(hdr)}
        ci = [col[k] for k in ('date','ticker','ts','open','high','low','close','composite','s1m','s5m','s1h','position','event','blocked_by','near_miss','entry_reason')]
        for rec in r:
            date, ticker, ts, o, h, l, c, comp, s1m, s5m, s1h, pos, ev, bb, nm, er = (rec[i] for i in ci)
            key = (date, ticker)
            if key != cur_key:
                if buf: yield cur_key[0], cur_key[1], buf
                cur_key = key; buf = []
            tse = int(datetime.fromisoformat(ts).timestamp())
            buf.append(dict(ts=tse, hm=hm_et(tse), o=float(o), h=float(h), l=float(l), c=float(c),
                            comp=fnum(comp), s1m=fnum(s1m), s5m=fnum(s5m), s1h=fnum(s1h),
                            position=pos, event=ev, blocked_by=bb, near_miss=nm, entry_reason=er))
        if buf: yield cur_key[0], cur_key[1], buf

def load_trades(year):
    out = []
    with open(os.path.join(ROOT, 'data', f'v15c_{year}_trades.csv')) as f:
        for row in csv.DictReader(f):
            if row['row_type'] != 'trade': continue
            row['entry_ts'] = int(datetime.fromisoformat(row['entry_time']).timestamp())
            row['exit_ts'] = int(datetime.fromisoformat(row['exit_time']).timestamp())
            row['pnl'] = float(row['pnl']); row['size'] = float(row['size'])
            out.append(row)
    return out

# ---------------- window / gate evaluation ----------------
def gate_name(bb):
    if not bb: return ''
    if bb.startswith('reject_gate:1m noise filter short'): return 'reject_noise_short'
    if bb.startswith('reject_gate:1m noise filter'): return 'reject_noise_long'
    return bb

def reject_fires(s1m, s5m, s1h):
    long_noise = s1m >= s5m + 0.15 and s1m >= s1h + 0.15 and s5m <= 0.35
    short_noise = s1m <= s5m - 0.15 and s1m <= s1h - 0.15 and s5m >= -0.35
    return long_noise or short_noise

def cond_flags(comp, s1m, s5m, s1h, p=BASE):
    """returns dict of FAIL flags for each condition of each window."""
    return dict(
        c_fail=comp > p['cmax'],
        t_s5_fail=s5m > p['t_s5'],
        t_lag_fail=not (s5m <= s1m - p['lag'] and s5m <= s1h - p['lag']),
        t_1h_fail=s1h > p['t_1h'],
        k_s5_fail=s5m > p['k_s5'],
        k_1h_fail=s1h > p['k_1h'],
    )

def window_pass(comp, s1m, s5m, s1h, p=BASE):
    """returns 'thrust', 'core' or None (thrust has priority 11 < core 21, matching the engine)."""
    if comp is None or s1m is None or s5m is None or s1h is None: return None
    f = cond_flags(comp, s1m, s5m, s1h, p)
    if not (f['c_fail'] or f['t_s5_fail'] or f['t_lag_fail'] or f['t_1h_fail']): return 'thrust'
    if not (f['c_fail'] or f['k_s5_fail'] or f['k_1h_fail']): return 'core'
    return None

# ---------------- exit simulator ----------------
def fills(open_px, mode):
    """(entry_fill, exit_fill) multipliers/offsets for a short. mode 'engine' reproduces the
    backtest's direction-blind cost model (favourable to shorts); 'adverse' is the honest one."""
    slip = open_px * SLIP_BPS / 10_000.0 + HALF_SPREAD
    if mode == 'engine':
        return open_px + slip, open_px - slip
    return open_px - slip, open_px + slip

def simulate_short(day, i, comps=None, mode='engine', stop_pct=0.025, max_hold=90, loss_red=15,
                   profit_ext=30, score_exit=None, breakeven=None, force_hm=FORCE_EXIT_HM):
    """hypothetical short signalled on bar index i of `day` (list of (ts,o,h,l,c)); filled at
    bar i+1 open. exits evaluated on every bar j>=i+1 at the close, filled at j+1 open.
    comps: optional list of composite per bar (same indexing as day) for the score exit.
    returns dict or None if no next bar."""
    n = len(day)
    if i + 1 >= n: return None
    e_idx = i + 1
    e_ts, e_open = day[e_idx][0], day[e_idx][1]
    entry_fill, _ = fills(e_open, mode)
    be_armed = False
    reason = None; x_idx = None
    for j in range(e_idx, n):
        ts, o, h, l, c = day[j]
        hold = (ts - e_ts) // 60
        unreal = (entry_fill - c) / entry_fill
        if c >= entry_fill * (1 + stop_pct):
            reason = 'HardStop'
        elif breakeven is not None and be_armed and c >= entry_fill:
            reason = 'BreakevenStop'
        elif score_exit is not None and comps is not None and comps[j] is not None and comps[j] >= score_exit:
            reason = 'ScoreExit'
        else:
            lim = (max_hold + profit_ext) if unreal > 0 else ((max_hold - loss_red) if unreal < 0 else max_hold)
            if hold >= lim:
                reason = 'MaxHoldTimeout'
            elif hm_et(ts) >= force_hm:
                reason = 'SessionClose'
        if breakeven is not None and unreal >= breakeven: be_armed = True
        if reason:
            x_idx = j + 1
            break
    if reason is None or x_idx >= n:
        # no next bar to fill on (should not happen before 11:56); fall back to last close
        x_idx = n - 1; reason = reason or 'EndOfData'
        _, exit_fill = fills(day[x_idx][4], mode)
    else:
        _, exit_fill = fills(day[x_idx][1], mode)
    size = int(CAPITAL_PER_POS // entry_fill)
    return dict(e_idx=e_idx, x_idx=x_idx, entry_fill=entry_fill, exit_fill=exit_fill, reason=reason,
                pnl_pct=(entry_fill - exit_fill) / entry_fill, size=size,
                pnl=size * (entry_fill - exit_fill), hold_min=(day[x_idx][0] - e_ts) // 60)

def path_stats(day, i, horizon=90):
    """simple forward stats for a short entered at bar i+1 open (no costs):
    ret_1155 = return to the 11:55 bar close; mfe/mae over `horizon` bars."""
    n = len(day)
    if i + 1 >= n: return None
    e = day[i + 1][1]
    lo = min(b[3] for b in day[i + 1:i + 1 + horizon])
    hi = max(b[2] for b in day[i + 1:i + 1 + horizon])
    c1155 = None
    for b in day[i + 1:]:
        if hm_et(b[0]) >= FORCE_EXIT_HM:
            c1155 = b[4]; break
    if c1155 is None: c1155 = day[-1][4]
    return dict(ret_1155=(e - c1155) / e, mfe=(e - lo) / e, mae=(hi - e) / e)
