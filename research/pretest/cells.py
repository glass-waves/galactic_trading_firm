"""pretest cell registry. a cell = one anomaly x instrument cell of research/edge_matrix.md.

fields: id, anomaly, instruments (home first; "A+B" = pooled into one book), variants
(name, rule, optional grid -- first point = the paper's parameters --, optional flag for a version
the intraday-only engine cannot trade), kill_text (pre-registered, copied from the matrix),
verdict_fn(row) -> (pass, why) per variant x instrument, verdict(result) -> (status, text) for the cell.
statuses: pass-pretest (a tradable version clears the bar on one of the cell's instruments), needs-product
(only an overnight / multi-day version clears it), dead."""
import csv
import datetime as dt
import os
import re
import warnings

import numpy as np

from harness import ET, ROOT, YEARS, Order, bars, execute, kill, run_rule

FOUR = "AAPL+AMZN+MSFT+NVDA"
FOUR_NAMES = set(FOUR.split("+"))
# the ~30 other large caps with 1m bars cached (data/bars_iex/*.csv), for the instrument-neighbourhood
# scan on single-stock cells. excludes the index/sector ETFs also in that directory.
OTHER_CAPS = sorted({"AAPL", "ABBV", "ADBE", "AMAT", "AMD", "AMZN", "AVGO", "CAT", "COIN", "COST", "CRM",
                      "CVX", "GS", "HD", "JPM", "LLY", "MA", "META", "MRK", "MSFT", "MU", "NFLX", "NVDA",
                      "ORCL", "PG", "PLTR", "QCOM", "SHOP", "TSLA", "TXN", "UBER", "UNH", "V", "XOM"} - FOUR_NAMES)
ALL34 = "+".join(sorted(FOUR_NAMES | set(OTHER_CAPS)))
INDEX3 = ["SPY", "QQQ", "IWM"]
warnings.filterwarnings("ignore", "Mean of empty slice")   # half days: no half-hour has all names
FOMC = {dt.date.fromisoformat(d) for d in re.findall(r'"(\d{4}-\d{2}-\d{2})"', open(
    f"{ROOT}/crates/indicators/src/custom/event_calendar.rs").read().split("FOMC_DECISION_DAYS")[1].split("];")[0])}


def standard_verdict(years_pos, pf):
    """the paper's parameters (first grid point) must clear the bar; a different grid point may pass
    instead only if leave-one-year-out selection also clears it (P&L > 0, >= years_pos years)."""
    def f(row):
        ok, why = kill(row["primary"], years_pos, pf)
        lo = row.get("loyo")
        if ok or not lo:
            return ok, why
        tail = f"; LOYO {lo['pnl']:+.0f} ({lo['years_pos']}/5)"
        for lab, m in row["grid"].items():
            if kill(m, years_pos, pf)[0] and lo["pnl"] > 0 and lo["years_pos"] >= years_pos:
                return True, f"grid {lab}: {kill(m, years_pos, pf)[1]}{tail}"
        return False, why + tail
    return f


def cell_verdict(res):
    home = res["variants"][0]["instrument"]
    ok = [v for v in res["variants"] if v["pass"]]
    if any(v["tradable"] for v in ok):     # instruments are the matrix cell's pre-registered set
        v = next(v for v in ok if v["tradable"])
        note = "" if v["instrument"] == home else f" (home {home} fails)"
        return "pass-pretest", f"{v['variant']} on {v['instrument']}: {v['why']}{note} -> engine study worth it"
    if any(not v["tradable"] for v in ok):
        v = next(v for v in ok if not v["tradable"])
        return "needs-product", f"only {v['variant']} ({v['flag']}) on {v['instrument']} clears the bar: {v['why']}"
    best = max((v for v in res["variants"] if v["tradable"] and v["instrument"] == home), key=lambda v: v["primary"]["pnl"])
    extra = f" (other instruments/variants pass: {', '.join(v['variant'] + '@' + v['instrument'] for v in ok)})" if ok else ""
    return "dead", f"best tradable on {home} = {best['variant']}: {best['primary'].get('bps', 0):+.1f} bps/trade, {best['why']}{extra}"

# ------------------------------------------------------------------ 1. pre-FOMC drift (#9)
def nxt(b, i):
    return b.dates[i + 1] if i + 1 < len(b.dates) else None


