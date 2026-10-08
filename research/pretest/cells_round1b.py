#!/usr/bin/env python3
"""pretest round 1b — pre-registered neighbourhood scans for five round-1/round-2 cells
(#1 QQQ ORB, #6 HKS half-hour, #15 earnings-gap fade, #17 turn-of-month, #9 pre-FOMC).

Why a separate file: round 1b needs (a) a custom bar-by-bar executor for a VWAP-cross stop,
which `harness.execute()` cannot express (it only checks a fixed numeric stop/target), and
(b) a cross-sectional rank-and-trade shape for the HKS cell (rank ~34 names each half-hour,
long the top bucket / short the bottom bucket) that does not fit `cells.py`'s one-symbol-at-a-time
`rule(b, i, p) -> [Order, ...]` interface that `harness.run_cell()` assumes. Rather than reshape
the harness to fit two one-off shapes, this file is run STANDALONE through harness.py's loader
functions (`bars`, `fill`, `metrics`, `loyo`, `kill`, `Order`, `execute`, `run_rule`) and writes
its own `results/<id>.json` files directly:

    python3 research/pretest/cells_round1b.py              # all 5 studies (~1-2 min)
    python3 research/pretest/cells_round1b.py c11 c14      # by id prefix

No edit was made to harness.py or cells.py. Every grid below is written out (pre-registered)
*before* the run that follows it in this file -- see the comment block at the top of each
section. Conventions, cost model and sizing are identical to harness.py (see its docstring).
"""
import csv
import datetime as dt
import json
import os
import re
import sys
import warnings

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from harness import (ET, NOTIONAL, COST_BPS, COST_SH, ROOT, YEARS, Order, bars, execute, fill, kill, loyo, metrics,
                     plabel, run_rule)

warnings.filterwarnings("ignore", "Mean of empty slice")
warnings.filterwarnings("ignore", "invalid value encountered")
RNG = np.random.default_rng(1)

ETFS = {"SPY", "QQQ", "SMH", "IWM", "XLE", "XLF", "XLK"}
ALL_SYMS = sorted(f[:-4] for f in os.listdir(f"{ROOT}/data/bars_iex") if f.endswith(".csv"))
STOCKS = [s for s in ALL_SYMS if s not in ETFS]          # ~34 single names for the HKS cross-section
FOMC = {dt.date.fromisoformat(d) for d in re.findall(r'"(\d{4}-\d{2}-\d{2})"', open(
    f"{ROOT}/crates/indicators/src/custom/event_calendar.rs").read().split("FOMC_DECISION_DAYS")[1].split("];")[0])}


def random_control(pool_bps, obs_bps, draws=5000):
    """is the observed trade set's mean bps beaten by `draws` random same-size draws (without
    replacement) from `pool_bps`? mirrors cells.py's calendar_control but reusable for any cell."""
    pool_bps, obs_bps = np.asarray(pool_bps, float), np.asarray(obs_bps, float)
    n = len(obs_bps)
    if n == 0 or len(pool_bps) < n:
        return {"pool_mean_bps": None, "obs_mean_bps": None, "p_random": None, "n": n}
    sims = np.array([RNG.choice(pool_bps, n, replace=False).mean() for _ in range(draws)])
    return {"pool_mean_bps": round(float(pool_bps.mean()), 2), "obs_mean_bps": round(float(obs_bps.mean()), 2),
            "p_random": round(float((sims >= obs_bps.mean()).mean()), 4), "n": n, "draws": draws}


def concentration(trades):
    """share of total P&L from the single best-contributing symbol and from the best 3 trades.
    `n_syms` lets classify() skip the top-name check on a single-instrument cell, where it is 100%
    by construction and says nothing about fragility."""
    if not trades:
        return {"top_name_share": 0.0, "top3_trades_share": 0.0, "top_name": None, "n_syms": 0}
    pnl = np.array([t["pnl"] for t in trades])
    total = pnl.sum()
    by_sym = {}
    for t in trades:
        by_sym[t["sym"]] = by_sym.get(t["sym"], 0.0) + t["pnl"]
    top_name, top_pnl = max(by_sym.items(), key=lambda kv: kv[1])
    top3 = float(np.sort(pnl)[-3:].sum())
    return {"top_name_share": round(float(top_pnl / total), 3) if total else 0.0, "top_name": top_name,
            "top3_trades_share": round(float(top3 / total), 3) if total else 0.0, "n_syms": len(by_sym)}


def pick_best(label_to_metrics, kill_years=4, kill_pf=1.3):
    """prefer a grid point that actually CLEARS the kill bar (highest P&L among those); only if
    none clears it fall back to the highest-P&L point overall. Returns (label, metrics, cleared)."""
    passing = [(lab, m) for lab, m in label_to_metrics.items() if kill(m, kill_years, kill_pf)[0]]
    pool = passing if passing else list(label_to_metrics.items())
    lab, m = max(pool, key=lambda kv: kv[1]["pnl"])
    return lab, m, bool(passing)


