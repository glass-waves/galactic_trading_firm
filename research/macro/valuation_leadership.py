#!/usr/bin/env python3
"""valuation + leadership follow-up to the recession-watch calibration (research only).

questions (owner, 2026-10-15):
  A. valuation is near a record — should the score include it? tests on 1990-2026:
     (i)   does the CAPE percentile (trailing 30-year rank) time >= 15 % drawdown starts (6 / 12 months)?
     (ii)  given a decline reached -8 %, does CAPE at the peak predict depth (>= 20 / 30 %) and duration?
     (iii) does CAPE add to the yield curve for the 12-month recession target?
  B. does "leadership rollover" (Nasdaq-100, semis, cap- vs equal-weight, our 2021+ mega-cap basket)
     lead the S&P 500 at >= 15 % drawdowns and recessions? small pre-registered rule grid chosen on
     1990-2015, evaluated on 2016-2026, false-alarm episodes merged (gap <= 21 sessions).

reuses research/macro/calibrate.py (loaders, labels, the evaluate() framework, the same conventions:
daily series lagged one session, warnings >= 14 days before the 15 % cross, episodes merged).

usage:
    python3 research/macro/valuation_leadership.py            # fetch missing/stale caches (7 days), run
    python3 research/macro/valuation_leadership.py --refresh  # refetch
    python3 research/macro/valuation_leadership.py --no-fetch # caches only

caches (data/macro/series/, gitignored): CAPE_multpl.csv, yahoo_<SYM>.csv, NCBEILQ027S.csv, GDP.csv.
results (research/macro/results/, tracked): valuation.json/.md, leadership.json/.md, current_readings.json.
python3 stdlib + numpy.
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import html
import json
import re
import sys
import time
import warnings
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import calibrate as C  # noqa: E402

SERIES_DIR, OUT, ROOT = C.SERIES_DIR, C.OUT, C.ROOT
START = np.datetime64("1990-01-02")
TRAIN_END = np.datetime64(C.TRAIN_END)
MULTPL = "https://www.multpl.com/shiller-pe/table/by-month"
YAHOO = ("https://query1.finance.yahoo.com/v8/finance/chart/{}?period1=473385600&period2={}"
         "&interval=1d&includeAdjustedClose=true")
YAHOO_SYMS = ["^GSPC", "^NDX", "^SOX", "SPY", "RSP", "QQQ", "SMH",
              "ORCL", "VRT", "CRWV", "NBIS", "APLD", "IREN", "CIFR", "WULF", "CORZ", "NVDA", "^FPMI", "DXYZ"]
AI_INFRA = ["ORCL", "CRWV", "NBIS", "APLD", "VRT", "IREN", "CIFR", "WULF", "CORZ"]   # current readings only
MEGA = ["AAPL", "MSFT", "NVDA", "AMZN", "GOOGL", "META", "AVGO", "TSLA"]   # 2021-06 → (our cache)
SEMIS = ["NVDA", "AMD", "AVGO", "MU", "AMAT", "QCOM", "TXN", "INTC"]


# ---------------------------------------------------------------- fetch / cache

def _fn(sym: str) -> Path:
    return SERIES_DIR / f"yahoo_{sym.replace('^', 'IDX_')}.csv"


def fetch(refresh: bool) -> dict:
    SERIES_DIR.mkdir(parents=True, exist_ok=True)
    st = {}
    p = SERIES_DIR / "CAPE_multpl.csv"
    if C._stale(p, refresh):
        try:
            t = C._http(MULTPL)
            rows = re.findall(r"<tr[^>]*>\s*<td[^>]*>([^<]+)</td>\s*<td[^>]*>(.*?)</td>", t, re.S)
            out = []
            for d, v in rows:
                try:
                    out.append((dt.datetime.strptime(d.strip(), "%b %d, %Y").date(),
                                float(re.sub(r"<[^>]+>", "", html.unescape(v)).strip())))
                except ValueError:
                    continue
            if len(out) > 1000:
                out.sort()
                p.write_text("date,cape\n" + "".join(f"{d},{v}\n" for d, v in out))
                st["CAPE_multpl"] = f"fetched {len(out)} rows"
            else:
                st["CAPE_multpl"] = "bad payload, kept cache"
        except RuntimeError as e:
            st["CAPE_multpl"] = f"failed ({e})"
    else:
        st["CAPE_multpl"] = "cached"
    for sid in ("NCBEILQ027S", "GDP", "T10Y3M", "USREC"):
        p = SERIES_DIR / f"{sid}.csv"
        if C._stale(p, refresh) and sid in ("NCBEILQ027S", "GDP") or not p.exists():
            try:
                txt = C._http(C.FRED_URL.format(sid))
                if txt.startswith("observation_date") or txt.startswith("DATE"):
                    p.write_text(txt); st[sid] = "fetched"
            except RuntimeError as e:
                st[sid] = f"failed ({e})"
        else:
            st[sid] = "cached"
    for s in YAHOO_SYMS:
        p = _fn(s)
        if not C._stale(p, refresh):
            st[s] = "cached"; continue
        try:
            j = json.loads(C._http(YAHOO.format(s.replace("^", "%5E"), int(time.time()) + 86400)))
            r = j["chart"]["result"][0]
            ts, cl = r["timestamp"], r["indicators"]["quote"][0]["close"]
            adj = r["indicators"].get("adjclose", [{}])[0].get("adjclose") or cl
            rows = ["date,close,adjclose"]
            for t, c, a in zip(ts, cl, adj):
                if c is None:
                    continue
                d = (dt.datetime.fromtimestamp(t, dt.timezone.utc) - dt.timedelta(hours=5)).date()
                rows.append(f"{d},{c:.4f},{(a if a is not None else c):.4f}")
            p.write_text("\n".join(rows) + "\n"); st[s] = "fetched"
            time.sleep(0.4)
        except Exception as e:  # noqa: BLE001
            st[s] = f"failed ({e})"
    return st


def read_yahoo(sym: str, adj: bool = False):
    d, v = [], []
    with open(_fn(sym)) as f:
        for row in csv.DictReader(f):
            d.append(row["date"]); v.append(float(row["adjclose" if adj else "close"]))
    d = np.array(d, dtype="datetime64[D]"); v = np.array(v)
    keep = np.concatenate([d[1:] != d[:-1], [True]])
    d, v = d[keep], v[keep]
    today = np.datetime64(dt.date.today().isoformat())
    return d[d < today], v[d < today]


def read_cache_basket(names, dates):
    """equal-weight, daily-rebalanced index of our 2021+ daily cache names on `dates` (nan before)."""
    rets = []
    for n in names:
        p = C.SWING_DAILY / f"{n}.csv"
        if not p.exists():
            continue
        d, c = [], []
        with open(p) as f:
            for row in csv.DictReader(f):
                d.append(row["date"]); c.append(float(row["close"]))
        d = np.array(d, dtype="datetime64[D]"); c = np.array(c)
        x = align(d, c, dates)
        r = np.full(len(dates), np.nan); r[1:] = x[1:] / x[:-1] - 1
        rets.append(r)
    S = np.stack(rets)
    cnt = (~np.isnan(S)).sum(axis=0)
    R = np.where(cnt > 0, np.nansum(S, axis=0) / np.maximum(cnt, 1), np.nan)
    out = np.full(len(dates), np.nan)
    ok = np.flatnonzero(~np.isnan(R))
    if len(ok):
        out[ok[0] - 1] = 100.0
        for i in range(ok[0], len(dates)):
            out[i] = out[i - 1] * (1 + (R[i] if not np.isnan(R[i]) else 0.0))
    return out


def align(d, v, dates):
    """last value at or before each date (same-day close); nan before the series starts or after it ends."""
    idx = np.searchsorted(d, dates, side="right") - 1
    out = np.where(idx >= 0, v[np.maximum(idx, 0)], np.nan)
    out[dates > d[-1] + np.timedelta64(5, "D")] = np.nan
    return out


def lag1(x):
    """a signal computed on today's close is usable from the next session."""
    y = np.empty_like(x)
    y[0] = False if x.dtype == bool else np.nan
    y[1:] = x[:-1]
    return y