def fomc_paper(b, i, p):          # Lucca-Moench: 14:00 the day before -> 14:00 (pre-release) on the day
    return [Order(i, 270, 1, i + 1, 270)] if nxt(b, i) in FOMC else []


def fomc_day_am(b, i, p):         # tradable: long 09:30 -> 14:00 on the decision day
    return [Order(i, 0, 1, i, 270)] if b.dates[i] in FOMC else []


def fomc_prior_session(b, i, p):  # tradable: long the full session before the decision day
    return [Order(i, 0, 1, i, 390)] if nxt(b, i) in FOMC else []

# ------------------------------------------------------------------ 2. turn of month (#17)
def tom_day(b, i):
    """-1 = last session of the month, else n = n-th session of the month (calendar-known)."""
    if i + 1 < len(b.dates) and b.dates[i + 1].month != b.dates[i].month:
        return -1
    k = 1
    while i - k >= 0 and b.dates[i - k].month == b.dates[i].month:
        k += 1
    return k                      # the cache starts on 2022-01-03, the first session of a month


def tom_paper(b, i, p):           # McConnell-Xu: close of day -2 -> close of day +3 (4 sessions held)
    return [Order(i, 390, 1, i + 4, 390)] if i + 4 < len(b.dates) and tom_day(b, i + 1) == -1 else []


def tom_intraday(b, i, p):        # long open -> close on day -1 and days +1..+after
    d = tom_day(b, i)
    return [Order(i, 0, 1, i, 390)] if d == -1 or 1 <= d <= p["after"] else []

def calendar_control(spec):
    """is a calendar rule better than the same trade on random sessions? spec rows:
    (label, sym, rule, params, control(b, i) -> Order). p = share of 5000 random draws (same n,
    without replacement, from every session's control trade) whose mean net bps >= the rule's."""
    def f():
        out, rng = {}, np.random.default_rng(0)
        for label, sym, rule, p, ctrl in spec:
            b = bars(sym)
            alls = [execute(b, o) for i in range(len(b.dates)) for o in [ctrl(b, i)] if o]
            allbps = np.array([t["bps"] for t in alls if t])
            obs = np.array([t["bps"] for t in run_rule(rule, [sym], p)[0]])
            sims = np.array([rng.choice(allbps, len(obs), replace=False).mean() for _ in range(5000)])
            out[label] = {"all_sessions_bps": round(float(allbps.mean()), 2), "rule_bps": round(float(obs.mean()), 2),
                          "p_random": round(float((sims >= obs.mean()).mean()), 4)}
        return out
    return f


def oc(t_out, days=0, t_in=0):
    return lambda b, i: Order(i, t_in, 1, i + days, t_out) if i + days < len(b.dates) else None

# ------------------------------------------------------------------ 3. HKS half-hour periodicity (#6)
_HH = {}
def halfhours(b):
    """R[i, h] = return of half-hour h (09:30+30h) traded open->close; NaN if a half has no bars."""
    if b.sym not in _HH:
        R = np.full((len(b.dates), 13), np.nan)
        for i in range(len(b.dates)):
            for h in range(13):
                if b.has[i, 30 * h:30 * h + 30].any():
                    R[i, h] = b.exit_px(i, 30 * h + 30) / b.entry_px(i, 30 * h)[0] - 1
        _HH[b.sym] = R
    return _HH[b.sym]


def hks_signal(R, i, h, look=40):
    past = R[max(0, i - look):i, h]
    past = past[~np.isnan(past)]
    if i < look or len(past) < 30:
        return 0.0, 0.0
    m, sd = past.mean(), past.std(ddof=1)
    return m, (m / (sd / np.sqrt(len(past))) if sd > 0 else 0.0)


def hks_trade(b, i, p):           # trade each half-hour in the sign of its 40-session same-clock mean
    R, out = halfhours(b), []
    for h in p.get("hours", range(13)):
        m, t = hks_signal(R, i, h)
        if m != 0 and abs(t) >= p["t"] and not np.isnan(R[i, h]):
            out.append(Order(i, 30 * h, int(np.sign(m)), i, 30 * h + 30))
    return out