def classify(points, best_m, cleared, kill_years=4, kill_pf=1.3, decay_early=(2022, 2023, 2024),
             decay_late=(2025, 2026), conc=None):
    """new verdict standard (see README): dead / sub-cost / decayed / fragile / pass. `best_m` /
    `cleared` come from pick_best() over the same pre-registered neighbourhood `points`."""
    if not points:
        return "dead", "no grid point produced a trade"
    any_gross = any(m.get("gross_bps", m.get("bps", -1)) > 0 for m in points)
    if not any_gross:
        return "dead", f"no gross edge anywhere in the {len(points)}-cell neighbourhood scanned"
    if not cleared:
        _, why = kill(best_m, kill_years, kill_pf)
        return "sub-cost", f"best gross {best_m.get('gross_bps', 0):+.1f} bps/trade but net fails: {why}"
    py = best_m.get("per_year", {})
    early = [py[y]["pnl"] for y in py if int(y) in decay_early]
    late = [py[y]["pnl"] for y in py if int(y) in decay_late]
    if early and all(v > 0 for v in early) and late and all(v <= 0 for v in late):
        return "decayed", f"positive in {sorted(int(y) for y in py if int(y) in decay_early)}, <=0 in {decay_late}"
    name_frag = conc and conc.get("n_syms", 0) > 1 and conc.get("top_name_share", 0) >= 0.7
    trade_frag = conc and conc.get("top3_trades_share", 0) >= 0.7
    if name_frag or trade_frag:
        return "fragile", f"{conc['top_name_share']*100:.0f}% from {conc.get('top_name')} (n_syms={conc.get('n_syms')}), {conc['top3_trades_share']*100:.0f}% from top-3 trades"
    return "pass", f"clears the bar: {kill(best_m, kill_years, kill_pf)[1]}"


def dump(cell_id, obj):
    with open(f"{HERE}/results/{cell_id}.json", "w") as f:
        json.dump(obj, f, indent=1, default=str)
    print(f"wrote results/{cell_id}.json")

# ======================================================================================
# (a) c11_orb_grid -- #1 QQQ opening-range breakout, pre-registered neighbourhood.
# PRE-REGISTRATION (written before this file was ever run against the bars): or_len in
# {5, 15, 30} x stop in {other side of range, 1x range, VWAP cross} x exit in {hold to close,
# 2R target, 5R target} x vol-filter in {none, 14-session avg open->close move >= 0.7%, >= 1.0%}
# = 81 cells, run in full on QQQ (one trade/day). The top 3 (stop, exit, filter, or_len) points by
# net bps on QQQ (min 50 trades) are then re-run, unchanged, on SPY/SMH/IWM -- that pruning (not
# the grid itself) is the one pre-registered shortcut, per the task brief. Kill: net positive in
# >= 4/5 years and PF > 1.3. Control: same breakout fired with the OPPOSITE side (reverse-side
# control) -- is the OR candle's direction actually informative, or would either side do?
# ======================================================================================
OR_LENS, STOPS, EXITS, FILTS = [5, 15, 30], ["other", "1r", "vwap"], ["close", "2r", "5r"], [None, 0.007, 0.01]
ORB_GRID = [{"or": o, "stop": s, "exit": x, "filt": f} for o in OR_LENS for s in STOPS for x in EXITS for f in FILTS]

_MOVE_AVG = {}
def move_avg(b, w=14):
    if b.sym not in _MOVE_AVG:
        n = len(b.dates)
        mv = np.full(n, np.nan)
        for i in range(n):
            if b.full(i):
                o, c = b.session_open(i), b.session_close(i)
                if np.isfinite(o) and np.isfinite(c) and o:
                    mv[i] = abs(c / o - 1)
        avg = np.full(n, np.nan)
        for i in range(n):
            hist = mv[max(0, i - w):i]
            hist = hist[~np.isnan(hist)]
            if len(hist) >= 10:
                avg[i] = hist.mean()
        _MOVE_AVG[b.sym] = avg
    return _MOVE_AVG[b.sym]


def scan_stop_target(b, i, m0, side, stop, target):
    """first bar at/after m0 (session i) that touches stop or target; gap-through fills at the
    bar's open; stop wins a bar that touches both. returns (fill_price, exit_session) or None."""
    h, l, o, ok = b.h[i, m0:390], b.l[i, m0:390], b.o[i, m0:390], b.has[i, m0:390]
    stop_hit = ok & ((l <= stop) if side > 0 else (h >= stop)) if stop is not None else np.zeros_like(ok)
    tgt_hit = ok & ((h >= target) if side > 0 else (l <= target)) if target is not None else np.zeros_like(ok)
    hit = np.flatnonzero(stop_hit | tgt_hit)
    if len(hit) == 0:
        return None
    j = hit[0]
    is_stop = stop_hit[j]
    lvl = stop if is_stop else target
    gapped = (o[j] - lvl) * side * (1 if is_stop else -1) < 0
    return o[j] if gapped else lvl


def exec_vwap(b, i, m0, side, target):
    """VWAP-cross stop (approximation of a trailing stop): cumulative session VWAP from the open;
    exit at the NEXT bar's open the first time a bar closes against VWAP, or on target, else hold
    to the close. harness.execute() cannot express this (fixed numeric stop/target only), hence the
    custom loop -- vectorized per session, no per-bar python loop."""
    hh, ll, cc, oo, vv, ok = b.h[i], b.l[i], b.c[i], b.o[i], b.v[i], b.has[i]
    pv = np.where(ok, (hh + ll + cc) / 3 * vv, 0.0)
    cv = np.cumsum(np.where(ok, vv, 0.0))
    vwap = np.where(cv > 0, np.cumsum(pv) / np.where(cv > 0, cv, 1), np.nan)
    tgt_hit = ok & ((hh >= target) if side > 0 else (ll <= target)) if target is not None else np.zeros(390, bool)
    vwap_hit = np.zeros(390, dtype=bool)
    if m0 + 1 < 390:
        seg = slice(m0 + 1, 390)
        vwap_hit[seg] = ok[seg] & ((cc[seg] < vwap[seg]) if side > 0 else (cc[seg] > vwap[seg]))
    hit = np.flatnonzero((tgt_hit | vwap_hit)[m0:])
    if len(hit) == 0:
        return b.exit_px(i, 390)
    k = m0 + hit[0]
    if tgt_hit[k]:
        lvl = target
        return oo[k] if (oo[k] - lvl) * side < 0 else lvl
    nxt = np.flatnonzero(ok[k + 1:])
    return oo[k + 1 + nxt[0]] if len(nxt) else b.exit_px(i, 390)


