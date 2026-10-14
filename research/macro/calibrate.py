#!/usr/bin/env python3
"""recession-watch calibration study (research only — never read by the live rubric).

question: with hindsight, which market/macro signals preceded (A) NBER recession starts within
12 months and (B) >= 15 % / >= 20 % S&P 500 drawdowns starting within 6 months, versus false
alarms — and how should the daily 0-10 score be built?

steps (each writes research/macro/results/<name>.json and, where useful, a .md table):
  1. reconstruct the CURRENT rubric (scripts/macro/fetch_indicators.py) daily, point-in-time
  2. episode table (recessions, drawdowns, notable false alarms)
  3. precision / recall / lead / false-alarm episodes per component and composite, both targets;
     integer-weight grid, fit 1997-2015, evaluated 2016-2026
  4. faster drop signals the rubric lacks, same framework

usage:
    python3 research/macro/calibrate.py            # fetch missing series (cache), run everything
    python3 research/macro/calibrate.py --refresh  # refetch every series

data cache: data/macro/series/ (gitignored). this script only ADDS files there (new series ids,
GSPC_yahoo.csv, BAMLH0A0HYM2_wayback_20231212.txt) and refreshes a live-series file only when it
is older than 20 h, in the same raw FRED format the live script writes — the live cache stays valid.

python3 stdlib + numpy.
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import io
import itertools
import json
import sys
import time
import urllib.request
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SERIES_DIR = ROOT / "data" / "macro" / "series"
OUT = ROOT / "research" / "macro" / "results"
SWING_DAILY = ROOT / "research" / "swing" / "daily"

FRED_URL = "https://fred.stlouisfed.org/graph/fredgraph.csv?id={}"
FRED_IDS = ["T10Y3M", "T10Y2Y", "BAMLH0A0HYM2", "SAHMREALTIME", "ICSA", "UNRATE", "VIXCLS",
            "VXVCLS", "DGS10", "DFF", "USREC", "UMCSENT", "PERMIT", "AWHMAN", "NFCI", "STLFSI4",
            "T10YIE", "BAA10Y", "CPIAUCSL", "SP500"]
# FRED serves only the last ~3 years of ICE BofA OAS since 2023 (licensing). the full history
# 1996-12-31 .. 2023-12-11 comes from the Internet Archive's copy of FRED's own text download.
HY_WAYBACK = ("http://web.archive.org/web/20231212182755id_/"
              "https://fred.stlouisfed.org/data/BAMLH0A0HYM2.txt")
YAHOO_SPX = ("https://query1.finance.yahoo.com/v8/finance/chart/%5EGSPC"
             "?period1=631152000&period2={}&interval=1d")

# publication lag (calendar days after the observation date before a value is usable).
# daily market series: 1 day (the 05:45 PT run sees yesterday's close).
# weekly ICSA (week ending Sat, released Thu): 5. NFCI (week ending Fri, released Wed): 5.
# STLFSI4 (week ending Fri, released Thu): 6. monthly obs dated the 1st: released next month.
LAG_DAYS = {"T10Y3M": 1, "T10Y2Y": 1, "HY": 1, "VIXCLS": 1, "VXVCLS": 1, "DGS10": 1, "DFF": 1,
            "BAA10Y": 1, "T10YIE": 1, "ICSA": 5, "NFCI": 5, "STLFSI4": 6,
            "SAHMREALTIME": 38, "UNRATE": 38, "AWHMAN": 38, "UMCSENT": 31, "PERMIT": 49,
            "CPIAUCSL": 45}

START = dt.date(1997, 1, 2)
TRAIN_END = dt.date(2015, 12, 31)


# ---------------------------------------------------------------- fetch / cache

def _http(url: str, timeout: int = 90, tries: int = 4) -> str:
    last = None
    for i in range(tries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=timeout) as r:
                return r.read().decode("utf-8", errors="replace")
        except Exception as e:  # noqa: BLE001 — flaky endpoints, retry
            last = e
            time.sleep(3 * (i + 1))
    raise RuntimeError(f"fetch failed {url}: {last}")


def _stale(p: Path, refresh: bool) -> bool:
    return refresh or not p.exists() or (time.time() - p.stat().st_mtime) > 7 * 86400


def fetch_all(refresh: bool) -> dict:
    SERIES_DIR.mkdir(parents=True, exist_ok=True)
    status = {}
    for sid in FRED_IDS:
        p = SERIES_DIR / f"{sid}.csv"
        # a study does not need hour-fresh data: keep any cache younger than a week
        if refresh or not p.exists() or (time.time() - p.stat().st_mtime) > 7 * 86400:
            try:
                txt = _http(FRED_URL.format(sid))
                if txt.startswith("observation_date") or txt.startswith("DATE"):
                    p.write_text(txt)
                    status[sid] = "fetched"
                else:
                    status[sid] = "bad payload, kept cache" if p.exists() else "n/a"
            except RuntimeError as e:
                status[sid] = f"failed ({e}); cache" if p.exists() else "n/a"
        else:
            status[sid] = "cached"
    p = SERIES_DIR / "BAMLH0A0HYM2_wayback_20231212.txt"
    if not p.exists():
        p.write_text(_http(HY_WAYBACK, timeout=180))
        status["HY_wayback"] = "fetched"
    else:
        status["HY_wayback"] = "cached"
    p = SERIES_DIR / "GSPC_yahoo.csv"
    if _stale(p, refresh):
        try:
            j = json.loads(_http(YAHOO_SPX.format(int(time.time()) + 86400)))
            res = j["chart"]["result"][0]
            ts, cl = res["timestamp"], res["indicators"]["quote"][0]["close"]
            rows = ["date,close"]
            for t, c in zip(ts, cl):
                if c is None:
                    continue
                d = (dt.datetime.fromtimestamp(t, dt.timezone.utc) - dt.timedelta(hours=5)).date()
                rows.append(f"{d.isoformat()},{c:.4f}")
            p.write_text("\n".join(rows) + "\n")
            status["GSPC_yahoo"] = "fetched"
        except Exception as e:  # noqa: BLE001
            status["GSPC_yahoo"] = f"failed ({e}); cache" if p.exists() else "n/a"
    else:
        status["GSPC_yahoo"] = "cached"
    return status


def read_fred(sid: str) -> tuple[np.ndarray, np.ndarray]:
    """(dates as datetime64[D], values float with nan)"""
    p = SERIES_DIR / f"{sid}.csv"
    d, v = [], []
    with open(p, newline="") as f:
        r = csv.reader(f)
        next(r, None)
        for row in r:
            if len(row) < 2 or not row[0]:
                continue
            try:
                x = float(row[1])
            except ValueError:
                continue
            d.append(row[0]); v.append(x)
    return np.array(d, dtype="datetime64[D]"), np.array(v, dtype=float)


def read_hy() -> tuple[np.ndarray, np.ndarray, dict]:
    """spliced HY OAS in percent: wayback history up to the first current-FRED date, FRED after."""
    txt = (SERIES_DIR / "BAMLH0A0HYM2_wayback_20231212.txt").read_text().splitlines()
    wd, wv = [], []
    started = False
    for line in txt:
        if line.startswith("DATE"):
            started = True
            continue
        if not started:
            continue
        parts = line.split()
        if len(parts) == 2:
            try:
                wv.append(float(parts[1])); wd.append(parts[0])
            except ValueError:
                pass
    wd = np.array(wd, dtype="datetime64[D]"); wv = np.array(wv)
    fd, fv = read_fred("BAMLH0A0HYM2")
    ov = np.intersect1d(wd, fd)
    diffs = [abs(wv[wd == x][0] - fv[fd == x][0]) for x in ov]
    cut = fd[0]
    d = np.concatenate([wd[wd < cut], fd]); v = np.concatenate([wv[wd < cut], fv])
    meta = {"wayback_range": [str(wd[0]), str(wd[-1])], "fred_range": [str(fd[0]), str(fd[-1])],
            "overlap_days": int(len(ov)),
            "overlap_max_abs_diff_pct": float(max(diffs)) if diffs else None,
            "overlap_mean_abs_diff_pct": float(np.mean(diffs)) if diffs else None}
    return d, v, meta


def read_spx() -> tuple[np.ndarray, np.ndarray]:
    d, v = [], []
    with open(SERIES_DIR / "GSPC_yahoo.csv") as f:
        next(f)
        for line in f:
            a, b = line.strip().split(",")
            d.append(a); v.append(float(b))
    d = np.array(d, dtype="datetime64[D]"); v = np.array(v)
    keep = np.concatenate([d[1:] != d[:-1], [True]])
    d, v = d[keep], v[keep]
    today = np.datetime64(dt.date.today().isoformat())
    return d[d < today], v[d < today]  # drop today's in-progress bar


def asof(src_d: np.ndarray, src_v: np.ndarray, dates: np.ndarray, lag: int) -> np.ndarray:
    """value usable on each of `dates`: latest obs with obs_date + lag <= date (nan before)."""
    avail = src_d + np.timedelta64(lag, "D")
    idx = np.searchsorted(avail, dates, side="right") - 1
    out = np.full(len(dates), np.nan)
    ok = idx >= 0
    out[ok] = src_v[idx[ok]]
    return out


def asof_idx(src_d: np.ndarray, dates: np.ndarray, lag: int) -> np.ndarray:
    return np.searchsorted(src_d + np.timedelta64(lag, "D"), dates, side="right") - 1


# ---------------------------------------------------------------- panel

def rolling_mean(x: np.ndarray, n: int) -> np.ndarray:
    out = np.full(len(x), np.nan)
    c = np.cumsum(np.insert(x, 0, 0.0))
    out[n - 1:] = (c[n:] - c[:-n]) / n
    return out


def rolling_max(x: np.ndarray, n: int) -> np.ndarray:
    from numpy.lib.stride_tricks import sliding_window_view
    out = np.full(len(x), np.nan)
    if len(x) >= n:
        out[n - 1:] = sliding_window_view(x, n).max(axis=1)
    # the live script uses the shorter window while history < n
    for i in range(min(n - 1, len(x))):
        out[i] = x[:i + 1].max()
    return out


def build_panel() -> dict:
    """every series aligned to the SPX trading calendar, point in time."""
    sd, sv = read_spx()
    P = {"spx_dates_all": sd, "spx_all": sv}
    cal = sd[sd >= np.datetime64(START)]
    P["dates"] = cal
    off = int(np.searchsorted(sd, cal[0]))
    P["off"] = off
    raw = {}
    for sid in ["T10Y3M", "T10Y2Y", "SAHMREALTIME", "ICSA", "UNRATE", "VIXCLS", "VXVCLS", "NFCI",
                "STLFSI4", "PERMIT", "UMCSENT", "AWHMAN", "BAA10Y", "USREC", "DFF", "DGS10"]:
        raw[sid] = read_fred(sid)
    hd, hv, hmeta = read_hy()
    raw["HY"] = (hd, hv)
    P["raw"] = raw
    P["hy_meta"] = hmeta
    return P


# ---------------------------------------------------------------- current rubric, point in time

def comp_yield_curve(P) -> tuple[np.ndarray, np.ndarray]:
    d, v = P["raw"]["T10Y3M"]
    n = len(d)
    score = np.zeros(n, dtype=int)
    run_start = None
    last_big_end = None  # end date of the most recent completed >=90d inversion streak
    for j in range(n):
        if v[j] < 0:
            if run_start is None:
                run_start = d[j]
            streak = int((d[j] - run_start).astype(int))
            score[j] = 2 if streak >= 90 else 1
        else:
            if run_start is not None:
                prev_end = d[j - 1]
                if int((prev_end - run_start).astype(int)) >= 90:
                    last_big_end = prev_end
                run_start = None
            if last_big_end is not None and int((d[j] - last_big_end).astype(int)) <= 183:
                score[j] = 2
            elif abs(v[j]) < 0.25:
                score[j] = 1
    idx = asof_idx(d, P["dates"], LAG_DAYS["T10Y3M"])
    return np.where(idx >= 0, score[np.maximum(idx, 0)], 0), np.where(idx >= 0, v[np.maximum(idx, 0)], np.nan)


def comp_credit(P) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    d, v = P["raw"]["HY"]
    bps = v * 100.0
    j90 = np.searchsorted(d, d - np.timedelta64(90, "D"), side="right") - 1
    delta = np.where(j90 >= 0, bps - bps[np.maximum(j90, 0)], np.nan)
    sc = np.where(bps >= 500, 2, np.where((bps >= 400) | (np.nan_to_num(delta) >= 100), 1, 0))
    idx = asof_idx(d, P["dates"], LAG_DAYS["HY"])
    ok = idx >= 0
    g = lambda a: np.where(ok, a[np.maximum(idx, 0)], np.nan)  # noqa: E731
    return np.where(ok, sc[np.maximum(idx, 0)], 0).astype(int), g(bps), g(delta)


def claims_4wk(P):
    d, v = P["raw"]["ICSA"]
    avg = rolling_mean(v, 4)
    jy = np.searchsorted(d, d - np.timedelta64(364, "D"), side="right") - 1
    prior = np.where(jy >= 3, avg[np.maximum(jy, 0)], np.nan)
    yoy = (avg - prior) / prior * 100
    # "Sahm-like" claims rule: 4wk average vs its own minimum over the prior 52 weeks
    from numpy.lib.stride_tricks import sliding_window_view
    mn = np.full(len(avg), np.nan)
    w = sliding_window_view(np.nan_to_num(avg, nan=np.inf), 52).min(axis=1)
    mn[51:] = w
    rise = (avg / mn - 1) * 100
    return d, avg, yoy, rise


def comp_labor(P):
    sd_, sv_ = P["raw"]["SAHMREALTIME"]
    sahm = asof(sd_, sv_, P["dates"], LAG_DAYS["SAHMREALTIME"])
    s_sc = np.where(sahm >= 0.5, 2, np.where(sahm >= 0.3, 1, 0))
    cd, avg, yoy, rise = claims_4wk(P)
    cy = asof(cd, yoy, P["dates"], LAG_DAYS["ICSA"])
    add = (np.nan_to_num(cy, nan=-1) >= 20).astype(int)
    return np.minimum(2, s_sc + add), sahm, cy


def spx_features(P):
    sv = P["spx_all"]
    hi = rolling_max(sv, 252)
    dd = (hi - sv) / hi * 100
    sma200 = rolling_mean(sv, 200)
    below = sv < sma200
    run = np.zeros(len(sv), dtype=int)
    for i in range(len(sv)):
        run[i] = run[i - 1] + 1 if (below[i] and i > 0) else (1 if below[i] else 0)
    sma50 = rolling_mean(sv, 50)
    lr = np.diff(np.log(sv), prepend=np.nan)
    rv20 = np.full(len(sv), np.nan)
    from numpy.lib.stride_tricks import sliding_window_view
    rv20[20:] = sliding_window_view(lr[1:], 20).std(axis=1, ddof=1) * np.sqrt(252)
    ret21 = np.full(len(sv), np.nan)
    ret21[21:] = sv[21:] / sv[:-21] - 1
    above = (sv > sma200).astype(float)
    above[np.isnan(sma200)] = np.nan
    share50 = rolling_mean(np.nan_to_num(above, nan=1.0), 50)
    o = P["off"]
    return {"dd": dd[o:], "below_run": run[o:], "dist50": (sv / sma50 - 1)[o:] * 100,
            "rv20": rv20[o:] * 100, "ret21": ret21[o:] * 100, "share_above200_50d": share50[o:],
            "close": sv[o:]}


def comp_equity(P, F):
    vix = asof(*P["raw"]["VIXCLS"], P["dates"], LAG_DAYS["VIXCLS"])
    vxv = asof(*P["raw"]["VXVCLS"], P["dates"], LAG_DAYS["VXVCLS"])
    base = np.where(F["dd"] >= 15, 2, np.where((F["dd"] >= 8) | (F["below_run"] > 20), 1, 0))
    vadd = (np.nan_to_num(vix) >= 30).astype(int)
    tadd = (~np.isnan(vxv) & ~np.isnan(vix) & (vxv < vix)).astype(int)
    return np.minimum(2, base + vadd + tadd), vix, vxv


def comp_breadth_proxy(F):
    """PROXY: share of the last 50 sessions the S&P 500 itself closed above its 200-day.
    the real component (59 names above their 200-day) has no history before 2021."""
    s = F["share_above200_50d"]
    return np.where(s < 0.20, 2, np.where(s < 0.30, 1, 0))


def real_breadth(P):
    """real 59-name breadth from research/swing/daily (2021-06 onward) on the panel calendar."""
    out = np.full(len(P["dates"]), np.nan)
    if not SWING_DAILY.exists():
        return out
    cnt = np.zeros(len(P["dates"])); tot = np.zeros(len(P["dates"]))
    for p in SWING_DAILY.glob("*.csv"):
        if p.stem == "SPY":
            continue
        d, c = [], []
        with open(p) as f:
            for row in csv.DictReader(f):
                d.append(row["date"]); c.append(float(row["close"]))
        d = np.array(d, dtype="datetime64[D]"); c = np.array(c)
        sma = rolling_mean(c, 200)
        ok = ~np.isnan(sma)
        idx = np.searchsorted(d, P["dates"], side="right") - 1
        valid = (idx >= 0) & (P["dates"] <= d[-1])
        valid &= ok[np.maximum(idx, 0)]
        ab = c[np.maximum(idx, 0)] > sma[np.maximum(idx, 0)]
        tot += valid; cnt += valid & ab
    m = tot >= 30
    out[m] = cnt[m] / tot[m] * 100
    return out


# ---------------------------------------------------------------- extra (faster / leading) signals

def extra_signals(P, F, hy_bps):
    dates = P["dates"]
    X = {}
    # HY OAS 1-month (21 trading days) change, bps
    hy1m = np.full(len(dates), np.nan); hy1m[21:] = hy_bps[21:] - hy_bps[:-21]
    X["hy_chg_1m"] = hy1m
    X["hy_level"] = hy_bps
    X["nfci"] = asof(*P["raw"]["NFCI"], dates, LAG_DAYS["NFCI"])
    nd, nv = P["raw"]["NFCI"]
    nchg = np.full(len(nv), np.nan); nchg[4:] = nv[4:] - nv[:-4]
    X["nfci_chg_4w"] = asof(nd, nchg, dates, LAG_DAYS["NFCI"])
    X["stlfsi"] = asof(*P["raw"]["STLFSI4"], dates, LAG_DAYS["STLFSI4"])
    vix = asof(*P["raw"]["VIXCLS"], dates, 1); vxv = asof(*P["raw"]["VXVCLS"], dates, 1)
    X["vix"] = vix
    X["vix_term"] = vix / vxv  # >1 = inverted (2007-12 onward only)
    X["dist50"] = F["dist50"]; X["ret21"] = F["ret21"]; X["rv20"] = F["rv20"]; X["dd"] = F["dd"]
    cd, avg, yoy, rise = claims_4wk(P)
    X["claims_rise"] = asof(cd, rise, dates, LAG_DAYS["ICSA"])
    pdv, pv = P["raw"]["PERMIT"]
    pyoy = np.full(len(pv), np.nan); pyoy[12:] = (pv[12:] / pv[:-12] - 1) * 100
    X["permits_yoy"] = asof(pdv, pyoy, dates, LAG_DAYS["PERMIT"])
    ud, uv = P["raw"]["UMCSENT"]
    uyoy = np.full(len(uv), np.nan); uyoy[12:] = uv[12:] - uv[:-12]
    X["umcsent_yoy"] = asof(ud, uyoy, dates, LAG_DAYS["UMCSENT"])
    X["t10y2y"] = asof(*P["raw"]["T10Y2Y"], dates, 1)
    # robust curve: 63-session average of T10Y3M (a one-day un-inversion cannot reset it)
    td, tv = P["raw"]["T10Y3M"]
    t63 = asof(td, rolling_mean(tv, 63), dates, 1)
    X["t10y3m_avg63"] = t63
    # inverted on a 63-session average at any point in the last 12 months
    inv = np.nan_to_num(t63, nan=1) < 0
    rec = np.zeros(len(dates), bool)
    last = None
    for i in range(len(dates)):
        if inv[i]:
            last = dates[i]
        rec[i] = last is not None and (dates[i] - last) <= np.timedelta64(365, "D")
    X["curve_inv_within_12m"] = rec
    # credit vs its own trailing 1-year minimum (bps)
    from numpy.lib.stride_tricks import sliding_window_view
    hd, hv = P["raw"]["HY"]
    hb = hv * 100
    mn = np.full(len(hb), np.nan); mn[251:] = sliding_window_view(hb, 252).min(axis=1)
    X["hy_rise_vs_1y_min"] = asof(hd, hb - mn, dates, 1)
    X["baa10y"] = asof(*P["raw"]["BAA10Y"], dates, 1)
    return X


# (name, raw key, direction, threshold for 1, threshold for 2) — a-priori, literature-style cut-offs
EXTRA_DEFS = [
    ("hy_chg_1m", "hy_chg_1m", +1, 50, 100),      # bps widening over 21 sessions
    ("hy_rise_1y", "hy_rise_vs_1y_min", +1, 100, 200),  # bps above the trailing 252-session low
    ("nfci", "nfci", +1, -0.25, 0.0),             # Chicago Fed NFCI (>0 = tighter than average)
    ("nfci_chg_4w", "nfci_chg_4w", +1, 0.10, 0.25),
    ("stlfsi", "stlfsi", +1, 0.0, 1.0),
    ("vix_term", "vix_term", +1, 1.0, 1.10),       # VIX / VIX3M, 2007-12 onward
    ("spx_dist50", "dist50", -1, -5.0, -8.0),       # % below 50-day
    ("spx_ret21", "ret21", -1, -5.0, -10.0),        # 1-month return %
    ("rv20", "rv20", +1, 20.0, 30.0),               # 20-day realized vol, annualized %
    ("claims_rise", "claims_rise", +1, 15.0, 25.0),  # 4wk avg vs prior-52w min, %
    ("permits_yoy", "permits_yoy", -1, -15.0, -25.0),
    ("umcsent_yoy", "umcsent_yoy", -1, -10.0, -20.0),
]


def to_score(x: np.ndarray, sign: int, t1: float, t2: float) -> np.ndarray:
    y = np.nan_to_num(x * sign, nan=-np.inf)
    return np.where(y >= t2 * sign, 2, np.where(y >= t1 * sign, 1, 0)).astype(int)


# ---------------------------------------------------------------- targets

def recession_starts(P) -> list[np.datetime64]:
    d, v = P["raw"]["USREC"]
    return [d[i] for i in range(1, len(v)) if v[i] == 1 and v[i - 1] == 0]


def drawdown_episodes(P, thr: float, merge_gap: int = 126) -> list[dict]:
    sd, sv = P["spx_dates_all"], P["spx_all"]
    hi = rolling_max(sv, 252)
    dd = (hi - sv) / hi * 100
    on = dd >= thr
    eps = []
    i = 0
    n = len(sv)
    while i < n:
        if on[i]:
            j = i
            while j + 1 < n and on[j + 1]:
                j += 1
            eps.append([i, j])
            i = j + 1
        else:
            i += 1
    merged = []
    for e in eps:
        if merged and e[0] - merged[-1][1] <= merge_gap:
            merged[-1][1] = e[1]
        else:
            merged.append(e)
    out = []
    for c, e in merged:
        lo = max(0, c - 251)
        pk = lo + int(np.argmax(sv[lo:c + 1]))
        tr = c + int(np.argmin(sv[c:e + 1]))
        out.append({"peak": sd[pk], "cross": sd[c], "trough": sd[tr], "end": sd[e],
                    "max_dd": float(dd[c:e + 1].max())})
    return out


def labels_recession(P, starts) -> np.ndarray:
    dates = P["dates"]
    ud, uv = P["raw"]["USREC"]
    lab = np.zeros(len(dates))
    for s in starts:
        lab[(dates < s) & (dates >= s - np.timedelta64(365, "D"))] = 1
    inrec = asof(ud, uv, dates, 0) == 1
    lab[inrec] = np.nan
    last_known = ud[-1] - np.timedelta64(365, "D")
    lab[(dates > last_known) & (lab != 1)] = np.nan
    return lab


def labels_drawdown(P, eps) -> np.ndarray:
    dates = P["dates"]
    lab = np.zeros(len(dates))
    for e in eps:
        lab[(dates >= e["peak"] - np.timedelta64(182, "D")) & (dates < e["cross"])] = 1
    for e in eps:
        lab[(dates >= e["cross"]) & (dates <= e["end"])] = np.nan
    last_known = dates[-1] - np.timedelta64(182, "D")
    lab[(dates > last_known) & (lab != 1)] = np.nan
    return lab


def event_windows(P, target: str, events) -> list[dict]:
    """per event: index window [ws, we) in which an alarm counts as a warning, and the
    reference dates for lead time."""
    dates = P["dates"]
    out = []
    for e in events:
        if target == "A":
            ws = np.searchsorted(dates, e - np.timedelta64(365, "D"))
            we = np.searchsorted(dates, e)
            out.append({"name": str(e)[:7], "ws": ws, "we": we, "ref": e, "ref2": e})
        else:
            # a warning must come >= 14 calendar days before the threshold is breached
            ws = np.searchsorted(dates, e["peak"] - np.timedelta64(182, "D"))
            we = np.searchsorted(dates, e["cross"] - np.timedelta64(14, "D"))
            wpk = np.searchsorted(dates, e["peak"])
            out.append({"name": str(e["peak"])[:7], "ws": ws, "we": we, "wpk": wpk, "ref": e["cross"],
                        "ref2": e["peak"]})
    return [w for w in out if w["we"] > 0 and w["ws"] < len(dates)]


# ---------------------------------------------------------------- evaluation

def evaluate(alarm: np.ndarray, lab: np.ndarray, wins: list[dict], dates: np.ndarray,
             mask: np.ndarray | None = None, gap: int = 21) -> dict:
    """alarm: bool per day. lab: 1/0/nan. wins: event windows. mask: restrict to a period."""
    if mask is None:
        mask = np.ones(len(alarm), bool)
    a = alarm & mask
    lab_ok = ~np.isnan(lab) & mask
    pos = lab_ok & (lab == 1); neg = lab_ok & (lab == 0)
    tp = int((a & pos).sum()); fp = int((a & neg).sum())
    day_prec = tp / (tp + fp) if tp + fp else None
    day_rec = tp / int(pos.sum()) if pos.sum() else None
    # episodes
    idx = np.flatnonzero(a)
    n_true = n_false = n_late = 0
    false_eps = []
    if len(idx):
        brk = np.flatnonzero(np.diff(idx) > gap)
        starts = np.concatenate([[idx[0]], idx[brk + 1]])
        ends = np.concatenate([idx[brk], [idx[-1]]])
        cpos = np.concatenate([[0], np.cumsum(pos)]); cneg = np.concatenate([[0], np.cumsum(neg)])
        for s, e in zip(starts, ends):
            npos = cpos[e + 1] - cpos[s]; nneg = cneg[e + 1] - cneg[s]
            if npos > 0:
                n_true += 1
            elif np.isnan(lab[s]) or not mask[s]:
                n_late += 1  # started inside an event (or outside the period): late, not false
            elif nneg > 0:
                n_false += 1
                false_eps.append([str(dates[s]), str(dates[e])])
            else:
                n_late += 1
    ep_prec = n_true / (n_true + n_false) if (n_true + n_false) else None
    # events
    det, leads, leads2, prepeak = [], [], [], []
    for w in wins:  # caller passes only the events of the period
        seg = alarm[w["ws"]:w["we"]]
        if alarm[w["ws"]:w.get("wpk", w["we"])].any():
            prepeak.append(w["name"])
        if seg.any():
            f = w["ws"] + int(np.argmax(seg))
            det.append(w["name"])
            leads.append(int((w["ref"] - dates[f]).astype(int)))
            leads2.append(int((w["ref2"] - dates[f]).astype(int)))
        else:
            det.append(None)
    n_ev = len(det)
    hit = [x for x in det if x]
    ev_rec = len(hit) / n_ev if n_ev else None
    return {"day_precision": day_prec, "day_recall": day_rec, "alarm_day_share": float(a.sum() / max(mask.sum(), 1)),
            "events": n_ev, "events_detected": hit, "event_recall": ev_rec, "detected_before_peak": prepeak,
            "lead_days_to_event": leads, "lead_days_to_peak": leads2,
            "median_lead_days": float(np.median(leads)) if leads else None,
            "true_episodes": n_true, "false_episodes": n_false, "late_episodes": n_late,
            "episode_precision": ep_prec, "false_episode_list": false_eps}


def objective(r: dict) -> float:
    if not r["event_recall"] or r["episode_precision"] is None:
        return 0.0
    return r["event_recall"] * r["episode_precision"]


# ---------------------------------------------------------------- study

def fmt(x, nd=2):
    if x is None:
        return "–"
    if isinstance(x, float):
        return f"{x:.{nd}f}"
    return str(x)


def months(days):
    return None if days is None else round(days / 30.44, 1)


def summarize(r: dict) -> dict:
    keep = ["day_precision", "day_recall", "alarm_day_share", "events", "events_detected", "detected_before_peak",
            "event_recall", "lead_days_to_event", "lead_days_to_peak", "median_lead_days",
            "true_episodes", "false_episodes", "late_episodes", "episode_precision",
            "false_episode_list"]
    return {k: r[k] for k in keep}


def period_masks(dates):
    tr = dates <= np.datetime64(TRAIN_END)
    return {"full": np.ones(len(dates), bool), "train": tr, "test": ~tr}


def period_wins(wins, period):
    if period == "full":
        return wins
    cut = np.datetime64(TRAIN_END)
    return [w for w in wins if (w["ref2"] <= cut) == (period == "train")]


def table_row(name, r, base_rate=None):
    lift = (r["day_precision"] / base_rate) if (base_rate and r["day_precision"] is not None) else None
    return (f"| {name} | {fmt(r['day_precision'])} ({fmt(lift, 1)}x) | {fmt(r['day_recall'])} | "
            f"{len(r['events_detected'])}/{r['events']} | "
            f"{fmt(months(r['median_lead_days']), 1)} | {r['false_episodes']} | {r['true_episodes']} | "
            f"{fmt(r['episode_precision'])} | {fmt(r['alarm_day_share'])} |")


TABLE_HEAD = ("| signal | day prec (lift vs base rate) | day recall | events hit | median lead (mo) | false episodes | "
              "true episodes | episode prec | share of days on |\n|---|---|---|---|---|---|---|---|---|")


def grid_search(comps: dict, names: list[str], lab, dates, train, test, wins_tr, wins_te,
                min_k: int = 2):
    """integer weights 0-2 per component, warn threshold k >= min_k chosen on train by
    objective = event recall x episode precision; ties broken by longer median lead, then fewer
    components, then fewer alarm days. crit threshold = the most episode-precise higher threshold
    that still hits >= half the train events. also reports how the whole tied-best set fares
    out of sample (a direct measure of how under-determined the fit is)."""
    C = np.stack([comps[n] for n in names]).astype(np.int16)
    scored = []
    best_f1 = None  # alternative pre-declared objective: day-level F1 on train
    for w in itertools.product((0, 1, 2), repeat=len(names)):
        if sum(w) == 0:
            continue
        raw = (np.array(w, dtype=np.int16)[:, None] * C).sum(axis=0)
        bestk = None
        for k in range(min_k, int(raw.max()) + 1):
            r = evaluate(raw >= k, lab, wins_tr, dates, train)
            pr, rc = r["day_precision"] or 0, r["day_recall"] or 0
            f1 = 2 * pr * rc / (pr + rc) if pr + rc else 0
            if best_f1 is None or (f1, -sum(1 for x in w if x)) > best_f1[0]:
                best_f1 = ((f1, -sum(1 for x in w if x)), w, k)
            lead = float(np.median(r["lead_days_to_event"])) if r["lead_days_to_event"] else 0.0
            key = (round(objective(r), 6), lead, -sum(1 for x in w if x), -r["alarm_day_share"])
            if bestk is None or key > bestk[0]:
                bestk = (key, k, r)
        if bestk:
            scored.append((bestk[0], w, bestk[1], bestk[2]))
    scored.sort(key=lambda t: t[0], reverse=True)
    key, w, k, rtr = scored[0]
    ties = [t for t in scored if t[0][0] == key[0]]
    tie_test = []
    for _, wt, kt, _ in ties:
        raw = (np.array(wt, dtype=np.int16)[:, None] * C).sum(axis=0)
        tie_test.append(objective(evaluate(raw >= kt, lab, wins_te, dates, test)))
    raw = (np.array(w, dtype=np.int16)[:, None] * C).sum(axis=0)
    crit = None
    for kk in range(k + 1, int(raw.max()) + 1):
        rr = evaluate(raw >= kk, lab, wins_tr, dates, train)
        if rr["event_recall"] and rr["event_recall"] >= 0.5:
            if crit is None or (rr["episode_precision"] or 0) > (crit[1]["episode_precision"] or 0):
                crit = (kk, rr)
    rte = evaluate(raw >= k, lab, wins_te, dates, test)
    out = {"components": names, "weights": dict(zip(names, w)), "max_points": int(2 * sum(w)),
           "warn_threshold": k, "train_objective": key[0], "test_objective": objective(rte),
           "train_warn": summarize(rtr), "test_warn": summarize(rte),
           "tied_best_in_train": len(ties),
           "tied_set_test_objective": {"min": float(np.min(tie_test)), "median": float(np.median(tie_test)),
                                       "max": float(np.max(tie_test)),
                                       "share_test_objective_ge_0.5": float(np.mean(np.array(tie_test) >= 0.5))}}
    if crit:
        out["crit_threshold"] = crit[0]
        out["train_crit"] = summarize(crit[1])
        out["test_crit"] = summarize(evaluate(raw >= crit[0], lab, wins_te, dates, test))
    (f1, _), wf, kf = best_f1
    rawf = (np.array(wf, dtype=np.int16)[:, None] * C).sum(axis=0)
    rf_tr = evaluate(rawf >= kf, lab, wins_tr, dates, train)
    rf_te = evaluate(rawf >= kf, lab, wins_te, dates, test)
    def _f1(r):
        pr, rc = r["day_precision"] or 0, r["day_recall"] or 0
        return 2 * pr * rc / (pr + rc) if pr + rc else 0.0
    out["day_f1_variant"] = {"weights": dict(zip(names, wf)), "threshold": kf,
                             "train_day_f1": _f1(rf_tr), "test_day_f1": _f1(rf_te),
                             "train_objective": objective(rf_tr), "test_objective": objective(rf_te),
                             "train": summarize(rf_tr), "test": summarize(rf_te)}
    out["_raw"] = raw
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--refresh", action="store_true")
    ap.add_argument("--no-fetch", action="store_true")
    args = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)
    fetch_status = {} if args.no_fetch else fetch_all(args.refresh)

    P = build_panel(); F = spx_features(P); dates = P["dates"]
    yc, ycv = comp_yield_curve(P)
    cr, hy, hyd = comp_credit(P)
    lb, sahm, cy = comp_labor(P)
    eq, vix, vxv = comp_equity(P, F)
    br = comp_breadth_proxy(F)
    base = np.minimum(8, yc + cr + lb + eq + br)
    cur = {"yield_curve": yc, "credit": cr, "labor": lb, "equity_stress": eq, "breadth_proxy": br}
    X = extra_signals(P, F, hy)
    extra = {n: to_score(X[k], s, t1, t2) for n, k, s, t1, t2 in EXTRA_DEFS}
    # robust curve variant: 2 = 63-session average inverted now or within the last 12 months;
    # 1 = inverted today or flat (< 25 bps) — compare the live streak rule
    ycv2 = np.where(X["curve_inv_within_12m"], 2, np.where(np.nan_to_num(ycv, nan=1) < 0.25, 1, 0))
    extra = {"curve_avg63": ycv2.astype(int), **extra}
    allc = {**cur, **extra}

    rb = real_breadth(P); m = ~np.isnan(rb)
    rbs = np.where(rb < 20, 2, np.where(rb < 30, 1, 0))
    meta = {"generated": dt.datetime.now().isoformat(timespec="seconds"), "fetch": fetch_status,
            "calendar": [str(dates[0]), str(dates[-1]), int(len(dates))], "hy_splice": P["hy_meta"],
            "breadth_proxy_check": {"days": int(m.sum()), "from": str(dates[m][0]) if m.any() else None,
                                    "score_agreement": float(np.mean(rbs[m] == br[m])) if m.any() else None,
                                    "corr_pct_above200_vs_proxy_share": float(np.corrcoef(rb[m], F["share_above200_50d"][m])[0, 1]) if m.any() else None},
            "series_ranges": {k: [str(v[0][0]), str(v[0][-1])] for k, v in P["raw"].items()}}

    # ---- step 1: daily reconstruction (csv, gitignored-size: keep monthly sample in results)
    R_ = sum(w * allc[c] for c, w in PROPOSAL["recession"]["weights"].items())
    D_ = sum(w * allc[c] for c, w in PROPOSAL["drawdown"]["weights"].items())
    with open(ROOT / "data" / "macro" / "series" / "_calibration_daily.csv", "w") as f:
        f.write("date,base,proposed_recession,proposed_drawdown," + ",".join(allc) +
                ",t10y3m,hy_bps,sahm,claims_yoy,spx_dd,vix\n")
        for i in range(len(dates)):
            f.write(f"{dates[i]},{base[i]},{R_[i]},{D_[i]}," + ",".join(str(allc[n][i]) for n in allc) +
                    f",{ycv[i]:.2f},{hy[i]:.0f},{sahm[i]:.2f},{cy[i]:.1f},{F['dd'][i]:.1f},{vix[i]:.1f}\n")
    monthly = []
    for i in range(1, len(dates)):
        if str(dates[i])[5:7] != str(dates[i - 1])[5:7] or i == len(dates) - 1:
            j = i - 1 if i != len(dates) - 1 else i
            monthly.append({"date": str(dates[j]), "base": int(base[j]), "proposed_recession": int(R_[j]),
                            "proposed_drawdown": int(D_[j]), **{k: int(v[j]) for k, v in cur.items()}})
    (OUT / "rubric_monthly.json").write_text(json.dumps(monthly))

    # ---- targets
    rstarts = [s for s in recession_starts(P) if s >= np.datetime64("1997-06-01")]
    eps = {thr: [e for e in drawdown_episodes(P, thr) if e["cross"] >= np.datetime64("1997-06-01")]
           for thr in (15, 20)}
    targets = {"A_recession_12m": (labels_recession(P, rstarts), event_windows(P, "A", rstarts)),
               "B15_drawdown_6m": (labels_drawdown(P, eps[15]), event_windows(P, "B", eps[15])),
               "B20_drawdown_6m": (labels_drawdown(P, eps[20]), event_windows(P, "B", eps[20]))}
    meta["recession_starts"] = [str(s) for s in rstarts]
    meta["drawdowns"] = {thr: [{k: (str(v) if not isinstance(v, float) else round(v, 1)) for k, v in e.items()}
                               for e in eps[thr]] for thr in eps}
    meta["label_days"] = {t: {"pos": int(np.nansum(l == 1)), "neg": int(np.nansum(l == 0)),
                              "excluded": int(np.isnan(l).sum())} for t, (l, _) in targets.items()}
    PM = period_masks(dates)
    (OUT / "meta.json").write_text(json.dumps(meta, indent=1, default=str))

    # ---- step 3a: current rubric — components and composite thresholds
    quality = {}
    md = ["# signal quality (generated by calibrate.py)\n"]
    for t, (lab, wins) in targets.items():
        quality[t] = {}
        for per in ("full", "train", "test"):
            wp = period_wins(wins, per); mk = PM[per]
            q = {}
            lp = lab[mk & ~np.isnan(lab)]
            br_ = float(np.mean(lp == 1)) if len(lp) else None
            q["_base_rate"] = br_
            md.append(f"\n## {t} — {per} (base rate of positive days {fmt(br_)})\n\n{TABLE_HEAD}")
            for n, c in allc.items():
                for lvl in (1, 2):
                    r = evaluate(c >= lvl, lab, wp, dates, mk)
                    q[f"{n}>={lvl}"] = summarize(r)
                    md.append(table_row(f"{n} >= {lvl}", r, br_))
            for k in range(3, 9):
                r = evaluate(base >= k, lab, wp, dates, mk)
                q[f"BASE>={k}"] = summarize(r)
                md.append(table_row(f"**current BASE >= {k}**", r, br_))
            nob = np.minimum(8, yc + cr + lb + eq)
            for k in (3, 4, 5, 6):
                r = evaluate(nob >= k, lab, wp, dates, mk)
                q[f"BASE_no_breadth>={k}"] = summarize(r)
                md.append(table_row(f"BASE w/o breadth >= {k}", r, br_))
            quality[t][per] = q
    for t in quality.values():  # keep the tracked json small: per-episode lists live in grid/proposal json
        for per in t.values():
            for r in per.values():
                if isinstance(r, dict):
                    r.pop("false_episode_list", None)
    (OUT / "signal_quality.json").write_text(json.dumps(quality, default=str))
    (OUT / "signal_quality.md").write_text("\n".join(md) + "\n")

    # ---- step 3b: grid. pool = the five live components + the robust curve and credit variants
    # + the two extras with the best TRAIN objective for that target (chosen on 1997-2015 only)
    def best_extras(t, n=2, exclude=("curve_avg63", "hy_rise_1y")):
        lab, wins = targets[t]
        sc = []
        for nme in extra:
            o = max(objective(evaluate(extra[nme] >= l, lab, period_wins(wins, "train"), dates, PM["train"]))
                    for l in (1, 2))
            sc.append((round(o, 3), nme))
        sc.sort(reverse=True)
        return [nme for o, nme in sc if nme not in exclude][:n], sc

    grids, pools = {}, {}
    for t in ("A_recession_12m", "B15_drawdown_6m"):
        ex, ranking = best_extras(t)
        names = list(cur) + ["curve_avg63", "hy_rise_1y"] + ex
        pools[t] = {"pool": names, "extras_ranked_by_train_objective": ranking}
        lab, wins = targets[t]
        g = grid_search(allc, names, lab, dates, PM["train"], PM["test"],
                        period_wins(wins, "train"), period_wins(wins, "test"))
        grids[t] = g
        print(f"grid {t}: {g['weights']} k={g['warn_threshold']} train={g['train_objective']:.2f} "
              f"test={g['test_objective']:.2f} ties={g['tied_best_in_train']}", file=sys.stderr)
        if t == "B15_drawdown_6m":
            lab20, wins20 = targets["B20_drawdown_6m"]
            for per in ("train", "test"):
                g[f"B20_{per}_warn"] = summarize(evaluate(g["_raw"] >= g["warn_threshold"], lab20,
                                                          period_wins(wins20, per), dates, PM[per]))
    # blended: ONE weight vector and ONE threshold for both targets (train objective = mean of the two)
    names_bl = list(dict.fromkeys(pools["A_recession_12m"]["pool"] + pools["B15_drawdown_6m"]["pool"]))
    C = np.stack([allc[n] for n in names_bl]).astype(np.int16)
    labA, winsA = targets["A_recession_12m"]; labB, winsB = targets["B15_drawdown_6m"]
    best = None
    for w in itertools.product((0, 1, 2), repeat=len(names_bl)):
        nz = sum(1 for x in w if x)
        if nz == 0 or nz > 6:
            continue
        raw = (np.array(w, dtype=np.int16)[:, None] * C).sum(axis=0)
        for k in range(2, int(raw.max()) + 1):
            a = raw >= k
            ra = evaluate(a, labA, period_wins(winsA, "train"), dates, PM["train"])
            rb_ = evaluate(a, labB, period_wins(winsB, "train"), dates, PM["train"])
            key = (round((objective(ra) + objective(rb_)) / 2, 6), -nz, -float(a[PM["train"]].mean()))
            if best is None or key > best[0]:
                best = (key, w, k)
    key, w, k = best
    raw = (np.array(w)[:, None] * C).sum(axis=0)
    blended = {"components": names_bl, "weights": dict(zip(names_bl, w)), "threshold": k,
               "note": "search limited to <= 6 non-zero weights"}
    for t, (lab, wins) in targets.items():
        for per in ("train", "test"):
            r = evaluate(raw >= k, lab, period_wins(wins, per), dates, PM[per])
            blended[f"{t}_{per}"] = summarize(r)
            blended[f"{t}_{per}_objective"] = objective(r)
    print(f"blended: {blended['weights']} k={k}", file=sys.stderr)
    current_cmp = {}
    for t, (lab, wins) in targets.items():
        for per in ("train", "test"):
            for kk in (4, 6, 7):
                r = evaluate(base >= kk, lab, period_wins(wins, per), dates, PM[per])
                current_cmp[f"{t}_{per}_>={kk}"] = summarize(r)
                current_cmp[f"{t}_{per}_>={kk}_objective"] = objective(r)
    for g in grids.values():
        g.pop("_raw", None)
    (OUT / "grid.json").write_text(json.dumps({"pools": pools, "separate": grids, "blended": blended,
                                               "current_rubric": current_cmp}, indent=1, default=str))

    # ---- step 3c: a handful of PRE-DECLARED parsimonious designs (base max 8 each), the design and
    # threshold picked on 1997-2015 only. (drafted after reading the single-signal tables, which
    # include 2016+ — so their test numbers are quasi-out-of-sample; the grid above is the clean test.)
    prop = proposals(allc, targets, dates, PM, F, eps)
    R = sum(w * allc[c] for c, w in PROPOSAL["recession"]["weights"].items())
    Dd = sum(w * allc[c] for c, w in PROPOSAL["drawdown"]["weights"].items())
    final = {"spec": PROPOSAL, "today": {"recession": int(R[-1]), "drawdown": int(Dd[-1])}}
    for nm, sc, tl in (("recession", R, ["A_recession_12m"]), ("drawdown", Dd, ["B15_drawdown_6m", "B20_drawdown_6m"])):
        for lvl in ("warn", "crit"):
            k = PROPOSAL[nm][lvl]
            for t in tl:
                lab, wins = targets[t]
                for per in ("train", "test", "full"):
                    final[f"{nm}_{lvl}>={k}_{t}_{per}"] = summarize(evaluate(sc >= k, lab, period_wins(wins, per), dates, PM[per]))
        final[f"{nm}_share_of_days_by_level"] = {int(k): float(np.mean(sc >= k)) for k in range(1, 9)}
    # escalation rule: drawdown warn AND recession >= warn
    esc = (Dd >= PROPOSAL["drawdown"]["warn"]) & (R >= PROPOSAL["recession"]["warn"])
    for t in ("B15_drawdown_6m", "B20_drawdown_6m"):
        lab, wins = targets[t]
        for per in ("train", "test", "full"):
            final[f"escalation_{t}_{per}"] = summarize(evaluate(esc, lab, period_wins(wins, per), dates, PM[per]))
    cond = conditional_8pct(dates, F["dd"], {"drawdown_score": Dd, "recession_score": R, "current_base": base,
                                             "curve_avg63": allc["curve_avg63"]})
    final["conditional_8pct"] = cond
    for nm, sc in (("drawdown", Dd), ("escalation", esc.astype(int))):
        kk = PROPOSAL["drawdown"]["warn"] if nm == "drawdown" else 1
        for thr in (15, 20):
            final[f"{nm}_warn_timing_dd{thr}"] = warn_timing(sc, kk, dates, eps[thr], F["close"])
    for thr in (15, 20):
        final[f"drawdown_crit_timing_dd{thr}"] = warn_timing(Dd, PROPOSAL["drawdown"]["crit"], dates, eps[thr], F["close"])
    (OUT / "final_proposal.json").write_text(json.dumps(final, indent=1, default=str))
    (OUT / "proposal.json").write_text(json.dumps(prop, indent=1, default=str))
    (OUT / "proposal.md").write_text(proposal_md(prop))

    # ---- step 2: episode table
    ep_rows = episode_table(dates, base, cur, extra, rstarts, eps, {"R": R, "D": Dd})
    (OUT / "episodes.json").write_text(json.dumps(ep_rows, indent=1, default=str))
    (OUT / "episodes.md").write_text(episode_md(ep_rows))

    # today
    print(json.dumps({"today": str(dates[-1]), "base": int(base[-1]),
                      **{k: int(v[-1]) for k, v in allc.items()}}), file=sys.stderr)
    print(f"wrote {OUT}/*.json, *.md", file=sys.stderr)
    return 0


DESIGNS = {
    "A_recession_12m": {
        "A1 curve2+hyrise+labor": {"curve_avg63": 2, "hy_rise_1y": 1, "labor": 1},
        "A2 curve2+labor+claims": {"curve_avg63": 2, "labor": 1, "claims_rise": 1},
        "A3 curve2+credit(live)+labor": {"curve_avg63": 2, "credit": 1, "labor": 1},
        "A4 live curve2+credit+labor": {"yield_curve": 2, "credit": 1, "labor": 1},
        "A5 curve2+hyrise+permits": {"curve_avg63": 2, "hy_rise_1y": 1, "permits_yoy": 1},
    },
    "B15_drawdown_6m": {
        "B1 ret21+rv20+hychg+curve": {"spx_ret21": 1, "rv20": 1, "hy_chg_1m": 1, "curve_avg63": 1},
        "B2 equity(live)+hyrise+ret21+curve": {"equity_stress": 1, "hy_rise_1y": 1, "spx_ret21": 1, "curve_avg63": 1},
        "B3 ret21+rv20+hyrise+stlfsi": {"spx_ret21": 1, "rv20": 1, "hy_rise_1y": 1, "stlfsi": 1},
        "B4 equity+vixterm+hychg+dist50": {"equity_stress": 1, "vix_term": 1, "hy_chg_1m": 1, "spx_dist50": 1},
        "B5 ret21+rv20+hyrise+curve": {"spx_ret21": 1, "rv20": 1, "hy_rise_1y": 1, "curve_avg63": 1},
        # added after reading the 2016+ single-signal tables (post hoc): B3 without the revised STLFSI
        "B6 ret21+rv20+hyrise+equity(live)": {"spx_ret21": 1, "rv20": 1, "hy_rise_1y": 1, "equity_stress": 1},
    },
}

# the study's PROPOSAL (not applied anywhere). base 0-8 each; thresholds on the base.
PROPOSAL = {
    "recession": {"weights": {"curve_avg63": 2, "hy_rise_1y": 1, "labor": 1}, "warn": 4, "crit": 6},
    "drawdown": {"weights": {"spx_ret21": 1, "rv20": 1, "hy_rise_1y": 1, "equity_stress": 1}, "warn": 3, "crit": 5},
}


def conditional_8pct(dates, dd, scores: dict, gap: int = 63) -> dict:
    """every S&P decline that reached 8 % from its 252-session high since 1997: did it go on to
    15 % / 20 %, and what did each score read on the first 8 % day?"""
    idx = np.flatnonzero(dd >= 8)
    brk = np.flatnonzero(np.diff(idx) > gap)
    st = np.concatenate([[idx[0]], idx[brk + 1]]); en = np.concatenate([idx[brk], [idx[-1]]])
    rows = [{"first_8pct_day": str(dates[a]), "max_dd": round(float(dd[a:b + 1].max()), 1),
             **{n: int(v[a]) for n, v in scores.items()}} for a, b in zip(st, en)]
    summary = {"all": {"n": len(rows), "reached15": sum(r["max_dd"] >= 15 for r in rows),
                       "reached20": sum(r["max_dd"] >= 20 for r in rows)}}
    for n in scores:
        for k in sorted(set(r[n] for r in rows)):
            if k == 0:
                continue
            on = [r for r in rows if r[n] >= k]; off = [r for r in rows if r[n] < k]
            summary[f"{n}>={k}"] = {"n": len(on), "reached15": sum(r["max_dd"] >= 15 for r in on),
                                    "reached20": sum(r["max_dd"] >= 20 for r in on),
                                    "n_below": len(off), "below_reached15": sum(r["max_dd"] >= 15 for r in off),
                                    "below_reached20": sum(r["max_dd"] >= 20 for r in off)}
    return {"rows": rows, "summary": summary}


def warn_timing(raw, k, dates, eps_thr, close):
    """per drawdown: first warning (score >= k) in [peak - 182d, cross - 14d], the loss already
    taken from the peak at that moment, and the fall still ahead (warning close -> trough)."""
    rows = []
    for e in eps_thr:
        i0 = int(np.searchsorted(dates, e["peak"] - np.timedelta64(182, "D")))
        i1 = int(np.searchsorted(dates, e["cross"] - np.timedelta64(14, "D")))
        ip = int(np.searchsorted(dates, e["peak"]))
        it = int(np.searchsorted(dates, e["trough"]))
        seg = np.flatnonzero(raw[i0:i1] >= k)
        if not len(seg):
            rows.append({"peak": str(e["peak"]), "warned": False, "max_dd": round(e["max_dd"], 1)})
            continue
        ia = i0 + int(seg[0])
        rows.append({"peak": str(e["peak"]), "warned": True, "first_warn": str(dates[ia]),
                     "months_before_peak": months(int((dates[ip] - dates[ia]).astype(int))),
                     "dd_from_peak_at_warn_pct": round(max(0.0, (1 - close[ia] / close[ip]) * 100), 1) if ia >= ip else 0.0,
                     "fall_from_warn_to_trough_pct": round((1 - close[it] / close[ia]) * 100, 1),
                     "max_dd": round(e["max_dd"], 1)})
    return rows


def proposals(allc, targets, dates, PM, F, eps):
    out = {}
    close = F["close"]
    for t, designs in DESIGNS.items():
        lab, wins = targets[t]
        wtr, wte = period_wins(wins, "train"), period_wins(wins, "test")
        cands = []
        for dn, wts in designs.items():
            raw = sum(w * allc[c] for c, w in wts.items())
            for k in range(2, 9):
                r = evaluate(raw >= k, lab, wtr, dates, PM["train"])
                lead = float(np.median(r["lead_days_to_event"])) if r["lead_days_to_event"] else 0.0
                cands.append(((round(objective(r), 6), lead, -r["alarm_day_share"]), dn, k))
        cands.sort(key=lambda x: x[0], reverse=True)
        _, dn, k = cands[0]
        wts = designs[dn]
        raw = sum(w * allc[c] for c, w in wts.items())
        crit = None
        for kk in range(k + 1, 9):
            rr = evaluate(raw >= kk, lab, wtr, dates, PM["train"])
            if rr["event_recall"] and rr["event_recall"] >= 0.5:
                if crit is None or (rr["episode_precision"] or 0) > (crit[1] or 0):
                    crit = (kk, rr["episode_precision"])
        res = {"chosen": dn, "weights": wts, "warn": k, "crit": crit[0] if crit else None,
               "design_ranking_train": [{"design": d, "k": kk, "train_objective": key[0]} for key, d, kk in cands[:12]],
               "by_threshold": {}}
        for kk in range(2, 9):
            res["by_threshold"][kk] = {}
            for tt, (lb, wn) in targets.items():
                if (t.startswith("A") and not tt.startswith("A")) or (t.startswith("B") and tt.startswith("A")):
                    continue
                for per in ("train", "test", "full"):
                    r = evaluate(raw >= kk, lb, period_wins(wn, per), dates, PM[per])
                    res["by_threshold"][kk][f"{tt}_{per}"] = summarize(r)
        if t.startswith("B"):
            for thr in (15, 20):
                res[f"warn_timing_dd{thr}"] = warn_timing(raw, k, dates, eps[thr], close)
        out[t] = res
        out[t]["_raw"] = raw
    for t in out:
        out[t]["daily_last"] = int(out[t]["_raw"][-1])
        out[t]["_raw"] = None
    return out


def proposal_md(prop):
    L = ["# pre-declared designs — chosen on 1997-2015\n"]
    for t, r in prop.items():
        L.append(f"\n## {t}: chosen **{r['chosen']}** weights {r['weights']}, warn >= {r['warn']}, crit >= {r['crit']} (today {r['daily_last']})\n")
        L.append("| k | target/period | events hit | before peak | median lead (mo) | false ep | ep prec | day prec | days on |")
        L.append("|---|---|---|---|---|---|---|---|---|")
        for k, d in r["by_threshold"].items():
            for key, x in d.items():
                L.append(f"| {k} | {key} | {len(x['events_detected'])}/{x['events']} | {len(x['detected_before_peak'])} | "
                         f"{fmt(months(x['median_lead_days']), 1)} | {x['false_episodes']} | {fmt(x['episode_precision'])} | "
                         f"{fmt(x['day_precision'])} | {fmt(x['alarm_day_share'])} |")
        for thr in (15, 20):
            if f"warn_timing_dd{thr}" in r:
                L.append(f"\nwarn timing vs >= {thr}% drawdowns: " + "; ".join(
                    (f"{x['peak'][:7]} warned {x['first_warn']} (dd {x['dd_from_peak_at_warn_pct']}% then -{x['fall_from_warn_to_trough_pct']}% more)"
                     if x["warned"] else f"{x['peak'][:7]} not warned") for x in r[f"warn_timing_dd{thr}"]))
    return "\n".join(L) + "\n"


EPISODES_EXTRA = [  # notable scares / false alarms: (name, anchor date, notes)
    ("1998 LTCM/Russia", "1998-07-17", "drawdown 19%, no recession"),
    ("2011 downgrade/euro", "2011-04-29", "drawdown 19%, no recession"),
    ("2015-16 China/oil", "2015-05-21", "max drawdown 14% (<15%), no recession"),
    ("2019 inversion", "2019-05-23", "T10Y3M inverted May-Oct 2019; COVID recession Mar 2020"),
    ("2022-24 inversion", "2022-10-25", "longest inversion on record, no recession (as of 2026-10)"),
    ("2023 SVB", "2023-03-08", "bank failures, no 15% drawdown, no recession"),
    ("2024 Aug Sahm trigger", "2024-08-02", "Sahm real-time 0.53 (Jul data), no recession"),
]


def episode_table(dates, base, cur, extra, rstarts, eps, prop_scores=None):
    def at(d):
        i = int(np.searchsorted(dates, d, side="right")) - 1
        return int(base[i]) if 0 <= i < len(dates) else None

    def first_cross(lo, hi, k):
        m = (dates >= lo) & (dates <= hi) & (base >= k)
        return str(dates[np.argmax(m)]) if m.any() else None

    def onset(ev, hi, c, gap=21):
        """start of the alarm run (>=1, gaps <= 21 sessions merged) that is active at the event
        date, else the first run that starts after it (up to `hi`); lead in months (+ = before)."""
        on = c >= 1
        i = int(np.searchsorted(dates, ev))
        i = min(i, len(dates) - 1)
        if on[max(0, i - gap):i + 1].any():
            j = max(0, i - gap) + int(np.flatnonzero(on[max(0, i - gap):i + 1])[-1])
            while True:
                prev = np.flatnonzero(on[max(0, j - gap - 1):j])
                if not len(prev):
                    break
                j = max(0, j - gap - 1) + int(prev[0])
                if j == 0:
                    break
        else:
            hi_i = int(np.searchsorted(dates, hi, side="right"))
            nxt = np.flatnonzero(on[i:hi_i])
            if not len(nxt):
                return None
            j = i + int(nxt[0])
        return [str(dates[j]), months(int((ev - dates[j]).astype(int)))]

    D = lambda n: np.timedelta64(n, "D")  # noqa: E731
    rows = []
    items = []
    for s in rstarts:
        items.append(("recession " + str(s)[:7], "A", s, s, s + D(365)))
    for e in eps[15]:
        items.append((f"drawdown {str(e['peak'])[:7]} (-{e['max_dd']:.0f}%)", "B", e["peak"], e["cross"], e["end"]))
    for n, a, note in EPISODES_EXTRA:
        a = np.datetime64(a)
        items.append((n + " — " + note, "FA", a, a, a + D(270)))
    for name, kind, ev, ev2, end in items:
        lo = ev - D(730)
        row = {"episode": name, "kind": kind, "event_date": str(ev), "cross_date": str(ev2),
               "score_-12m": at(ev - D(365)), "score_-6m": at(ev - D(182)), "score_-3m": at(ev - D(91)),
               "score_-1m": at(ev - D(30)), "score_at_event": at(ev), "score_at_cross": at(ev2),
               "first_ge4": first_cross(ev - D(365), end, 4), "first_ge6": first_cross(ev - D(365), end, 6),
               "first_ge7": first_cross(ev - D(365), end, 7),
               "max_score_-12m_to_end": int(base[(dates >= ev - D(365)) & (dates <= end)].max()),
               "component_onset": {k: onset(ev, end, v) for k, v in {**cur, **extra}.items()}}
        for pn, ps in (prop_scores or {}).items():
            def pat(d, ps=ps):
                i = int(np.searchsorted(dates, d, side="right")) - 1
                return int(ps[i]) if 0 <= i < len(dates) else None
            row[f"{pn}_path"] = [pat(ev - D(365)), pat(ev - D(182)), pat(ev - D(91)), pat(ev - D(30)), pat(ev), pat(ev2)]
            for k in (3, 4, 5, 6):
                m = (dates >= ev - D(365)) & (dates <= end) & (ps >= k)
                row[f"{pn}_first_ge{k}"] = str(dates[np.argmax(m)]) if m.any() else None
        rows.append(row)
    return rows


def episode_md(rows):
    L = ["| episode | event | −12m | −6m | −3m | −1m | at event | at 15% cross | first ≥4 | first ≥6 | first ≥7 | max |",
         "|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in rows:
        L.append(f"| {r['episode']} | {r['event_date']} | {r['score_-12m']} | {r['score_-6m']} | {r['score_-3m']} | "
                 f"{r['score_-1m']} | {r['score_at_event']} | {r['score_at_cross'] if r['kind'] == 'B' else '–'} | "
                 f"{r['first_ge4'] or '–'} | {r['first_ge6'] or '–'} | {r['first_ge7'] or '–'} | {r['max_score_-12m_to_end']} |")
    L.append("\nproposed scores (R = recession, D = drawdown): path at -12m/-6m/-3m/-1m/event/15%-cross; first date >= warn/crit\n")
    L.append("| episode | R path | R first >=4 | R first >=6 | D path | D first >=3 | D first >=5 |\n|---|---|---|---|---|---|---|")
    for r in rows:
        if "R_path" in r:
            L.append(f"| {r['episode'][:28]} | {'/'.join(map(str, r['R_path']))} | {r['R_first_ge4'] or '–'} | {r['R_first_ge6'] or '–'} | "
                     f"{'/'.join(map(str, r['D_path']))} | {r['D_first_ge3'] or '–'} | {r['D_first_ge5'] or '–'} |")
    L.append("\nonset of each component's alarm run (>=1) active at the event date, else the first run "
             "after it; months before the event (negative = after):\n")
    for r in rows:
        f = {k: v for k, v in r["component_onset"].items() if v}
        L.append(f"- **{r['episode']}** ({r['event_date']}): " +
                 "; ".join(f"{k} {v[1]:+}" for k, v in sorted(f.items(), key=lambda kv: kv[1][0])))
    return "\n".join(L) + "\n"


if __name__ == "__main__":
    sys.exit(main())