def hks_open_close(b, i, p):
    return hks_trade(b, i, dict(p, hours=(0, 12)))


def hks_extra():
    """predictive statistics: same-clock lag-k correlation (pooled, z-scored per name x half-hour),
    Fama-MacBeth cross-sectional slope over the 4 names, other-clock placebo, the 40-day signal's IC,
    and the filter test on the live v18 book's trades (does agreeing with the HKS sign help?)."""
    Rs = {s: halfhours(bars(s)) for s in FOUR.split("+")}
    n = min(len(R) for R in Rs.values())
    assert all(bars(s).dates[:n] == bars("AAPL").dates[:n] for s in Rs)
    Z = {s: (R[:n] - np.nanmean(R[:n], 0)) / np.nanstd(R[:n], 0) for s, R in Rs.items()}
    out = {"lags": {}}
    for k in range(1, 6):
        x = np.concatenate([Z[s][k:].ravel() for s in Z]); y = np.concatenate([Z[s][:-k].ravel() for s in Z])
        ok = ~np.isnan(x) & ~np.isnan(y)
        c = float(np.corrcoef(x[ok], y[ok])[0, 1])
        xp = np.concatenate([Z[s][k:, 1:].ravel() for s in Z]); yp = np.concatenate([Z[s][:-k, :-1].ravel() for s in Z])
        okp = ~np.isnan(xp) & ~np.isnan(yp)
        A = np.stack([Rs[s][:n] for s in Z])                       # names x days x halves
        cur, lag = A[:, k:] - np.nanmean(A[:, k:], 0), A[:, :-k] - np.nanmean(A[:, :-k], 0)
        num, den = np.nansum(cur * lag, 0), np.nansum(lag * lag, 0)
        g = (num / np.where(den > 0, den, np.nan)).ravel(); g = g[~np.isnan(g)]
        out["lags"][k] = {"corr": round(c, 4), "t": round(c * np.sqrt(ok.sum()), 2), "r2": round(c * c, 6),
                          "placebo_corr": round(float(np.corrcoef(xp[okp], yp[okp])[0, 1]), 4),
                          "fm_gamma": round(float(g.mean()), 4), "fm_t": round(float(g.mean() / g.std() * np.sqrt(len(g))), 2)}
    sig, ret = [], []
    for s, R in Rs.items():
        for i in range(40, len(R)):
            for h in range(13):
                m, _ = hks_signal(R, i, h)
                if m != 0 and not np.isnan(R[i, h]):
                    sig.append(m); ret.append(R[i, h])
    sig, ret = np.array(sig), np.array(ret)
    out["ic40"] = round(float(np.corrcoef(sig, ret)[0, 1]), 4)
    out["gross_bps_signed"] = round(float(np.mean(np.sign(sig) * ret) * 1e4), 3)
    # filter test on the live v18 book (data/iex_v18_<year>_trades.csv, engine replay at 36 % sizing)
    agree, disagree = [], []
    for y in YEARS:
        for r in csv.DictReader(open(f"{ROOT}/data/iex_v18_{y}_trades.csv")):
            if r["row_type"] != "trade" or r["ticker"] not in Rs:
                continue
            t = dt.datetime.fromisoformat(r["entry_time"].replace("Z", "+00:00")).astimezone(ET)
            b = bars(r["ticker"]); i = b.index.get(t.date())
            h = (t.hour * 60 + t.minute - 570) // 30
            if i is None or not 0 <= h < 13:
                continue
            m, _ = hks_signal(Rs[r["ticker"]], i, h)
            side = 1 if r["direction"].lower().startswith("l") else -1
            (agree if m * side > 0 else disagree).append((y, float(r["pnl"])))
    def s(v):
        p = [x for _, x in v]
        return {"n": len(p), "pnl": round(sum(p), 1), "mean": round(float(np.mean(p)) if p else 0, 2),
                "per_year": {y: round(sum(x for yy, x in v if yy == y), 1) for y in YEARS}}
    out["v18_filter"] = {"agree": s(agree), "disagree": s(disagree)}
    return out