def run_orb(sym, p, flip=False):
    b, n, trades = bars(sym), p["or"], []
    avg = move_avg(b)
    for i in range(len(b.dates)):
        if not b.has[i, :n].all():
            continue
        o, c = b.o[i, 0], b.c[i, n - 1]
        if c == o:
            continue
        side = (1 if c > o else -1) * (-1 if flip else 1)
        if p["filt"] is not None and not (avg[i] >= p["filt"]):
            continue
        e, m0 = b.entry_px(i, n)
        if not np.isfinite(e):
            continue
        hi, lo = b.h[i, :n].max(), b.l[i, :n].min()
        rng = hi - lo
        if not rng > 0:
            continue
        stop = (lo if side > 0 else hi) if p["stop"] != "1r" else (e - side * rng)
        R = abs(e - stop)
        target = e + side * {"close": 0, "2r": 2, "5r": 5}[p["exit"]] * R if p["exit"] != "close" else None
        x = exec_vwap(b, i, m0, side, target) if p["stop"] == "vwap" else (
            scan_stop_target(b, i, m0, side, stop, target) or b.exit_px(i, 390))
        if not np.isfinite(x):
            continue
        t = fill(sym, side, b.dates[i], b.dates[i], e, x)
        if t:
            trades.append(t)
    return trades, len(b.dates)


def cell_orb():
    by_p = {}
    for p in ORB_GRID:
        trades, sessions = run_orb("QQQ", p)
        by_p[plabel(p)] = dict(metrics(trades, sessions), trades_cached=len(trades))
    lo = loyo({k: v for k, v in by_p.items()})
    ranked = sorted(((lab, m) for lab, m in by_p.items() if m["n"] >= 50), key=lambda kv: -kv[1]["bps"])
    top3 = ranked[:3] if ranked else sorted(by_p.items(), key=lambda kv: -kv[1]["bps"])[:3]
    top3_params = [next(pp for pp in ORB_GRID if plabel(pp) == lab) for lab, _ in top3]
    cross = {}
    for inst in ("SPY", "SMH", "IWM"):
        cross[inst] = {}
        for pp in top3_params:
            trades, sessions = run_orb(inst, pp)
            cross[inst][plabel(pp)] = metrics(trades, sessions)
    all_points_by_label = dict(by_p)
    for inst, d in cross.items():
        for lab, m in d.items():
            all_points_by_label[f"{inst}:{lab}"] = m
    best_label, best_m, cleared = pick_best(all_points_by_label)
    best_params = next((pp for pp in ORB_GRID if plabel(pp) == best_label.split(":")[-1]), top3_params[0])
    best_inst = best_label.split(":")[0] if ":" in best_label else "QQQ"
    obs_trades, _ = run_orb(best_inst, best_params)
    flip_trades, _ = run_orb(best_inst, best_params, flip=True)
    reverse_control = {"actual_bps": round(float(np.mean([t["bps"] for t in obs_trades])), 2) if obs_trades else None,
                        "reverse_side_bps": round(float(np.mean([t["bps"] for t in flip_trades])), 2) if flip_trades else None,
                        "actual_pnl": round(sum(t["pnl"] for t in obs_trades), 1),
                        "reverse_side_pnl": round(sum(t["pnl"] for t in flip_trades), 1)}
    conc = concentration(obs_trades)
    all_points = list(all_points_by_label.values())
    verdict, why = classify(all_points, best_m, cleared, conc=conc)
    res = {"id": "c11_orb_grid", "anomaly": "#1 Zarattini-Aziz ORB, grid neighbourhood",
           "cells_scanned_qqq": len(ORB_GRID), "grid_dims": {"or": OR_LENS, "stop": STOPS, "exit": EXITS,
           "filt": [str(f) for f in FILTS]}, "qqq_grid": by_p, "loyo_qqq": lo, "best_label": best_label,
           "best": best_m, "top3_labels": [lab for lab, _ in top3], "cross_instrument": cross,
           "reverse_side_control": reverse_control, "concentration": conc, "verdict": verdict, "verdict_text": why}
    dump("c11_orb_grid", res)
    return res

# ======================================================================================
# (b) c12_hks_cross_sectional -- #6 HKS half-hour periodicity, done the way the paper does it:
# CROSS-SECTIONAL. PRE-REGISTRATION: each half-hour h, rank the STOCKS names (~34 with minute
# bars) by their same-clock half-hour return k trading days ago, k in {1, 2, 5, 10, 20, 40} and
# the 40-session trailing average ("avg40", the paper's "every 1-day lag" pattern) x bucket width
# in {decile, quintile} x half-hour scope in {all 13 halves, first+last half-hour only} = 7 x 2 x
# 2 = 28 cells. Go long the top bucket / short the bottom bucket, dollar-neutral, equal-weight,
# that half-hour only (no overnight carry). Kill (tradability): net positive in >= 4/5 years and
# PF > 1.3 on the half-hour long-short book. Kill (mechanism, separate): Fama-MacBeth cross-
# sectional slope of today's half-hour return on the lagged signal, averaged across days, must be
# reliably signed (t-stat) -- this answers "does the periodicity exist at all," independent of
# whether it clears costs. Control: permutation test -- shuffle which day's signal is attached to
# which day's realized cross-section (1000 draws), does the real long-short beat the shuffled null?
# ======================================================================================
HKS_HALVES_ALL = tuple(range(13))
HKS_HALVES_OC = (0, 12)
HKS_KS = [1, 2, 5, 10, 20, 40, "avg40"]
HKS_BUCKETS = {"decile": 0.1, "quintile": 0.2}