# ---------------------------------------------------------------- valuation series

def cape_daily(dates):
    d, v = [], []
    with open(SERIES_DIR / "CAPE_multpl.csv") as f:
        for row in csv.DictReader(f):
            d.append(row["date"]); v.append(float(row["cape"]))
    d = np.array(d, dtype="datetime64[D]"); v = np.array(v)
    # a month's CAPE uses that month's average price: usable from the 1st of the next month.
    # multpl's current-month row is dated today (mid-month): usable the next day.
    one_m, one_d = np.timedelta64(1, "M"), np.timedelta64(1, "D")
    avail = np.array([(np.datetime64(str(x)[:7], "M") + one_m).astype("datetime64[D]") if str(x)[8:] == "01"
                      else x + one_d for x in d], dtype="datetime64[D]")
    n = len(v)
    pct30 = np.full(n, np.nan); pctall = np.full(n, np.nan)
    for i in range(n):
        if i >= 359:
            pct30[i] = np.mean(v[i - 359:i + 1] <= v[i])
        pctall[i] = np.mean(v[:i + 1] <= v[i])
    idx = np.searchsorted(avail, dates, side="right") - 1
    g = lambda a: np.where(idx >= 0, a[np.maximum(idx, 0)], np.nan)  # noqa: E731
    return {"cape": g(v), "cape_pct30": g(pct30), "cape_pct_all": g(pctall),
            "monthly": (d, v, pct30, pctall)}


def buffett_daily(dates, spx_d, spx_v):
    """nonfinancial corporate equities at market value (Z.1, NCBEILQ027S, $m) / nominal GDP ($bn),
    end of quarter, made timely with the S&P 500's move since the quarter end. Z.1 is published about
    10 weeks after the quarter: usable 75 days after the quarter end. pct = rank vs the prior 120 quarters."""
    ed, ev = C.read_fred("NCBEILQ027S"); gd, gv = C.read_fred("GDP")
    common = np.intersect1d(ed, gd)
    ratio = np.array([ev[ed == x][0] / 1000 / gv[gd == x][0] for x in common])
    qend = np.array([(np.datetime64(str(x)[:7], "M") + np.timedelta64(3, "M")).astype("datetime64[D]")
                     - np.timedelta64(1, "D") for x in common])
    avail = qend + np.timedelta64(75, "D")
    idx = np.searchsorted(avail, dates, side="right") - 1
    spx_q = align(spx_d, spx_v, qend)
    spx_t = align(spx_d, spx_v, dates)
    out = np.full(len(dates), np.nan); pct = np.full(len(dates), np.nan)
    for i in range(len(dates)):
        j = idx[i]
        if j < 120 or np.isnan(spx_q[j]):
            continue
        out[i] = ratio[j] * spx_t[i] / spx_q[j]
        pct[i] = np.mean(ratio[j - 119:j + 1] <= out[i])
    return {"buffett": out, "buffett_pct30": pct, "last_quarter": str(common[-1]), "last_ratio": float(ratio[-1]),
            "max_ratio_quarter": str(common[int(np.argmax(ratio))]), "max_ratio": float(ratio.max())}


def curve_signals(dates):
    td, tv = C.read_fred("T10Y3M")
    t63 = C.asof(td, C.rolling_mean(tv, 63), dates, 1)
    now = C.asof(td, tv, dates, 1)
    inv = np.nan_to_num(t63, nan=1) < 0
    rec = np.zeros(len(dates), bool); last = None
    for i in range(len(dates)):
        if inv[i]:
            last = dates[i]
        rec[i] = last is not None and (dates[i] - last) <= np.timedelta64(365, "D")
    score = np.where(rec, 2, np.where(np.nan_to_num(now, nan=1) < 0.25, 1, 0))
    return score, t63


# ---------------------------------------------------------------- targets on the 1990 calendar

def dd_labels(P, eps, horizon):
    dates = P["dates"]; lab = np.zeros(len(dates))
    for e in eps:
        lab[(dates >= e["peak"] - np.timedelta64(horizon, "D")) & (dates < e["cross"])] = 1
    for e in eps:
        lab[(dates >= e["cross"]) & (dates <= e["end"])] = np.nan
    lab[(dates > dates[-1] - np.timedelta64(horizon, "D")) & (lab != 1)] = np.nan
    return lab


def dd_windows(P, eps, horizon, until="cross14"):
    """until: 'cross14' = warning >= 14 days before the threshold cross (calibrate.py convention);
    'dd5' = before the index first closes 5 % below its high after the peak."""
    dates, dd = P["dates"], P["dd"]
    out = []
    for e in eps:
        ws = np.searchsorted(dates, e["peak"] - np.timedelta64(horizon, "D"))
        wpk = np.searchsorted(dates, e["peak"])
        if until == "dd5":
            j = wpk + int(np.argmax(dd[wpk:] >= 5))
            we = j
        else:
            we = np.searchsorted(dates, e["cross"] - np.timedelta64(14, "D"))
        out.append({"name": str(e["peak"])[:7], "ws": ws, "we": we, "wpk": wpk, "ref": e["cross"], "ref2": e["peak"]})
    return [w for w in out if w["we"] > 0]


def masks(dates, lo=None):
    base = np.ones(len(dates), bool) if lo is None else dates >= np.datetime64(lo)
    tr = dates <= TRAIN_END
    return {"full": base, "train": base & tr, "test": base & ~tr}


def pwins(wins, per, lo=None):
    w = wins if lo is None else [x for x in wins if x["ref2"] >= np.datetime64(lo)]
    if per == "full":
        return w
    return [x for x in w if (x["ref2"] <= TRAIN_END) == (per == "train")]


def ev_row(name, alarm, lab, wins, dates, mk, base_rate, extra=None):
    r = C.evaluate(alarm, lab, wins, dates, mk)
    lift = r["day_precision"] / base_rate if (r["day_precision"] is not None and base_rate) else None
    row = {"signal": name, "day_precision": r["day_precision"], "lift": lift,
           "events_hit": r["events_detected"], "events": r["events"],
           "median_lead_months": C.months(r["median_lead_days"]), "false_episodes": r["false_episodes"],
           "false_episode_list": r["false_episode_list"], "episode_precision": r["episode_precision"],
           "days_on": r["alarm_day_share"], "objective": C.objective(r),
           "before_peak": r["detected_before_peak"]}
    if extra:
        row.update(extra)
    return row


def md_rows(rows, head=True):
    L = []
    if head:
        L += ["| signal | lift | events hit | median lead (mo) | false ep | ep prec | days on |",
              "|---|---|---|---|---|---|---|"]
    for r in rows:
        L.append(f"| {r['signal']} | {C.fmt(r['lift'], 1)}x | {len(r['events_hit'])}/{r['events']} "
                 f"({','.join(x[2:] for x in r['events_hit'])}) | {C.fmt(r['median_lead_months'], 1)} | "
                 f"{r['false_episodes']} | {C.fmt(r['episode_precision'])} | {C.fmt(r['days_on'])} |")
    return L