# ------------------------------------------------------------------ 4. Zarattini-Aziz ORB (#1)
def orb(b, i, p):
    """first n-minute candle up -> long at the next bar open, stop at its low; down -> short, stop at
    its high; doji -> no trade. target = 10R (paper), else liquidate at the close. one trade/day."""
    n = p["or"]
    if not b.has[i, :n].all():
        return []
    o, c, hi, lo = b.o[i, 0], b.c[i, n - 1], b.h[i, :n].max(), b.l[i, :n].min()
    if c == o:
        return []
    side, stop = (1, lo) if c > o else (-1, hi)
    e = b.entry_px(i, n)[0]
    tgt = e + side * p["R"] * abs(e - stop) if p.get("R") else None
    return [Order(i, n, side, i, 390, stop, tgt)]

# ------------------------------------------------------------------ 5. earnings-day gap reversal (#15)
def reaction_days(sym):
    """8-K item 2.02 filing dates (both repo lists, where they exist -- 4 of the 34-stock scan
    universe have no earnings file, e.g. COIN/PLTR/SHOP/UBER, and are treated as having no
    exclusion, same as the index ETFs). all four home names report after the close, so the
    reaction session is the next session. NVDA 2022-08-08 is a pre-market preannouncement, not the
    scheduled report (that is 2022-08-24) -- dropped."""
    b, out = bars(sym), set()
    files = [f for f in (f"{ROOT}/research/swing/earnings/{sym}.txt", f"{ROOT}/research/entries/data/earnings_{sym}.txt")
             if os.path.exists(f)]
    for d in {dt.date.fromisoformat(x) for f in files for x in open(f).read().split()} - {dt.date(2022, 8, 8)}:
        i = next((k for k, x in enumerate(b.dates) if x > d), None)
        if i is not None and d >= b.dates[0]:
            out.add(b.dates[i])
    return out


_EARN = {}
def earn_gap(b, i, p):
    if b.sym not in _EARN:
        _EARN[b.sym] = reaction_days(b.sym)
    if i < 1 or b.dates[i] not in _EARN[b.sym]:
        return []
    gap = b.session_open(i) / b.session_close(i - 1) - 1
    if gap == 0:
        return []
    return [Order(i, 0, int(np.sign(gap)) * (-1 if p["mode"] == "fade" else 1), i, p["exit"])]

# ------------------------------------------------------------------ 6. size-dependent gap fade/fill (#16)
def gap_fade_fill(b, i, p):
    """|gap| < thresh -> fade (small/no-news gaps mean-revert); |gap| >= thresh -> continue (large,
    news-driven gaps keep going -- the QQQ >2% continuation evidence in the catalog). excludes
    earnings reaction days on the four mega-caps (no earnings calendar for SPY/QQQ, so no exclusion
    needed there)."""
    if i < 1:
        return []
    if b.sym in FOUR_NAMES:
        if b.sym not in _EARN:
            _EARN[b.sym] = reaction_days(b.sym)
        if b.dates[i] in _EARN[b.sym]:
            return []
    gap = b.session_open(i) / b.session_close(i - 1) - 1
    if not np.isfinite(gap) or gap == 0:
        return []
    side = np.sign(gap) if abs(gap) >= p["thresh"] else -np.sign(gap)
    return [Order(i, 0, int(side), i, p["exit"])]

# ------------------------------------------------------------------ 7. overnight-return decile lean (#7)
_ON = {}
def overnight_rets(b):
    """close(i-1) -> open(i), one value per session (NaN for the first session or a missing bar)."""
    if b.sym not in _ON:
        r = np.full(len(b.dates), np.nan)
        for i in range(1, len(b.dates)):
            o, c = b.session_open(i), b.session_close(i - 1)
            if np.isfinite(o) and np.isfinite(c):
                r[i] = o / c - 1
        _ON[b.sym] = r
    return _ON[b.sym]


def overnight_decile_lean(b, i, p):
    """Lou-Polk-Skouras cross-predictability, signal-only: yesterday's overnight return's percentile
    rank in a trailing window sets today's open->close lean. an overnight-winner day (top decile)
    predicts an intraday loser next -> short lean; an overnight-loser day (bottom decile) -> long
    lean. no position is carried overnight -- the signal is read off yesterday's already-closed
    overnight return, today's trade is open->close only."""
    R = overnight_rets(b)
    if i < 2 or np.isnan(R[i - 1]):
        return []
    w = p.get("window", 60)
    hist = R[max(1, i - 1 - w):i - 1]
    hist = hist[~np.isnan(hist)]
    if len(hist) < 20:
        return []
    rank = (hist < R[i - 1]).mean()
    dec = p.get("decile", 0.1)
    if rank >= 1 - dec:
        side = -1
    elif rank <= dec:
        side = 1
    else:
        return []
    return [Order(i, 0, side, i, 390)]