def build_halfhour_grid():
    """OPEN_H/CLOSE_H/R per name (own date index), then reindexed onto the common date list all
    34 names share, so cross-sectional ranking on day d compares the same calendar day everywhere."""
    per_sym = {}
    for s in STOCKS:
        b = bars(s)
        oh = b.o[:, 0:390:30]          # (sessions, 13): open of each half-hour's first minute
        ch = b.c[:, 29:390:30]         # close of each half-hour's last minute
        ok = b.has[:, 0:390:30] & b.has[:, 29:390:30]
        r = np.where(ok, ch / np.where(oh != 0, oh, np.nan) - 1, np.nan)
        per_sym[s] = (b.dates, r)
    common = sorted(set.intersection(*[set(d) for d, _ in per_sym.values()]))
    D, N, H = len(common), len(STOCKS), 13
    R = np.full((N, D, H), np.nan)
    for n, s in enumerate(STOCKS):
        dates, r = per_sym[s]
        idx = {d: i for i, d in enumerate(dates)}
        rows = [idx[d] for d in common]
        R[n] = r[rows]
    return common, R


def signal_array(R, k):
    """SIG[n, d, h]: the k-days-ago same-half-hour return (single lag) or the 40-session trailing
    mean ("avg40"), NaN where there isn't enough history."""
    N, D, H = R.shape
    if k == "avg40":
        SIG = np.full_like(R, np.nan)
        for d in range(40, D):
            SIG[:, d, :] = np.nanmean(R[:, d - 40:d, :], axis=1)
        return SIG
    SIG = np.full_like(R, np.nan)
    if k < D:
        SIG[:, k:, :] = R[:, :-k, :]
    return SIG


def cross_sectional_trades(common, R, SIG, bucket_frac, halves):
    """for each (day, half) in scope: rank names by SIG, equal-weight long top bucket / short
    bottom bucket, realize at R (the day's actual half-hour return), cost like any other trade."""
    N, D, H = R.shape
    trades, daily_ls = [], {}         # daily_ls[(year, half)] = list of long-short gross returns (for FM/t-stat)
    start = 40
    for d in range(start, D):
        date = common[d]
        for h in halves:
            sig, ret = SIG[:, d, h], R[:, d, h]
            valid = ~np.isnan(sig) & ~np.isnan(ret)
            names = np.flatnonzero(valid)
            if len(names) < 10:
                continue
            order = names[np.argsort(sig[names])]
            m = max(1, int(round(len(order) * bucket_frac)))
            bottom, top = order[:m], order[-m:]
            ls_gross = float(ret[top].mean() - ret[bottom].mean())
            daily_ls.setdefault((date.year, h), []).append(ls_gross)
            for side, bucket in ((1, top), (-1, bottom)):
                e = np.array([bars(STOCKS[n]).o[bars(STOCKS[n]).index[date], 30 * h] for n in bucket])
                x = np.array([bars(STOCKS[n]).c[bars(STOCKS[n]).index[date], 30 * h + 29] for n in bucket])
                shares = (NOTIONAL / np.where(e > 0, e, np.nan)).astype(int)
                okm = (shares > 0) & np.isfinite(e) & np.isfinite(x)
                for j in np.flatnonzero(okm):
                    sh = int(shares[j])
                    gross = side * (x[j] - e[j]) * sh
                    c = (COST_BPS / 1e4 * (e[j] + x[j]) + 2 * COST_SH) * sh
                    pnl = gross - c
                    trades.append({"sym": STOCKS[bucket[j]], "side": side, "date": date.isoformat(),
                                    "year": date.year, "pnl": float(pnl), "bps": float(pnl / (e[j] * sh) * 1e4),
                                    "gross_bps": float(gross / (e[j] * sh) * 1e4), "overnight": False, "half": h})
    return trades, daily_ls


def mechanism_fm(common, R, k, halves):
    """Fama-MacBeth: per (day, half) cross-sectional OLS slope of today's return on the lagged
    signal across names; average slope and its t-stat across days, by half-hour."""
    SIG = signal_array(R, k)
    out = {}
    for h in halves:
        gammas = []
        for d in range(40, R.shape[1]):
            x, y = SIG[:, d, h], R[:, d, h]
            ok = ~np.isnan(x) & ~np.isnan(y)
            if ok.sum() < 10:
                continue
            xm, ym = x[ok] - x[ok].mean(), y[ok] - y[ok].mean()
            den = float((xm * xm).sum())
            if den > 0:
                gammas.append(float((xm * ym).sum() / den))
        g = np.array(gammas)
        out[h] = {"n_days": len(g), "gamma": round(float(g.mean()), 4) if len(g) else None,
                   "t": round(float(g.mean() / g.std(ddof=1) * np.sqrt(len(g))), 2) if len(g) > 1 and g.std() > 0 else None}
    return out