def base_rate(lab, mk):
    lp = lab[mk & ~np.isnan(lab)]
    return float(np.mean(lp == 1)) if len(lp) else None


# ---------------------------------------------------------------- small stats

def auc(score, y):
    ok = ~np.isnan(score) & ~np.isnan(y)
    s, t = score[ok], y[ok]
    pos, neg = s[t == 1], s[t == 0]
    if not len(pos) or not len(neg):
        return None
    order = np.argsort(np.concatenate([pos, neg]), kind="mergesort")
    ranks = np.empty(len(order)); ranks[order] = np.arange(1, len(order) + 1)
    allv = np.concatenate([pos, neg])
    for v in np.unique(allv):  # average ties
        m = allv == v
        ranks[m] = ranks[m].mean()
    return float((ranks[:len(pos)].sum() - len(pos) * (len(pos) + 1) / 2) / (len(pos) * len(neg)))


def logit(X, y, iters=50):
    w = np.zeros(X.shape[1])
    for _ in range(iters):
        p = 1 / (1 + np.exp(-X @ w))
        H = X.T @ (X * (p * (1 - p))[:, None]) + 1e-6 * np.eye(X.shape[1])
        w += np.linalg.solve(H, X.T @ (y - p))
    p = np.clip(1 / (1 + np.exp(-X @ w)), 1e-9, 1 - 1e-9)
    return w, float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p)))


def spearman(a, b):
    ra = np.argsort(np.argsort(a)); rb = np.argsort(np.argsort(b))
    return float(np.corrcoef(ra, rb)[0, 1])


# ---------------------------------------------------------------- study