# ------------------------------------------------------------------ 8. hedging-demand last-30-min momentum (#4)
_RR = {}
def session_range_proxy(b, i, t0, t1):
    """realized-range proxy for dealer hedging flow we don't have the OI data for: (high-low)/price
    over [t0, t1) minutes since the open. a bigger realized range stands in for a bigger hedging
    flow that day (Baltussen-Da-Lammers-Martens use order/hedging flow directly; we only have bars)."""
    if not b.has[i, t0:t1].any():
        return np.nan
    h = np.where(b.has[i, t0:t1], b.h[i, t0:t1], -np.inf)
    l = np.where(b.has[i, t0:t1], b.l[i, t0:t1], np.inf)
    p0 = b.entry_px(i, t0)[0]
    return (np.nanmax(h) - np.nanmin(l)) / p0 if np.isfinite(p0) and p0 else np.nan


def range_hist(b, t0, t1):
    key = (b.sym, t0, t1)
    if key not in _RR:
        _RR[key] = np.array([session_range_proxy(b, i, t0, t1) for i in range(len(b.dates))])
    return _RR[key]


def hedging_momentum(b, i, p):
    """last-N-min momentum (Baltussen et al., paper N=30): trade the sign of the 09:30->(390-hold)
    return in the last `hold` minutes, gated on the day's realized range so far being above its
    trailing-window percentile (the hedging-demand proxy). gate=None trades every day -- the
    plain-momentum comparison (anomaly #3, already dead) that isolates what the gate adds."""
    hold = p.get("hold", 30)
    t = 390 - hold
    if not b.full(i) or not b.has[i, :t].any():
        return []
    mom = b.exit_px(i, t) / b.session_open(i) - 1
    if not np.isfinite(mom) or mom == 0:
        return []
    if p.get("gate") is not None:
        RR, w = range_hist(b, 0, t), p.get("window", 60)
        if i < w or np.isnan(RR[i]):
            return []
        hist = RR[max(0, i - w):i]
        hist = hist[~np.isnan(hist)]
        if len(hist) < 20 or RR[i] < np.nanpercentile(hist, p["gate"]):
            return []
    return [Order(i, t, int(np.sign(mom)), i, 390)]

# ------------------------------------------------------------- 9/10. FOMC / monthly-OpEx range compression (#10, #19)
# time-of-day neighbourhood around the task's 10:30-14:00 window: 1h earlier start, 30min later end.
WINDOWS = {"10:30-14:00": (60, 270), "10:00-14:00": (30, 270), "10:30-14:30": (60, 300)}


def third_friday(year, month):
    d = dt.date(year, month, 1)
    d += dt.timedelta(days=(4 - d.weekday()) % 7)
    return d + dt.timedelta(days=14)


OPEX_DAYS = {third_friday(y, m) for y in range(2021, 2028) for m in range(1, 13)}


def no_trade(b, i, p):
    return []


def compression_stats(is_event, syms=INDEX3, windows=WINDOWS):
    """Welch-style comparison of the realized-range proxy on event days vs all other full sessions
    (half days excluded from both groups) across every (instrument x time-of-day window) cell."""
    def f():
        out = {}
        for sym in syms:
            b = bars(sym)
            out[sym] = {}
            for wlabel, (t0, t1) in windows.items():
                ev, ot = [], []
                for i in range(len(b.dates)):
                    if not b.full(i):
                        continue
                    rr = session_range_proxy(b, i, t0, t1)
                    if np.isnan(rr):
                        continue
                    (ev if is_event(b.dates[i]) else ot).append(rr)
                ev, ot = np.array(ev), np.array(ot)
                if len(ev) < 5 or len(ot) < 5:
                    out[sym][wlabel] = {"n_event": len(ev), "n_other": len(ot), "z": 0.0, "compression_pct": 0.0}
                    continue
                me, mo = float(ev.mean()), float(ot.mean())
                se = float(np.sqrt(ev.var(ddof=1) / len(ev) + ot.var(ddof=1) / len(ot)))
                out[sym][wlabel] = {"n_event": len(ev), "n_other": len(ot), "event_range_pct": round(me * 100, 3),
                                     "other_range_pct": round(mo * 100, 3), "compression_pct": round((mo - me) / mo * 100, 1),
                                     "z": round((me - mo) / se, 2) if se > 0 else 0.0}
        return out
    return f