def permutation_control(daily_ls, draws=1000):
    """shuffle which day's long-short return is attributed to which day (breaks any true temporal
    signal while preserving the return distribution) -- p = share of shuffles with mean >= observed."""
    vals = np.array([v for lst in daily_ls.values() for v in lst])
    if len(vals) < 10:
        return None
    obs = float(vals.mean())
    rng = np.random.default_rng(2)
    # a day-permutation of the SAME vector can't move its own mean, so the live null is a random
    # sign flip per day: is the long-short return's mean reliably signed, or could an equal-size
    # coin-flip-signed sample produce it (a nonparametric sign test on the daily long-short series)?
    sims = np.array([(rng.choice([-1, 1], len(vals)) * vals).mean() for _ in range(draws)])
    return {"obs_mean_gross": round(obs, 6), "p_sign_flip": round(float((sims >= obs).mean()), 4), "draws": draws}


def cell_hks():
    common, R = build_halfhour_grid()
    grid, all_points = {}, []
    for k in HKS_KS:
        SIG = signal_array(R, k)
        for bname, bfrac in HKS_BUCKETS.items():
            for scope_name, halves in (("all13", HKS_HALVES_ALL), ("openclose", HKS_HALVES_OC)):
                trades, daily_ls = cross_sectional_trades(common, R, SIG, bfrac, halves)
                lab = f"k={k},bucket={bname},halves={scope_name}"
                m = metrics(trades, len(common))
                grid[lab] = m
                all_points.append(m)
    by_p_loyo = {k2: v for k2, v in grid.items()}
    lo = loyo(by_p_loyo)
    best_label, best_m, cleared = pick_best(grid)
    # mechanism check on the best k, both scopes, per half-hour
    k_of_best = best_label.split(",")[0].split("=")[1]
    k_val = int(k_of_best) if k_of_best != "avg40" else "avg40"
    fm_all = mechanism_fm(common, R, k_val, HKS_HALVES_ALL)
    fm_avg40_all = mechanism_fm(common, R, "avg40", HKS_HALVES_ALL)
    # permutation control on the best grid point
    SIG_best = signal_array(R, k_val)
    bfrac_best = HKS_BUCKETS[best_label.split("bucket=")[1].split(",")[0]]
    halves_best = HKS_HALVES_ALL if "all13" in best_label else HKS_HALVES_OC
    _, daily_ls_best = cross_sectional_trades(common, R, SIG_best, bfrac_best, halves_best)
    perm = permutation_control(daily_ls_best)
    # mechanism verdict: does periodicity exist at all? avg40, all 13 halves, |t|>=2 on >=2 halves (incl open/close)
    sig_halves = [h for h, d in fm_avg40_all.items() if d["t"] is not None and abs(d["t"]) >= 2]
    mech_verdict = ("mechanism present" if len(sig_halves) >= 2 else "mechanism not detected",
                     f"{len(sig_halves)}/13 half-hours clear |t|>=2 on the avg40 FM slope (half-hours: {sig_halves})")
    trade_verdict, trade_why = classify(all_points, best_m, cleared)
    # per-half-hour net/gross bps + per-year, on avg40/decile/all13 (the paper's own signal definition)
    SIG_avg40 = signal_array(R, "avg40")
    trades_avg40_all, daily_ls_avg40 = cross_sectional_trades(common, R, SIG_avg40, 0.1, HKS_HALVES_ALL)
    by_half = {}
    for h in range(13):
        ht = [t for t in trades_avg40_all if t["half"] == h]
        hm = metrics(ht, len(common))
        ls = np.array([v for (yy, hh), lst in daily_ls_avg40.items() if hh == h for v in lst])
        t_stat = round(float(ls.mean() / ls.std(ddof=1) * np.sqrt(len(ls))), 2) if len(ls) > 1 and ls.std() > 0 else None
        by_half[h] = {"n": hm["n"], "net_bps": hm.get("bps"), "gross_bps": hm.get("gross_bps"), "pf": hm.get("pf"),
                       "years_pos": hm.get("years_pos"), "long_short_gross_t": t_stat}
    res = {"id": "c12_hks_cross_sectional", "anomaly": "#6 HKS half-hour periodicity, cross-sectional (paper's own design)",
           "names": STOCKS, "n_names": len(STOCKS), "n_days": len(common), "cells_scanned": len(grid),
           "grid_dims": {"k": [str(k) for k in HKS_KS], "bucket": list(HKS_BUCKETS), "halves": ["all13", "openclose"]},
           "grid": grid, "loyo": lo, "best_label": best_label, "best": best_m,
           "by_half_hour_avg40_decile_all13": by_half,
           "fm_mechanism_by_half_hour": {"k=avg40": fm_avg40_all, f"k={k_of_best}(best-tradable-k)": fm_all},
           "permutation_control": perm, "mechanism_verdict": mech_verdict[0], "mechanism_why": mech_verdict[1],
           "tradability_verdict": trade_verdict, "tradability_why": trade_why}
    dump("c12_hks_cross_sectional", res)
    return res

# ======================================================================================
# (c) c13_earnings_gap_large -- #15 earnings-day gap fade, large sample. PRE-REGISTRATION: every
# name with BOTH minute bars in data/bars_iex AND an earnings-date file in research/swing/earnings/
# or research/entries/data/earnings_<T>.txt (30 of the 34 STOCKS names; COIN/PLTR/SHOP/UBER have no
# earnings file and are excluded, not silently zero-filled). Reaction day = first session strictly
# after the filing date (same convention cells.py's earn_gap() uses for the 4-name cell -- all
# these names report after the close per that convention; not independently re-verified per name,
# noted as a limitation). Grid: gap threshold {any, >=1%, >=2%, >=4%} x direction {fade, follow} x
# entry {open, 09:45, 10:00} x exit {11:30, 14:00, close} x gap measure {absolute, relative to
# SPY's same-day gap} = 4 x 2 x 3 x 3 x 2 = 144 cells, pooled across the 30 names (one book). Kill:
# net positive in >= 3/5 years (thin sample, same bar round 1's c05 used) and PF > 1.3. Control:
# same fade/follow rule's mean bps on ALL sessions (not just reaction days) for the 30 names vs the
# reaction-day sample (5000 random draws, no replacement) -- is the earnings day special, or would
# any day's gap fade do as well?
# ======================================================================================
EARN_UNIVERSE = [s for s in STOCKS if os.path.exists(f"{ROOT}/research/swing/earnings/{s}.txt")
                  or os.path.exists(f"{ROOT}/research/entries/data/earnings_{s}.txt")]
