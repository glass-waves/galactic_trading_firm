"""pretest cell registry. a cell = one anomaly x instrument cell of research/edge_matrix.md.

fields: id, anomaly, instruments (home first; "A+B" = pooled into one book), variants
(name, rule, optional grid -- first point = the paper's parameters --, optional flag for a version
the intraday-only engine cannot trade), kill_text (pre-registered, copied from the matrix),
verdict_fn(row) -> (pass, why) per variant x instrument, verdict(result) -> (status, text) for the cell.
statuses: pass-pretest (a tradable version clears the bar on one of the cell's instruments), needs-product
(only an overnight / multi-day version clears it), dead."""
import csv
import datetime as dt
import re
import warnings

import numpy as np

from harness import ET, ROOT, YEARS, Order, bars, execute, kill, run_rule

FOUR = "AAPL+AMZN+MSFT+NVDA"
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
    """8-K item 2.02 filing dates (both repo lists). all four names report after the close, so the
    reaction session is the next session. NVDA 2022-08-08 is a pre-market preannouncement, not the
    scheduled report (that is 2022-08-24) -- dropped."""
    b, out = bars(sym), set()
    files = [f"{ROOT}/research/swing/earnings/{sym}.txt", f"{ROOT}/research/entries/data/earnings_{sym}.txt"]
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
]