def regime_verdict(z_bar, pct_bar, action):
    """pre-registered: a regime 'passes' (worth gating an existing window on) only if some
    (instrument x time-of-day window) cell's realized range is >= pct_bar % tighter than non-event
    sessions at |z| >= z_bar; otherwise it is dead -- discarded as a filter candidate, not a P&L
    verdict. scans every instrument x window cell and reports the number scanned, the best cell,
    and whether any other cell is within half the bar (a 'near miss' worth flagging)."""
    def f(res):
        ex = res["extra"]
        cells = [(s, w, d) for s, ws in ex.items() for w, d in ws.items()]
        n = len(cells)
        hit = [(s, w, d) for s, w, d in cells if d.get("n_event", 0) >= 5 and d["z"] <= -z_bar and d["compression_pct"] >= pct_bar]
        best = max(cells, key=lambda t: abs(t[2].get("z", 0)))
        near = [(s, w, d) for s, w, d in cells if (s, w, d) not in hit and d.get("n_event", 0) >= 5
                and d["z"] <= -z_bar / 2 and d["compression_pct"] >= pct_bar / 2]
        near_txt = f"; near-miss: {near[0][0]} {near[0][1]} z={near[0][2]['z']}, {near[0][2]['compression_pct']}%" if near and not hit else ""
        if hit:
            s, w, d = hit[0]
            return "pass-pretest", (f"{n} cells scanned ({len(ex)} instruments x {len(WINDOWS)} windows); "
                                     f"{s} {w} compressed {d['compression_pct']}% vs non-event sessions "
                                     f"(z={d['z']}, n={d['n_event']}) -> measurable, worth gating {action}")
        s, w, d = best
        return "dead", (f"{n} cells scanned ({len(ex)} instruments x {len(WINDOWS)} windows); "
                         f"no cell clears |z|>={z_bar} and {pct_bar}% compression "
                         f"(best {s} {w}: z={d.get('z')}, {d.get('compression_pct')}%){near_txt} -> discard as a regime gate")
    return f