GAP_THRESH = ["any", 0.01, 0.02, 0.04]
GAP_DIR = ["fade", "follow"]
GAP_ENTRY = {"open": 0, "0945": 15, "1000": 30}
GAP_EXIT = {"1130": 120, "1400": 270, "close": 390}
GAP_MEASURE = ["absolute", "relative"]
EARN_GRID = [{"thresh": t, "dir": d, "entry": en, "exit": ex, "measure": me}
             for t in GAP_THRESH for d in GAP_DIR for en in GAP_ENTRY for ex in GAP_EXIT for me in GAP_MEASURE]

_EARN_IDX, _SPY_GAP = {}, None
def reaction_idx(sym):
    """union of the two earnings-date file conventions (whichever exist), -> set of SESSION
    INDICES (not dates) for the symbol's own Bars, dropping the one known pre-announcement
    (NVDA 2022-08-08, per cells.py's note)."""
    if sym not in _EARN_IDX:
        b = bars(sym)
        files = [f"{ROOT}/research/swing/earnings/{sym}.txt", f"{ROOT}/research/entries/data/earnings_{sym}.txt"]
        ds = {dt.date.fromisoformat(x) for f in files if os.path.exists(f) for x in open(f).read().split()}
        ds -= {dt.date(2022, 8, 8)}
        out = set()
        for d in ds:
            i = next((k for k, x in enumerate(b.dates) if x > d), None)
            if i is not None and d >= b.dates[0]:
                out.add(i)
        _EARN_IDX[sym] = out
    return _EARN_IDX[sym]


def spy_gap_by_date():
    global _SPY_GAP
    if _SPY_GAP is None:
        b = bars("SPY")
        _SPY_GAP = {}
        for i in range(1, len(b.dates)):
            o, c = b.session_open(i), b.session_close(i - 1)
            if np.isfinite(o) and np.isfinite(c) and c:
                _SPY_GAP[b.dates[i]] = o / c - 1
    return _SPY_GAP


def earn_gap_rule(b, i, p):
    if i < 1 or i not in reaction_idx(b.sym):
        return []
    o, c = b.session_open(i), b.session_close(i - 1)
    if not (np.isfinite(o) and np.isfinite(c)) or c == 0:
        return []
    g = o / c - 1
    if p["measure"] == "relative":
        sg = spy_gap_by_date().get(b.dates[i])
        if sg is None:
            return []
        g -= sg
    if g == 0 or (p["thresh"] != "any" and abs(g) < p["thresh"]):
        return []
    side = (-1 if p["dir"] == "fade" else 1) * int(np.sign(g))
    return [Order(i, GAP_ENTRY[p["entry"]], side, i, GAP_EXIT[p["exit"]])]


def earn_gap_rule_anyday(b, i, p):
    """control rule: same fade/follow-the-gap logic, but fired on EVERY session (no earnings-date
    gate) -- the random-session control pool."""
    if i < 1:
        return []
    o, c = b.session_open(i), b.session_close(i - 1)
    if not (np.isfinite(o) and np.isfinite(c)) or c == 0:
        return []
    g = o / c - 1
    if p["measure"] == "relative":
        sg = spy_gap_by_date().get(b.dates[i])
        if sg is None:
            return []
        g -= sg
    if g == 0 or (p["thresh"] != "any" and abs(g) < p["thresh"]):
        return []
    side = (-1 if p["dir"] == "fade" else 1) * int(np.sign(g))
    return [Order(i, GAP_ENTRY[p["entry"]], side, i, GAP_EXIT[p["exit"]])]


def cell_earnings():
    by_p = {}
    for p in EARN_GRID:
        trades, sessions = run_rule(earn_gap_rule, EARN_UNIVERSE, p)
        by_p[plabel(p)] = metrics(trades, sessions)
    lo = loyo(by_p)
    best_label, best_m, cleared = pick_best(by_p, kill_years=3)
    best_p = next(pp for pp in EARN_GRID if plabel(pp) == best_label)
    obs_trades, _ = run_rule(earn_gap_rule, EARN_UNIVERSE, best_p)
    pool_trades, _ = run_rule(earn_gap_rule_anyday, EARN_UNIVERSE, best_p)
    ctrl = random_control([t["bps"] for t in pool_trades], [t["bps"] for t in obs_trades])
    conc = concentration(obs_trades)
    verdict, why = classify(list(by_p.values()), best_m, cleared, kill_years=3, conc=conc)
    res = {"id": "c13_earnings_gap_large", "anomaly": "#15 earnings-day gap fade, large sample",
           "names": EARN_UNIVERSE, "n_names": len(EARN_UNIVERSE),
           "excluded_no_earnings_file": [s for s in STOCKS if s not in EARN_UNIVERSE],
           "n_events": sum(len(reaction_idx(s)) for s in EARN_UNIVERSE), "cells_scanned": len(EARN_GRID),
           "grid_dims": {"thresh": [str(t) for t in GAP_THRESH], "dir": GAP_DIR, "entry": list(GAP_ENTRY),
                         "exit": list(GAP_EXIT), "measure": GAP_MEASURE},
           "grid": by_p, "loyo": lo, "best_label": best_label, "best": best_m, "concentration": conc,
           "random_session_control": ctrl, "verdict": verdict, "verdict_text": why}
    dump("c13_earnings_gap_large", res)
    return res