def declines_8pct(dates, close, dd, gap=63):
    idx = np.flatnonzero(dd >= 8)
    brk = np.flatnonzero(np.diff(idx) > gap)
    st = np.concatenate([[idx[0]], idx[brk + 1]]); en = np.concatenate([idx[brk], [idx[-1]]])
    runs = []
    for a, b in zip(st, en):  # two -8 % runs measured from the same peak are one decline
        lo = max(0, a - 251)
        pk = lo + int(np.argmax(close[lo:a + 1]))
        if runs and runs[-1][0] == pk:
            runs[-1][2] = b
        else:
            runs.append([pk, a, b])
    out = []
    for pk, a, b in runs:
        tr = a + int(np.argmin(close[a:b + 1]))
        rec = np.flatnonzero(close[tr:] >= close[pk])
        out.append({"i_peak": pk, "i_first8": a, "i_trough": tr, "peak": str(dates[pk]), "first_8pct": str(dates[a]),
                    "trough": str(dates[tr]), "max_dd": round(float((1 - close[tr] / close[pk]) * 100), 1),
                    "sessions_peak_to_trough": int(tr - pk),
                    "sessions_peak_to_recovery": int(tr + rec[0] - pk) if len(rec) else None})
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--refresh", action="store_true")
    ap.add_argument("--no-fetch", action="store_true")
    a = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    fst = {} if a.no_fetch else fetch(a.refresh)

    sd, sv = read_yahoo("^GSPC")
    dates = sd[sd >= START]
    off = int(np.searchsorted(sd, dates[0]))
    hi = C.rolling_max(sv, 252)
    dd_all = (hi - sv) / hi * 100
    P = {"dates": dates, "spx_dates_all": sd, "spx_all": sv, "dd": dd_all[off:],
         "raw": {"USREC": C.read_fred("USREC")}}
    close = sv[off:]
    rstarts = [s for s in C.recession_starts(P) if s >= np.datetime64("1990-01-01")]
    eps = {thr: [e for e in C.drawdown_episodes(P, thr) if e["cross"] >= np.datetime64("1990-01-01")]
           for thr in (15, 20)}
    labA = C.labels_recession(P, rstarts)
    winsA = C.event_windows(P, "A", rstarts)
    lab6 = dd_labels(P, eps[15], 182); lab12 = dd_labels(P, eps[15], 365)
    win6 = dd_windows(P, eps[15], 182); win12 = dd_windows(P, eps[15], 365)
    lab20 = dd_labels(P, eps[20], 182); win20 = dd_windows(P, eps[20], 182)
    M = masks(dates)
    meta = {"generated": dt.datetime.now().isoformat(timespec="seconds"), "fetch": fst,
            "calendar": [str(dates[0]), str(dates[-1]), int(len(dates))],
            "recession_starts": [str(s) for s in rstarts],
            "drawdowns15": [{"peak": str(e["peak"]), "cross": str(e["cross"]), "trough": str(e["trough"]),
                             "max_dd": round(e["max_dd"], 1)} for e in eps[15]]}

    # ================================================================ PART A — valuation
    V = cape_daily(dates)
    B = buffett_daily(dates, sd, sv)
    curve, t63 = curve_signals(dates)
    md_d, md_v, md_p30, md_pall = V["monthly"]
    valuation = {"meta": meta, "cape_now": {"date": str(md_d[-1]), "cape": float(md_v[-1]),
                                            "pct30": float(md_p30[-1]), "pct_since_1871": float(md_pall[-1]),
                                            "record": {"date": str(md_d[int(np.argmax(md_v))]), "cape": float(md_v.max())}},
                 "buffett_now": {"timely_ratio": float(B["buffett"][-1]), "pct30": float(B["buffett_pct30"][-1]),
                                 "last_z1_quarter": B["last_quarter"], "last_z1_ratio": B["last_ratio"],
                                 "z1_record": [B["max_ratio_quarter"], B["max_ratio"]]}}
    share_hi = {f"cape_pct30>={k}": float(np.mean(V["cape_pct30"] >= k)) for k in (0.8, 0.9)}
    share_hi.update({f"buffett_pct30>={k}": float(np.nanmean(B["buffett_pct30"] >= k)) for k in (0.8, 0.9)})
    valuation["share_of_days_1990_2026"] = share_hi
    vmd = ["# valuation tests (generated by valuation_leadership.py)\n",
           f"CAPE now {md_v[-1]:.1f} ({md_d[-1]}); trailing-30y pct {md_p30[-1]:.2f}; pct since 1871 "
           f"{md_pall[-1]:.3f}; record {md_v.max():.1f} ({md_d[int(np.argmax(md_v))]}).",
           f"Buffett proxy (Z.1 nonfin equities / GDP, timely) now {B['buffett'][-1]:.2f}, pct30 "
           f"{B['buffett_pct30'][-1]:.2f}; Z.1 record {B['max_ratio']:.2f} ({B['max_ratio_quarter']}).",
           f"share of days 1990-2026: {json.dumps({k: round(v, 2) for k, v in share_hi.items()})}\n"]

    # (i) timing
    timing = {}
    sigs = {"CAPE pct30 >= 0.8 (top quintile)": V["cape_pct30"] >= 0.8,
            "CAPE pct30 >= 0.9 (top decile)": V["cape_pct30"] >= 0.9,
            "CAPE pct since 1871 >= 0.9": V["cape_pct_all"] >= 0.9,
            "Buffett pct30 >= 0.9": np.nan_to_num(B["buffett_pct30"]) >= 0.9,
            "CAPE >= 30 (level)": V["cape"] >= 30}
    for tname, lab, wins in (("B15 within 6m", lab6, win6), ("B15 within 12m", lab12, win12), ("B20 within 6m", lab20, win20)):
        timing[tname] = {}
        for per in ("full", "train", "test"):
            br = base_rate(lab, M[per])
            rows = [ev_row(n, s, lab, pwins(wins, per), dates, M[per], br) for n, s in sigs.items()]
            timing[tname][per] = {"base_rate": br, "rows": rows}
            if per == "full":
                vmd.append(f"\n## (i) timing — {tname}, {per} 1990-2026 (base rate {br:.2f})\n")
                vmd += md_rows(rows)
    # monthly frequency by CAPE pct30 bucket: P(a >= 15 % drawdown PEAKS within the next 12 months)
    me = np.array([i for i in range(len(dates) - 1) if str(dates[i])[5:7] != str(dates[i + 1])[5:7]])
    peaks = np.array([e["peak"] for e in eps[15]])
    fut = np.array([bool(np.any((peaks > dates[i]) & (peaks <= dates[i] + np.timedelta64(365, "D")))) for i in me])
    known = dates[me] <= dates[-1] - np.timedelta64(365, "D")
    inside = np.array([any(e["peak"] <= dates[i] <= e["end"] for e in eps[15]) for i in me])
    use = known & ~inside
    buckets = [(0, 0.6), (0.6, 0.8), (0.8, 0.9), (0.9, 1.01)]
    bt = []
    for lo_, hi_ in buckets:
        m = use & (V["cape_pct30"][me] >= lo_) & (V["cape_pct30"][me] < hi_)
        bt.append({"bucket": f"{lo_:.1f}-{min(hi_, 1):.1f}", "months": int(m.sum()),
                   "share_followed_by_peak_12m": float(fut[m].mean()) if m.sum() else None})
    allr = float(fut[use].mean())
    timing["monthly_bucket_table"] = {"unconditional": allr, "rows": bt}
    vmd.append(f"\nmonthly (month-ends outside a drawdown, 1990–2025-10): share followed by a ≥15% drawdown peak "
               f"within 12 months — all {allr:.2f}; " + "; ".join(
                   f"pct30 {r['bucket']}: {C.fmt(r['share_followed_by_peak_12m'])} (n={r['months']})" for r in bt))
    # context: forward S&P price return by CAPE level at month-end (1990 → last fully known month)
    fwd = {}
    for h, nm in ((252, "12m"), (756, "36m")):
        ok_ = me[me + h < len(dates)]
        r_ = (close[ok_ + h] / close[ok_] - 1) * 100
        mx = np.array([(1 - close[i:i + h + 1].min() / close[i]) * 100 for i in ok_])
        cv = V["cape"][ok_]
        rows = []
        for lo_, hi_ in ((0, 20), (20, 25), (25, 30), (30, 35), (35, 60)):
            m = (cv >= lo_) & (cv < hi_)
            if m.sum():
                rows.append({"cape": f"{lo_}-{hi_}", "months": int(m.sum()), "median_return_pct": round(float(np.median(r_[m])), 1),
                             "share_negative": round(float(np.mean(r_[m] < 0)), 2),
                             "median_max_drawdown_within_pct": round(float(np.median(mx[m])), 1)})
        fwd[nm] = rows
    timing["forward_return_by_cape"] = fwd
    for nm, rows in fwd.items():
        vmd.append(f"\nforward {nm} S&P price return by CAPE at month-end: " + "; ".join(
            f"CAPE {r['cape']}: median {r['median_return_pct']:+.1f}%, {r['share_negative']:.0%} negative, median worst dd {r['median_max_drawdown_within_pct']:.0f}% (n={r['months']})" for r in rows))
    valuation["timing"] = timing

    # (ii) depth / duration given an 8 % decline
    dec = declines_8pct(dates, close, P["dd"])
    for r in dec:
        ip, i8 = r["i_peak"], r["i_first8"]
        r["cape_at_peak"] = round(float(V["cape"][ip]), 1)
        r["cape_pct30_at_peak"] = round(float(V["cape_pct30"][ip]), 2)
        r["buffett_pct30_at_peak"] = None if np.isnan(B["buffett_pct30"][ip]) else round(float(B["buffett_pct30"][ip]), 2)
        r["curve_at_8pct"] = int(curve[i8])
        # recessionary decline: an NBER start between 6 months before the peak and 6 months after the trough
        r["recession_within_12m"] = any(dates[ip] - np.timedelta64(183, "D") <= s <= dates[r["i_trough"]] + np.timedelta64(183, "D")
                                        for s in rstarts)
        r["ongoing"] = r["sessions_peak_to_recovery"] is None
    n = len(dec)
    order = sorted(range(n), key=lambda k: dec[k]["cape_pct30_at_peak"])
    terc = {}
    cuts = [order[: n // 3], order[n // 3: 2 * n // 3], order[2 * n // 3:]]
    for name, ids in zip(("low", "mid", "high"), cuts):
        rs = [dec[k] for k in ids]
        p2t = [x["sessions_peak_to_trough"] for x in rs]
        rcv = [x["sessions_peak_to_recovery"] for x in rs if x["sessions_peak_to_recovery"] is not None]
        terc[name] = {"n": len(rs), "cape_pct30_range": [rs[0]["cape_pct30_at_peak"], rs[-1]["cape_pct30_at_peak"]],
                      "cape_range": [min(x["cape_at_peak"] for x in rs), max(x["cape_at_peak"] for x in rs)],
                      "reached15": sum(x["max_dd"] >= 15 for x in rs), "reached20": sum(x["max_dd"] >= 20 for x in rs),
                      "reached30": sum(x["max_dd"] >= 30 for x in rs),
                      "median_max_dd": float(np.median([x["max_dd"] for x in rs])),
                      "median_sessions_peak_to_trough": float(np.median(p2t)),
                      "median_sessions_peak_to_recovery": float(np.median(rcv)) if rcv else None,
                      "recession_within_12m": sum(x["recession_within_12m"] for x in rs),
                      "episodes": [x["peak"][:7] for x in rs]}
    # 2x2: valuation high/low (pct30 >= 0.9) x curve on/off at the 8 % day
    two = {}
    for vh in (True, False):
        for cv in (True, False):
            rs = [x for x in dec if (x["cape_pct30_at_peak"] >= 0.9) == vh and (x["curve_at_8pct"] == 2) == cv]
            two[f"cape_pct30{'>=' if vh else '<'}0.9 & curve{'=2' if cv else '<2'}"] = {
                "n": len(rs), "reached20": sum(x["max_dd"] >= 20 for x in rs), "reached30": sum(x["max_dd"] >= 30 for x in rs),
                "episodes": [f"{x['peak'][:7]} -{x['max_dd']:.0f}%" for x in rs]}
    mdd = np.array([x["max_dd"] for x in dec]); cp = np.array([x["cape_pct30_at_peak"] for x in dec])
    cl = np.array([x["cape_at_peak"] for x in dec])
    p2t = np.array([x["sessions_peak_to_trough"] for x in dec])
    depth = {"declines": dec, "terciles_by_cape_pct30": terc, "two_by_two": two,
             "spearman_cape_pct30_vs_maxdd": spearman(cp, mdd), "spearman_cape_level_vs_maxdd": spearman(cl, mdd),
             "spearman_cape_pct30_vs_peak_to_trough": spearman(cp, p2t),
             "spearman_curve_vs_maxdd": spearman(np.array([x["curve_at_8pct"] for x in dec], float), mdd)}
    for x in dec:
        for k in ("i_peak", "i_first8", "i_trough"):
            x.pop(k)
    valuation["depth_given_8pct"] = depth
    vmd.append(f"\n## (ii) every decline that reached −8% (1990–2026, n={n}): depth and duration by CAPE tercile at the peak\n")
    vmd.append("| tercile (pct30 range) | n | ≥15% | ≥20% | ≥30% | median max dd | median sessions to trough | median sessions to recover | recessionary | episodes |")
    vmd.append("|---|---|---|---|---|---|---|---|---|---|")
    for k, t in terc.items():
        vmd.append(f"| {k} ({t['cape_pct30_range'][0]:.2f}–{t['cape_pct30_range'][1]:.2f}; CAPE {t['cape_range'][0]}–{t['cape_range'][1]}) | "
                   f"{t['n']} | {t['reached15']} | {t['reached20']} | {t['reached30']} | {t['median_max_dd']:.1f} | "
                   f"{t['median_sessions_peak_to_trough']:.0f} | {C.fmt(t['median_sessions_peak_to_recovery'], 0)} | "
                   f"{t['recession_within_12m']} | {', '.join(t['episodes'])} |")
    vmd.append(f"\nSpearman: CAPE pct30 vs max dd {depth['spearman_cape_pct30_vs_maxdd']:.2f}; CAPE level vs max dd "
               f"{depth['spearman_cape_level_vs_maxdd']:.2f}; CAPE pct30 vs sessions to trough "
               f"{depth['spearman_cape_pct30_vs_peak_to_trough']:.2f}; curve score at the 8% day vs max dd "
               f"{depth['spearman_curve_vs_maxdd']:.2f}\n")
    vmd.append("| valuation × curve at the −8% day | n | ≥20% | ≥30% | episodes |\n|---|---|---|---|---|")
    for k, t in two.items():
        vmd.append(f"| {k} | {t['n']} | {t['reached20']} | {t['reached30']} | {'; '.join(t['episodes'])} |")
    vmd.append("\n| peak | first −8% | max dd | to trough | to recover | CAPE | pct30 | Buffett pct30 | curve | recessionary |\n|---|---|---|---|---|---|---|---|---|---|")
    for x in dec:
        vmd.append(f"| {x['peak']} | {x['first_8pct']} | {x['max_dd']} | {x['sessions_peak_to_trough']} | "
                   f"{C.fmt(x['sessions_peak_to_recovery'])} | {x['cape_at_peak']} | {x['cape_pct30_at_peak']} | "
                   f"{C.fmt(x['buffett_pct30_at_peak'])} | {x['curve_at_8pct']} | {'yes' if x['recession_within_12m'] else ''} |")

    # (iii) recession within 12 months: curve alone vs curve + CAPE
    rsig = {"curve_avg63 = 2": curve == 2,
            "CAPE pct30 >= 0.9": V["cape_pct30"] >= 0.9,
            "curve = 2 AND CAPE pct30 >= 0.8": (curve == 2) & (V["cape_pct30"] >= 0.8),
            "curve = 2 AND CAPE pct30 >= 0.9": (curve == 2) & (V["cape_pct30"] >= 0.9),
            "curve = 2 OR CAPE pct30 >= 0.95": (curve == 2) | (V["cape_pct30"] >= 0.95)}
    rec = {}
    vmd.append("\n## (iii) recession within 12 months (1990-08, 2001-04, 2008-01, 2020-03)\n")
    for per in ("full", "train", "test"):
        br = base_rate(labA, M[per])
        rows = [ev_row(nm, s, labA, pwins(winsA, per), dates, M[per], br) for nm, s in rsig.items()]
        rec[per] = {"base_rate": br, "rows": rows}
        vmd.append(f"\n{per} (base rate {br:.2f})\n")
        vmd += md_rows(rows)
    # logistic on month-ends: curve level (63d avg) with / without CAPE pct30; train 1990-2015, test 2016+
    yA = labA[me]; x1 = t63[me]; x2 = V["cape_pct30"][me]
    ok = ~np.isnan(yA) & ~np.isnan(x1) & ~np.isnan(x2)
    trm = ok & (dates[me] <= TRAIN_END); tem = ok & (dates[me] > TRAIN_END)
    Xa = np.column_stack([np.ones(len(me)), x1]); Xb = np.column_stack([np.ones(len(me)), x1, x2])
    wa, lla = logit(Xa[trm], yA[trm]); wb, llb = logit(Xb[trm], yA[trm])
    sa = Xa @ wa; sb = Xb @ wb
    lg = {"months_train": int(trm.sum()), "months_test": int(tem.sum()),
          "curve_only": {"coef": wa.tolist(), "loglik_train": lla, "auc_train": auc(sa[trm], yA[trm]), "auc_test": auc(sa[tem], yA[tem])},
          "curve_plus_cape": {"coef": wb.tolist(), "loglik_train": llb, "auc_train": auc(sb[trm], yA[trm]), "auc_test": auc(sb[tem], yA[tem])},
          "cape_only_auc": {"train": auc(x2[trm], yA[trm]), "test": auc(x2[tem], yA[tem])},
          "note": "monthly obs are heavily autocorrelated (12 overlapping labels per event): read the log-lik gain as descriptive"}
    rec["logit_monthly"] = lg
    vmd.append(f"\nmonthly logit (train 1990–2015, {lg['months_train']} months; test {lg['months_test']}): curve only AUC "
               f"train {lg['curve_only']['auc_train']:.2f} / test {C.fmt(lg['curve_only']['auc_test'])}; curve + CAPE pct30 AUC "
               f"train {lg['curve_plus_cape']['auc_train']:.2f} / test {C.fmt(lg['curve_plus_cape']['auc_test'])}; "
               f"log-lik {lla:.1f} → {llb:.1f}; CAPE coef {wb[2]:+.2f}; CAPE alone AUC train {C.fmt(lg['cape_only_auc']['train'])} / "
               f"test {C.fmt(lg['cape_only_auc']['test'])}")
    valuation["recession_12m"] = rec
    (OUT / "valuation.json").write_text(json.dumps(valuation, indent=1, default=str))
    (OUT / "valuation.md").write_text("\n".join(vmd) + "\n")

    # ================================================================ PART B — leadership
    I = close
    spy = align(*read_yahoo("SPY", adj=True), dates)
    proxies = {
        "NDX": (align(*read_yahoo("^NDX"), dates), I),
        "SOX": (align(*read_yahoo("^SOX"), dates), I),
        # cap-weight leaders vs the average stock: SPY (total return) relative to RSP (total return)
        "CAPvsEW": (spy, align(*read_yahoo("RSP", adj=True), dates)),
        "MEGA8": (read_cache_basket(MEGA, dates), I),
        "SEMI8": (read_cache_basket(SEMIS, dates), I),
    }
    feats = {}
    for nm, (L, X) in proxies.items():
        f = {}
        hL = np.full(len(L), np.nan); hX = np.full(len(L), np.nan)
        ok = np.flatnonzero(~np.isnan(L))
        s0 = ok[0] if len(ok) else len(L)
        if s0 < len(L):
            hL[s0:] = C.rolling_max(L[s0:], 252);
        okx = np.flatnonzero(~np.isnan(X)); s1 = okx[0]
        hX[s1:] = C.rolling_max(X[s1:], 252)
        if nm in ("NDX", "SOX", "MEGA8", "SEMI8"):
            hX = hi[off:]  # the S&P's own 252-session high (history from 1985)
        f["ddL"] = (1 - L / hL) * 100; f["ddX"] = (1 - X / hX) * 100
        rel = L / X
        f["rel21"] = np.full(len(L), np.nan); f["rel21"][21:] = (rel[21:] / rel[:-21] - 1) * 100
        f["rel63"] = np.full(len(L), np.nan); f["rel63"][63:] = (rel[63:] / rel[:-63] - 1) * 100
        sL = np.full(len(L), np.nan); sX = np.full(len(L), np.nan)
        if s0 < len(L):
            sL[s0:] = C.rolling_mean(L[s0:], 50)
        sX[s1:] = C.rolling_mean(X[s1:], 50)
        cond = (L < sL) & (X > sX)
        f["div50_20"] = C.rolling_mean(cond.astype(float), 20)
        hr = np.full(len(L), np.nan)
        okr = np.flatnonzero(~np.isnan(rel))
        if len(okr):
            hr[okr[0]:] = C.rolling_max(np.where(np.isnan(rel[okr[0]:]), -np.inf, rel[okr[0]:]), 252)
        f["ddR"] = (1 - rel / hr) * 100   # the relative-strength line's drawdown from its 252-session high
        f["rel"] = rel
        f["valid"] = ~np.isnan(f["rel63"]) & ~np.isnan(f["ddL"])
        feats[nm] = f

    def rule(nm, kind, a_, b_=None):
        f = feats[nm]
        with np.errstate(invalid="ignore"):
            if kind == "R1":
                s = (f["ddL"] >= a_) & (f["ddX"] <= b_)
            elif kind == "R2":
                s = f["rel63"] <= -a_
            elif kind == "R3":
                s = f["div50_20"] * 20 >= a_
            elif kind == "R4":
                s = f["rel21"] <= -a_
            elif kind == "R5":
                s = f["ddR"] >= a_
            else:  # R6: narrowing leadership — cap-weight beating equal-weight by a_ % over 3 months
                s = f["rel63"] >= a_
        return lag1(np.nan_to_num(s, nan=0).astype(bool) & f["valid"])

    # pre-registered grid (written before any result was seen; CAPvsEW thresholds scaled to the
    # much smaller spread between cap- and equal-weight)
    TH = {"NDX": {"R1": (8, 10, 12, 15, 20), "R2": (5, 8, 10, 12), "R4": (4, 6, 8), "R5": (5, 8, 10, 12)},
          "SOX": {"R1": (8, 10, 12, 15, 20), "R2": (5, 8, 10, 12), "R4": (4, 6, 8), "R5": (10, 15, 20, 25)},
          "CAPvsEW": {"R2": (2, 3, 4, 5), "R4": (1.5, 2, 3), "R5": (3, 4, 5, 6), "R6": (3, 4, 5)}}
    grid = []
    for nm, th in TH.items():
        for x in th.get("R1", ()):
            for y in (3, 5):
                grid.append((nm, "R1", x, y, f"{nm}: leaders ≥{x}% off high while S&P within {y}%"))
        for z in th["R2"]:
            grid.append((nm, "R2", z, None, f"{nm}: 3-month relative return ≤ −{z}%"))
        for k in (5, 10, 15):
            grid.append((nm, "R3", k, None, f"{nm}: leaders < 50d while S&P > 50d on ≥{k} of 20 days"))
        for w in th["R4"]:
            grid.append((nm, "R4", w, None, f"{nm}: 1-month relative return ≤ −{w}%"))
        for x in th["R5"]:
            grid.append((nm, "R5", x, None, f"{nm}: relative-strength line ≥{x}% below its 1-year high"))
        for x in th.get("R6", ()):
            grid.append((nm, "R6", x, None, f"{nm}: narrowing — cap-weight beats equal-weight by ≥{x}% in 3 months"))
    win5 = dd_windows(P, eps[15], 182, until="dd5")
    res = []
    for nm, kind, x, y, label in grid:
        s = rule(nm, kind, x, y)
        row = {"rule": label, "proxy": nm, "kind": kind, "params": [x, y]}
        for per in ("train", "test", "full"):
            r = C.evaluate(s, lab6, pwins(win6, per), dates, M[per])
            r5 = C.evaluate(s, lab6, pwins(win5, per), dates, M[per])
            rA = C.evaluate(s, labA, pwins(winsA, per), dates, M[per])
            br = base_rate(lab6, M[per])
            row[per] = {"objective": C.objective(r), "events_hit": r["events_detected"], "events": r["events"],
                        "before_index_-5pct": r5["events_detected"], "before_peak": r["detected_before_peak"],
                        "false_episodes": r["false_episodes"], "episode_precision": r["episode_precision"],
                        "lift": (r["day_precision"] / br) if r["day_precision"] is not None and br else None,
                        "days_on": r["alarm_day_share"], "median_lead_months": C.months(r["median_lead_days"]),
                        "recessions_hit": rA["events_detected"], "recession_false_episodes": rA["false_episodes"],
                        "false_episode_list": r["false_episode_list"]}
        res.append(row)
    # selection on 1990-2015: objective, then hits before -5 %, then fewer days on
    key = lambda r: (round(r["train"]["objective"], 6), len(r["train"]["before_index_-5pct"]), -r["train"]["days_on"])  # noqa: E731
    best_by_family = {}
    for r in res:
        k = (r["proxy"], r["kind"])
        if k not in best_by_family or key(r) > key(best_by_family[k]):
            best_by_family[k] = r
    overall = max((r for r in res if r["proxy"] != "CAPvsEW"), key=key)   # CAPvsEW has only 3 train events
    lmd = ["# leadership rollover (generated by valuation_leadership.py)\n",
           "target B15: a ≥15% S&P drawdown peaks within 6 months; warning ≥14 days before the 15% cross; "
           "'pre −5%' = fired between peak − 6 months and the first close 5% below the high. "
           "False episodes merged with gaps ≤21 sessions. Rules chosen on 1990–2015 (NDX: 6 events; SOX from 1994: 5; "
           "cap vs equal weight from 2003: 3), evaluated on 2016–2026 (4 events: 2018, 2020, 2022, 2025).\n",
           "| rule (best of its family on train) | train obj | train hit / pre −5% | train false (ep prec) | test hit / pre −5% | test false | test ep prec | lift full | days on | recessions hit (full) |",
           "|---|---|---|---|---|---|---|---|---|---|"]
    fam = sorted(best_by_family.values(), key=key, reverse=True)
    for r in fam:
        t, e, fu = r["train"], r["test"], r["full"]
        lmd.append(f"| {r['rule']} | {t['objective']:.2f} | {len(t['events_hit'])}/{t['events']} / {len(t['before_index_-5pct'])} | "
                   f"{t['false_episodes']} ({C.fmt(t['episode_precision'])}) | {len(e['events_hit'])}/{e['events']} ({','.join(x[2:] for x in e['events_hit'])}) / "
                   f"{len(e['before_index_-5pct'])} | {e['false_episodes']} | {C.fmt(e['episode_precision'])} | "
                   f"{C.fmt(fu['lift'], 1)}x | {C.fmt(fu['days_on'])} | {len(fu['recessions_hit'])}/4 |")
    lmd.append(f"\n**overall train pick (NDX/SOX): {overall['rule']}** — train {overall['train']['objective']:.2f}, "
               f"test hit {overall['test']['events_hit']}, pre −5% {overall['test']['before_index_-5pct']}, "
               f"false {overall['train']['false_episodes']}/{overall['test']['false_episodes']}")
    # pre-registered rules from the brief, reported regardless of selection
    pre = [r for r in res if (r["kind"] == "R1" and r["params"] == [10, 3]) or (r["kind"] == "R2" and r["params"][0] == 8)]
    lmd.append("\npre-registered rules from the brief:\n")
    lmd.append("| rule | train hit / pre −5% / false | test hit / pre −5% / false | full ep prec | days on |\n|---|---|---|---|---|")
    for r in pre:
        t, e, fu = r["train"], r["test"], r["full"]
        lmd.append(f"| {r['rule']} | {len(t['events_hit'])}/{t['events']} / {len(t['before_index_-5pct'])} / {t['false_episodes']} | "
                   f"{len(e['events_hit'])}/{e['events']} / {len(e['before_index_-5pct'])} / {e['false_episodes']} | "
                   f"{C.fmt(fu['episode_precision'])} | {C.fmt(fu['days_on'])} |")

    # episode table: did leaders peak first? (each >= 15 % drawdown; recessionary ones flagged)
    s_over = rule(overall["proxy"], overall["kind"], *overall["params"])
    s_pre1 = {nm: rule(nm, "R1", 10, 3) for nm in ("NDX", "SOX", "MEGA8", "SEMI8")}
    s_pre2 = {nm: rule(nm, "R2", 8) for nm in ("NDX", "SOX", "CAPvsEW", "MEGA8", "SEMI8")}
    fires = {"pick": s_over, "SOX R1 10/3": s_pre1["SOX"], "SOX rel63<=-8": s_pre2["SOX"],
             "NDX rel63<=-8": s_pre2["NDX"]}
    etab = []
    for e in eps[15]:
        ip = int(np.searchsorted(dates, e["peak"])); ic = int(np.searchsorted(dates, e["cross"]))
        j5 = ip + int(np.argmax(P["dd"][ip:] >= 5))
        recn = any(e["peak"] - np.timedelta64(183, "D") <= x <= e["trough"] + np.timedelta64(183, "D") for x in rstarts)
        row = {"event": f"{str(e['peak'])[:7]} (−{e['max_dd']:.0f}%){' R' if recn else ''}", "recession": recn,
               "index_peak": str(dates[ip]), "index_-5pct": str(dates[j5]), "sessions_peak_to_-5pct": int(j5 - ip)}
        lo_ = max(0, ip - 252); hi_ = max(ic, ip + 21)
        for nm in ("NDX", "SOX", "MEGA8", "SEMI8", "CAPvsEW"):
            for kind, series in (("abs", proxies[nm][0]), ("rs", feats[nm]["rel"])):
                if nm == "CAPvsEW" and kind == "abs":
                    continue
                seg = series[lo_:hi_ + 1]
                if np.sum(~np.isnan(seg)) < 60 or np.isnan(series[ip]):
                    row[f"{nm}_{kind}"] = None; continue
                lp = lo_ + int(np.nanargmax(seg))
                ddv = feats[nm]["ddL"][ip] if kind == "abs" else feats[nm]["ddR"][ip]
                row[f"{nm}_{kind}"] = {"peak": str(dates[lp]), "lead_sessions": int(ip - lp),
                                       "dd_at_index_peak": None if np.isnan(ddv) else round(float(ddv), 1)}
        w0 = int(np.searchsorted(dates, dates[ip] - np.timedelta64(182, "D")))
        for tag, sgl in fires.items():
            f_ = np.flatnonzero(sgl[w0:j5])
            row[f"fire_{tag}"] = None if not len(f_) else {"date": str(dates[w0 + f_[0]]), "sessions_before_peak": int(ip - (w0 + f_[0]))}
        etab.append(row)
    lmd.append("\n## episode table — leader peak vs S&P peak (sessions, + = leaders first; dd = leaders' distance below "
               "their high on the S&P peak day). abs = price, rs = relative-strength line vs the S&P. R = recessionary\n")
    lmd.append("| drawdown | S&P peak → −5% (sessions) | NDX abs | SOX abs | SOX rs | cap/EW rs | MEGA8 abs | pick fired | SOX rel63≤−8 fired | NDX rel63≤−8 fired |")
    lmd.append("|---|---|---|---|---|---|---|---|---|---|")
    def cell(x):
        return "–" if not x else f"{x['lead_sessions']:+d} (−{x['dd_at_index_peak']}%)"
    def fcell(x):
        return "–" if not x else f"{x['date']} ({x['sessions_before_peak']:+d})"
    for r in etab:
        lmd.append(f"| {r['event']} | {r['index_peak']} → {r['index_-5pct']} ({r['sessions_peak_to_-5pct']}) | {cell(r['NDX_abs'])} | "
                   f"{cell(r['SOX_abs'])} | {cell(r['SOX_rs'])} | {cell(r['CAPvsEW_rs'])} | {cell(r['MEGA8_abs'])} | "
                   f"{fcell(r['fire_pick'])} | {fcell(r['fire_SOX rel63<=-8'])} | {fcell(r['fire_NDX rel63<=-8'])} |")
    leads = {k: [r[k]["lead_sessions"] for r in etab if r.get(k)] for k in ("NDX_abs", "SOX_abs", "SOX_rs", "CAPvsEW_rs")}
    lsum = {k: {"n": len(v), "led_by_>=10_sessions": sum(x >= 10 for x in v), "median_lead": float(np.median(v)) if v else None}
            for k, v in leads.items()}
    # null: on every S&P new 252-session closing high (outside drawdowns), how far below their own high were
    # the leaders, and had they peaked >= 10 sessions earlier? (is "leaders peaked first" special at real peaks?)
    newhi = (P["dd"] <= 0.0001)
    null = {}
    for nm, key_ in (("NDX", "ddL"), ("SOX", "ddL"), ("SOX_rs", "ddR"), ("CAPvsEW_rs", "ddR")):
        base_nm = nm.split("_")[0]
        x = feats[base_nm][key_]
        m = newhi & ~np.isnan(x)
        null[nm] = {"sp_new_high_days": int(m.sum()),
                    **{f"share_leaders_>={t}pct_below": float(np.mean(x[m] >= t)) for t in (5, 10, 15)}}
        pk_dd = [r.get(f"{base_nm}_{'rs' if key_ == 'ddR' else 'abs'}") for r in etab]
        pk_dd = [q["dd_at_index_peak"] for q in pk_dd if q and q["dd_at_index_peak"] is not None]
        null[nm]["at_the_9_or_10_real_peaks"] = {f">={t}pct_below": f"{sum(v >= t for v in pk_dd)}/{len(pk_dd)}" for t in (5, 10, 15)}
    lmd.append("\nnull check — on all S&P new-high days, share with leaders already below their own high, vs at the real peaks:\n")
    lmd.append("| leaders | S&P new-high days | ≥5% below | ≥10% | ≥15% | at real peaks ≥5 / ≥10 / ≥15% |\n|---|---|---|---|---|---|")
    for nm, v in null.items():
        q = v["at_the_9_or_10_real_peaks"]
        lmd.append(f"| {nm} | {v['sp_new_high_days']} | {v['share_leaders_>=5pct_below']:.2f} | {v['share_leaders_>=10pct_below']:.2f} | "
                   f"{v['share_leaders_>=15pct_below']:.2f} | {q['>=5pct_below']} / {q['>=10pct_below']} / {q['>=15pct_below']} |")
    lmd.append("\nleaders first by ≥10 sessions: " + "; ".join(f"{k} {v['led_by_>=10_sessions']}/{v['n']} (median lead {C.fmt(v['median_lead'], 0)})"
                                                     for k, v in lsum.items()))

    # leadership as a +1 add-on to the proposed drawdown score D (1997→, from calibrate.py's panel)
    dpath = SERIES_DIR / "_calibration_daily.csv"
    addon = {}
    if dpath.exists():
        dd_, Dv = [], []
        with open(dpath) as f:
            for row in csv.DictReader(f):
                dd_.append(row["date"]); Dv.append(int(row["proposed_drawdown"]))
        dd_ = np.array(dd_, dtype="datetime64[D]"); Dv = np.array(Dv, float)
        D = np.nan_to_num(align(dd_, Dv, dates), nan=0)
        M97 = masks(dates, "1997-01-02")
        cand = {"D>=3 (proposed warning)": D >= 3,
                f"D + [{overall['rule']}] >= 3": (D + s_over) >= 3,
                "D + [NDX rel63<=-8] >= 3": (D + s_pre2["NDX"]) >= 3,
                "D + [SOX rel63<=-8] >= 3": (D + s_pre2["SOX"]) >= 3,
                "D>=2 AND leadership pick": (D >= 2) & s_over}
        lmd.append("\n## leadership as a +1 add-on to the proposed drawdown score D (1997–2026)\n")
        for per in ("train", "test"):
            br = base_rate(lab6, M97[per])
            rows = [ev_row(nm, s, lab6, pwins(win6, per, "1997-06-01"), dates, M97[per], br) for nm, s in cand.items()]
            r5 = {nm: C.evaluate(s, lab6, pwins(win5, per, "1997-06-01"), dates, M97[per])["events_detected"] for nm, s in cand.items()}
            for r in rows:
                r["pre_-5pct"] = r5[r["signal"]]
            addon[per] = rows
            lmd.append(f"\n{per} (base rate {br:.2f}); pre −5% hits: " + "; ".join(f"{k}: {len(v)}" for k, v in r5.items()) + "\n")
            lmd += md_rows(rows)

    # current readings (for the AI-confidence design)
    cur = {"date": str(dates[-1]), "spx_dd_from_252d_high": round(float(P["dd"][-1]), 1)}
    for nm in proxies:
        f = feats[nm]
        cur[nm] = {k: (None if np.isnan(f[k][-1]) else round(float(f[k][-1]), 1)) for k in ("ddL", "ddR", "rel21", "rel63")}
        cur[nm]["div50_days_of_20"] = None if np.isnan(f["div50_20"][-1]) else int(round(f["div50_20"][-1] * 20))
    ai = {}
    smh = align(*read_yahoo("SMH"), dates)
    smh21 = (smh[-1] / smh[-22] - 1) * 100
    for s in AI_INFRA + ["NVDA", "SMH", "^FPMI", "DXYZ"]:
        try:
            d_, v_ = read_yahoo(s)
        except FileNotFoundError:
            continue
        x = align(d_, v_, dates)
        okk = np.flatnonzero(~np.isnan(x))
        if len(okk) < 30:
            continue
        xx = x[okk[0]:]
        h = np.nanmax(xx[-252:]); lo_ = np.nanmin(xx[-252:])
        r63 = (xx[-1] / xx[-64] - 1) * 100 if len(xx) > 64 else None
        s63 = (close[-1] / close[-64] - 1) * 100
        ai[s] = {"close": round(float(xx[-1]), 2), "dd_from_252d_high_pct": round(float((1 - xx[-1] / h) * 100), 1),
                 "above_252d_low_pct": round(float((xx[-1] / lo_ - 1) * 100), 1),
                 "rel63_vs_spx_pct": None if r63 is None else round(float((1 + r63 / 100) / (1 + s63 / 100) * 100 - 100), 1),
                 "ret21_vs_smh_pct": round(float(((xx[-1] / xx[-22]) / (1 + smh21 / 100) - 1) * 100), 1),
                 "history_from": str(dates[okk[0]])}
    vals = [v["dd_from_252d_high_pct"] for k, v in ai.items() if k in AI_INFRA]
    cur["ai_infra_basket_median_ret21_vs_smh"] = float(np.median([v["ret21_vs_smh_pct"] for k, v in ai.items() if k in AI_INFRA]))
    # the basket's own short history: how often would draft AI-core thresholds have been on? (no backtest possible)
    DDs, R21 = [], []
    for s in AI_INFRA:
        x = align(*read_yahoo(s), dates)
        okk = np.flatnonzero(~np.isnan(x))
        d_ = np.full(len(x), np.nan); r_ = np.full(len(x), np.nan)
        if len(okk) > 60:
            seg = x[okk[0]:]
            d_[okk[0]:] = (1 - seg / C.rolling_max(seg, 252)) * 100
            d_[okk[0]:okk[0] + 60] = np.nan
            r_[22:] = ((x[22:] / x[:-22]) / (smh[22:] / smh[:-22]) - 1) * 100
        DDs.append(d_); R21.append(r_)
    DDs, R21 = np.stack(DDs), np.stack(R21)
    nv = (~np.isnan(DDs)).sum(axis=0)
    hist_m = nv >= 7
    with np.errstate(all="ignore"), warnings.catch_warnings():
        warnings.simplefilter("ignore", RuntimeWarning)
        mdd = np.where(hist_m, np.nanmedian(np.where(np.isnan(DDs), np.nan, DDs), axis=0), np.nan)
        mr21 = np.where(hist_m, np.nanmedian(R21, axis=0), np.nan)
    hm = hist_m & ~np.isnan(mdd)
    cur["ai_infra_basket_history"] = {
        "from": str(dates[hm][0]) if hm.any() else None, "days": int(hm.sum()),
        "share_days_median_dd>=40": float(np.mean(mdd[hm] >= 40)), "share_days_median_dd>=50": float(np.mean(mdd[hm] >= 50)),
        "share_days_median_ret21_vs_smh<=-15": float(np.mean(mr21[hm] <= -15)),
        "share_days_median_ret21_vs_smh<=-20": float(np.mean(mr21[hm] <= -20)),
        "first_day_median_dd>=40": str(dates[hm][np.argmax(mdd[hm] >= 40)]) if (mdd[hm] >= 40).any() else None,
        "monthly_median_dd": {str(dates[i])[:7]: round(float(mdd[i]), 1) for i in me if hm[i]}}
    cur["ai_infra"] = ai
    cur["ai_infra_basket_median_dd"] = float(np.median(vals)) if vals else None
    cur["cape"] = valuation["cape_now"]; cur["buffett"] = valuation["buffett_now"]
    cur["leadership_rules_today"] = {"overall_pick": overall["rule"], "fires": bool(rule(overall["proxy"], overall["kind"], *overall["params"])[-1]),
                                     **{f"{nm}_R1_10_3": bool(s_pre1[nm][-1]) for nm in s_pre1},
                                     "SOX_rs_line_dd": cur["SOX"]["ddR"] if "ddR" in cur["SOX"] else None,
                                     **{f"{nm}_R2_8": bool(s_pre2[nm][-1]) for nm in s_pre2}}
    (OUT / "current_readings.json").write_text(json.dumps(cur, indent=1, default=str))
    for r in res:
        for per in ("train", "test", "full"):
            r[per]["false_episode_list"] = r[per]["false_episode_list"][:10]
    (OUT / "leadership.json").write_text(json.dumps({"meta": meta, "grid": res, "best_by_family": [r["rule"] for r in fam],
                                                     "overall_pick": overall["rule"], "episodes": etab, "lead_summary": lsum, "null_new_high_days": null,
                                                     "d_addon": addon}, indent=1, default=str))
    (OUT / "leadership.md").write_text("\n".join(lmd) + "\n")
    print(json.dumps({"cape": valuation["cape_now"], "overall_pick": overall["rule"],
                      "pick_train": overall["train"]["objective"], "pick_test": overall["test"]["objective"],
                      "today": cur["leadership_rules_today"]}, default=str), file=sys.stderr)
    print(f"wrote {OUT}/valuation.*, leadership.*, current_readings.json", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