# ------------------------------------------------------------------ registry
K45 = "net positive in >= 4/5 years and PF > 1.3 (3 bps + $0.005/sh per leg, 36 % of $10k)"
CELLS = [
    {"id": "c01_pre_fomc", "anomaly": "#9 pre-FOMC drift (Lucca-Moench 2015)", "instruments": ["SPY", "QQQ"],
     "kill_text": K45 + "; expected ~8 trades/yr",
     "variants": [
         {"name": "paper 14:00(d-1)->14:00(d)", "rule": fomc_paper, "flag": "overnight: needs a new product"},
         {"name": "FOMC day 09:30->14:00", "rule": fomc_day_am},
         {"name": "prior session open->close", "rule": fomc_prior_session}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict,
     "extra": calendar_control([(f"{v} {s}", s, r, {}, c) for s in ("SPY", "QQQ") for v, r, c in (
         ("paper", fomc_paper, oc(270, 1, 270)), ("FOMC day 09:30->14:00", fomc_day_am, oc(270)),
         ("prior session", fomc_prior_session, oc(390)))])},
    {"id": "c02_turn_of_month", "anomaly": "#17 turn-of-month (Ariel 1987; McConnell-Xu 2008)", "instruments": ["SPY", "QQQ"],
     "kill_text": K45 + "; expected ~12 events/yr",
     "variants": [
         {"name": "paper close(d-2)->close(d+3)", "rule": tom_paper, "flag": "multi-day hold: needs a new product"},
         {"name": "intraday open->close d-1,d+1..+N", "rule": tom_intraday, "grid": [{"after": 3}, {"after": 1}]}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict,
     "extra": calendar_control([(f"{v} {s}", s, r, p, c) for s in ("SPY", "QQQ") for v, r, p, c in (
         ("paper", tom_paper, {}, oc(390, 4, 390)), ("intraday after=3", tom_intraday, {"after": 3}, oc(390)))])},
    {"id": "c03_hks_halfhour", "anomaly": "#6 HKS same-clock half-hour periodicity (Heston-Korajczyk-Sadka 2010)",
     "instruments": [FOUR],
     "kill_text": "trade version net bps/half-hour must clear 3 bps + $0.005/leg: " + K45 +
                  "; as a filter: the agree-subset of the live v18 trades must beat the disagree-subset",
     "variants": [
         {"name": "all 13 half-hours, sign of 40d same-clock mean", "rule": hks_trade, "grid": [{"t": 0}, {"t": 1}, {"t": 2}]},
         {"name": "first+last half-hour only", "rule": hks_open_close, "grid": [{"t": 0}, {"t": 1}, {"t": 2}]}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict, "extra": hks_extra},
    {"id": "c04_orb5_qqq", "anomaly": "#1 Zarattini-Aziz 5-min opening-range breakout", "instruments": ["QQQ", "SPY"],
     "kill_text": K45 + " (same bar as the index_momentum study)",
     "variants": [{"name": "OR candle direction, stop other side, 10R target or close", "rule": orb,
                   "grid": [{"or": 5, "R": 10}, {"or": 15, "R": 10}]}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict},
    {"id": "c05_earnings_gap", "anomaly": "#15 earnings-day gap reversal (Ben-Rephael), intraday adaptation",
     "instruments": [FOUR],
     "kill_text": "net positive in >= 3/5 years and PF > 1.3 (thin sample; same bar research/earnings used)",
     "variants": [
         {"name": "fade the gap from the open", "rule": earn_gap, "grid": [{"mode": "fade", "exit": 120}, {"mode": "fade", "exit": 390}]},
         {"name": "continue the gap from the open", "rule": earn_gap, "grid": [{"mode": "cont", "exit": 120}, {"mode": "cont", "exit": 390}]}],
     "verdict_fn": standard_verdict(3, 1.3), "verdict": cell_verdict},
    {"id": "c06_gap_fade_fill", "anomaly": "#16 gap fade/fill, size-dependent (practitioner-tier sourcing)",
     "instruments": [FOUR, "SPY", "QQQ", "IWM", ALL34],
     "kill_text": K45 + " -- low prior given the practitioner-only sourcing, so the bar is strict; "
                  "expected ~30-50 trades/yr/name (any gap >= 1%); earnings reaction days excluded where we have "
                  "an earnings calendar. pre-registered neighbourhood: threshold in {1,1.5,2}%, exit in "
                  "{10:30,11:30,close}, instruments = home (4 names) + SPY/QQQ + IWM + the other ~30 large caps",
     "variants": [
         {"name": "fade/continue, exit 10:30", "rule": gap_fade_fill,
          "grid": [{"thresh": 0.02, "exit": 60}, {"thresh": 0.015, "exit": 60}, {"thresh": 0.01, "exit": 60}]},
         {"name": "fade/continue, exit 11:30", "rule": gap_fade_fill,
          "grid": [{"thresh": 0.02, "exit": 120}, {"thresh": 0.015, "exit": 120}, {"thresh": 0.01, "exit": 120}]},
         {"name": "fade/continue, exit close", "rule": gap_fade_fill,
          "grid": [{"thresh": 0.02, "exit": 390}, {"thresh": 0.015, "exit": 390}, {"thresh": 0.01, "exit": 390}]}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict},
    {"id": "c07_overnight_decile", "anomaly": "#7 overnight-return decile as an intraday lean (Lou-Polk-Skouras), signal-only",
     "instruments": [FOUR, ALL34],
     "kill_text": "next-session intraday alpha must be net positive in >= 4/5 years and PF > 1.3; discard if not "
                  "cleanly separable from PEAD (#14). expected signal frequency: daily. pre-registered "
                  "neighbourhood: trailing window in {20,60,120} sessions, cut in {5,10,20}% (decile/ventile/quintile), "
                  "instruments = home (4 names) + the other ~30 large caps",
     "variants": [
         {"name": "top/bottom 5% (ventile) lean, trailing window", "rule": overnight_decile_lean,
          "grid": [{"window": 60, "decile": 0.05}, {"window": 20, "decile": 0.05}, {"window": 120, "decile": 0.05}]},
         {"name": "top/bottom decile lean, trailing window", "rule": overnight_decile_lean,
          "grid": [{"window": 60, "decile": 0.1}, {"window": 20, "decile": 0.1}, {"window": 120, "decile": 0.1}]},
         {"name": "top/bottom quintile lean (wider), trailing window", "rule": overnight_decile_lean,
          "grid": [{"window": 60, "decile": 0.2}, {"window": 20, "decile": 0.2}, {"window": 120, "decile": 0.2}]}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict},
    {"id": "c08_hedging_momentum", "anomaly": "#4 hedging-demand last-30-min momentum (Baltussen-Da-Lammers-Martens), realized-range proxy",
     "instruments": INDEX3,
     "kill_text": K45 + "; discard without an options-data upgrade if the realized-range proxy does not beat "
                  "plain last-N-min momentum (#3, already dead). proxy: (high-low)/open over 09:30->(390-hold). "
                  "pre-registered neighbourhood: gate percentile in {50,60,70}, holding window in {20,30,45} min, "
                  "instruments SPY+QQQ+IWM",
     "variants": [
         {"name": "gated on realized-range proxy (trailing percentile), hold=30 (paper)", "rule": hedging_momentum,
          "grid": [{"gate": 50, "window": 60, "hold": 30}, {"gate": 60, "window": 60, "hold": 30}, {"gate": 70, "window": 60, "hold": 30}]},
         {"name": "ungated (plain momentum, #3 comparison), holding-window neighbourhood", "rule": hedging_momentum,
          "grid": [{"gate": None, "hold": 30}, {"gate": None, "hold": 20}, {"gate": None, "hold": 45}]}],
     "verdict_fn": standard_verdict(4, 1.3), "verdict": cell_verdict},
    {"id": "c09_fomc_compression", "anomaly": "#10 FOMC-day intraday range compression 10:30-14:00 (crude realized-range proxy, regime filter)",
     "instruments": INDEX3,
     "kill_text": "regime-filter candidate, not a standalone trade: realized range 10:30-14:00 ET must be "
                  "measurably tighter on FOMC days than other full sessions (>= 10% compression, Welch z <= -2) "
                  "to be worth gating an existing window on; otherwise discard. 38 FOMC days in-sample. "
                  "pre-registered neighbourhood: time-of-day window in {10:00-14:00, 10:30-14:00, 10:30-14:30}, "
                  "instruments SPY+QQQ+IWM",
     "variants": [{"name": "no-trade regime probe (diagnostics only, see extra)", "rule": no_trade,
                   "flag": "diagnostic only, not a trade"}],
     "verdict_fn": lambda row: (False, "diagnostic only -- see cell verdict / extra"),
     "verdict": regime_verdict(2.0, 10.0, "the FOMC-day windows in c01_pre_fomc"),
     "extra": compression_stats(lambda d: d in FOMC)},
    {"id": "c10_opex_compression", "anomaly": "#19 0DTE/gamma pinning via days-to-monthly-OpEx 10:30-14:00 (crude proxy, no options data, regime filter)",
     "instruments": INDEX3,
     "kill_text": "same regime-filter bar as c09_fomc_compression: realized range 10:30-14:00 ET on the monthly "
                  "OpEx session (3rd Friday) must be measurably tighter than other full sessions (>= 10% "
                  "compression, Welch z <= -2) to be worth building; otherwise discard without options data. "
                  "pre-registered neighbourhood: time-of-day window in {10:00-14:00, 10:30-14:00, 10:30-14:30}, "
                  "instruments SPY+QQQ+IWM",
     "variants": [{"name": "no-trade regime probe (diagnostics only, see extra)", "rule": no_trade,
                   "flag": "diagnostic only, not a trade"}],
     "verdict_fn": lambda row: (False, "diagnostic only -- see cell verdict / extra"),
     "verdict": regime_verdict(2.0, 10.0, "an existing SPY/QQQ window on OpEx day"),
     "extra": compression_stats(lambda d: d in OPEX_DAYS)},
]