# ======================================================================================
# (d) c14_turn_of_month_grid -- #17 turn-of-month, day-of-cycle neighbourhood. PRE-REGISTRATION:
# day-of-cycle in {-2, -1, 0, +1, +2, +3, +4} (0 = last session of the month; -1/-2 = the 2
# sessions before it; +1..+4 = the 1st-4th session of the new month) tested INDIVIDUALLY, plus 4
# pre-chosen WINDOWS ([-1..0], [0..+1], [-1..+1], [-2..+4]) x exit {open->close, open->11:30} x
# instrument {SPY, QQQ, IWM} = (7 + 4) x 2 x 3 = 66 cells. Kill: net positive in >= 4/5 years and
# PF > 1.3. Control: calendar_control-style -- the window's mean bps vs 5000 random same-size,
# same-type (open->close or open->11:30) draws from ALL sessions of that instrument.
# ======================================================================================
TOM_DAYS_SINGLE = [-2, -1, 0, 1, 2, 3, 4]
TOM_WINDOWS = {"[-1,0]": (-1, 0), "[0,1]": (0, 1), "[-1,1]": (-1, 0, 1), "[-2,4]": (-2, -1, 0, 1, 2, 3, 4)}
TOM_EXIT = {"close": 390, "1130": 120}
TOM_INSTR = ["SPY", "QQQ", "IWM"]

_DOC = {}
def day_of_cycle(b):
    if b.sym not in _DOC:
        n = len(b.dates)
        doc = np.full(n, np.nan)
        ends = [i for i in range(n) if i + 1 >= n or b.dates[i + 1].month != b.dates[i].month]
        for e in ends:
            for off in range(0, 3):
                if e - off >= 0:
                    doc[e - off] = -off
            for off in range(1, 5):
                if e + off < n:
                    doc[e + off] = off
        _DOC[b.sym] = doc
    return _DOC[b.sym]


def make_tom_rule(days, t_out):
    def rule(b, i, p):
        doc = day_of_cycle(b)
        return [Order(i, 0, 1, i, t_out)] if (not np.isnan(doc[i]) and doc[i] in days) else []
    return rule


def tom_anyday_rule(t_out):
    def rule(b, i, p):
        return [Order(i, 0, 1, i, t_out)]
    return rule


def cell_turn_of_month():
    by_inst, params = {}, {}
    for inst in TOM_INSTR:
        by_p = {}
        for d in TOM_DAYS_SINGLE:
            for ex_name, t_out in TOM_EXIT.items():
                lab = f"day={d:+d},exit={ex_name}"
                trades, sessions = run_rule(make_tom_rule((d,), t_out), [inst], {})
                by_p[lab] = metrics(trades, sessions)
                params[lab] = ((d,), t_out)
        for wname, days in TOM_WINDOWS.items():
            for ex_name, t_out in TOM_EXIT.items():
                lab = f"window={wname},exit={ex_name}"
                trades, sessions = run_rule(make_tom_rule(days, t_out), [inst], {})
                by_p[lab] = metrics(trades, sessions)
                params[lab] = (days, t_out)
        by_inst[inst] = by_p
    cells_scanned = sum(len(v) for v in by_inst.values())
    # which days carry it: SPY, open->close, single days only
    which_days = {f"day {d:+d}": by_inst["SPY"][f"day={d:+d},exit=close"] for d in TOM_DAYS_SINGLE}
    lo = {inst: loyo(by_p) for inst, by_p in by_inst.items()}
    combined = {f"{inst}:{lab}": m for inst, by_p in by_inst.items() for lab, m in by_p.items()}
    best_key, m_b, cleared = pick_best(combined)
    inst_b, lab_b = best_key.split(":", 1)
    # random-session control for the best (inst, days, exit) vs all sessions of that instrument
    days_b, t_out_b = params[lab_b]
    obs_trades, _ = run_rule(make_tom_rule(days_b, t_out_b), [inst_b], {})
    pool_trades, _ = run_rule(tom_anyday_rule(t_out_b), [inst_b], {})
    ctrl = random_control([t["bps"] for t in pool_trades], [t["bps"] for t in obs_trades])
    conc = concentration(obs_trades)
    all_points = [m for by_p in by_inst.values() for m in by_p.values()]
    verdict, why = classify(all_points, m_b, cleared, conc=conc)
    res = {"id": "c14_turn_of_month_grid", "anomaly": "#17 turn-of-month, day-of-cycle neighbourhood",
           "cells_scanned": cells_scanned, "grid_dims": {"days_single": TOM_DAYS_SINGLE, "windows": TOM_WINDOWS,
           "exit": list(TOM_EXIT), "instruments": TOM_INSTR}, "by_instrument": by_inst, "loyo": lo,
           "which_days_carry_it_SPY_close": which_days, "best_instrument": inst_b, "best_label": lab_b,
           "best": m_b, "random_session_control": ctrl, "concentration": conc, "verdict": verdict, "verdict_text": why}
    dump("c14_turn_of_month_grid", res)
    return res

# ======================================================================================
# (e) c15_fomc_variants -- #9 pre-FOMC drift neighbourhood. PRE-REGISTRATION: the catalog's rule
# (overnight 14:00(d-1)->14:00(d), flagged non-tradable) plus FIVE variants x SPY/QQQ/IWM = 18
# cells: FOMC-day 09:30->14:00, day-before open->close ("day-before intraday", same trade as the
# catalog's "prior session" variant -- kept for completeness), post-announcement 14:00->close same
# day, and next-day open->close. Kill (tradable variants): net positive in >= 4/5 years and PF >
# 1.3. Decay check: gross bps in 2022-2024 pooled vs 2025-2026 pooled, per instrument, per variant
# -- "decayed" must be a measured split, not a label. Control: calendar_control (5000 random-day
# draws of the same trade type, same instrument).
# ======================================================================================
FOMC_INSTR = ["SPY", "QQQ", "IWM"]


def nxt_fomc(b, i):
    return b.dates[i + 1] if i + 1 < len(b.dates) else None


def fomc_paper(b, i, p):
    return [Order(i, 270, 1, i + 1, 270)] if nxt_fomc(b, i) in FOMC else []


def fomc_day_am(b, i, p):
    return [Order(i, 0, 1, i, 270)] if b.dates[i] in FOMC else []


def fomc_day_before(b, i, p):
    return [Order(i, 0, 1, i, 390)] if nxt_fomc(b, i) in FOMC else []


def fomc_post_announcement(b, i, p):
    return [Order(i, 270, 1, i, 390)] if b.dates[i] in FOMC else []


def fomc_next_day(b, i, p):
    return [Order(i + 1, 0, 1, i + 1, 390)] if b.dates[i] in FOMC and i + 1 < len(b.dates) else []


def fomc_control_rule(t_in, t_out, days=0):
    def rule(b, i, p):
        return [Order(i, t_in, 1, i + days, t_out)] if i + days < len(b.dates) else []
    return rule


FOMC_VARIANTS = [
    ("paper 14:00(d-1)->14:00(d)", fomc_paper, True, fomc_control_rule(270, 270, 1)),
    ("FOMC day 09:30->14:00", fomc_day_am, False, fomc_control_rule(0, 270)),
    ("day-before intraday open->close", fomc_day_before, False, fomc_control_rule(0, 390)),
    ("post-announcement 14:00->close (same day)", fomc_post_announcement, False, fomc_control_rule(270, 390)),
    ("next-day open->close", fomc_next_day, False, fomc_control_rule(0, 390)),
]


def decay_split(trades):
    early = [t["bps"] for t in trades if t["year"] in (2022, 2023, 2024)]
    late = [t["bps"] for t in trades if t["year"] in (2025, 2026)]
    return {"gross_early_22_24_bps": round(float(np.mean([t["gross_bps"] for t in trades if t["year"] in (2022,2023,2024)])), 2) if early else None,
            "gross_late_25_26_bps": round(float(np.mean([t["gross_bps"] for t in trades if t["year"] in (2025,2026)])), 2) if late else None,
            "n_early": len(early), "n_late": len(late)}


def cell_fomc():
    by_p, decay = {}, {}
    for name, rule, flagged, ctrl_rule in FOMC_VARIANTS:
        for inst in FOMC_INSTR:
            trades, sessions = run_rule(rule, [inst], {})
            lab = f"{name}|{inst}"
            by_p[lab] = metrics(trades, sessions)
            decay[lab] = decay_split(trades)
    # tradable-only pool for the verdict (the paper's overnight window needs a product we don't have)
    tradable = {lab: m for lab, m in by_p.items() if not lab.startswith(FOMC_VARIANTS[0][0])}
    lo = loyo(tradable) if tradable else None
    best_label, best_m, cleared = pick_best(tradable)
    best_name, best_inst = best_label.split("|")
    _, rule_b, _, ctrl_b = next(v for v in FOMC_VARIANTS if v[0] == best_name)
    obs_trades, _ = run_rule(rule_b, [best_inst], {})
    pool_trades, _ = run_rule(ctrl_b, [best_inst], {})
    ctrl = random_control([t["bps"] for t in pool_trades], [t["bps"] for t in obs_trades])
    conc = concentration(obs_trades)
    verdict, why = classify(list(tradable.values()), best_m, cleared, conc=conc)
    # the paper's own (non-tradable) overnight window, for the "needs-product" context
    paper_rows = {lab: by_p[lab] for lab in by_p if lab.startswith(FOMC_VARIANTS[0][0])}
    res = {"id": "c15_fomc_variants", "anomaly": "#9 pre-FOMC drift, variant + instrument neighbourhood",
           "cells_scanned": len(by_p), "n_fomc_days": len(FOMC), "instruments": FOMC_INSTR,
           "grid": by_p, "decay_split_gross_bps": decay, "paper_overnight_window_flag_not_tradable": paper_rows,
           "loyo_tradable": lo, "best_label": best_label, "best": best_m, "random_session_control": ctrl,
           "concentration": conc, "verdict": verdict, "verdict_text": why}
    dump("c15_fomc_variants", res)
    return res


def main():
    ids = {"c11": cell_orb, "c12": cell_hks, "c13": cell_earnings, "c14": cell_turn_of_month, "c15": cell_fomc}
    want = sys.argv[1:] or list(ids)
    results = []
    for prefix, fn in ids.items():
        if any(w.startswith(prefix) or prefix.startswith(w) for w in want):
            results.append(fn())
    for r in results:
        v = r.get("verdict") or r.get("tradability_verdict")
        print(f"{r['id']}: cells_scanned={r.get('cells_scanned') or r.get('cells_scanned_qqq')} "
              f"best={r.get('best_label')} verdict={v}")
    return results


if __name__ == "__main__":
    main()
