#!/usr/bin/env python3
"""
Crash-days hindsight study (research/crash_days/2026-09-27_crash_days_hindsight.md).

Read-only. No rust, no sweeps, no db. Inputs:
  - data/bars_iex/{SPY,AAPL,AMZN,MSFT,NVDA}.csv  1-min RTH bars, 2022-01-03..latest
  - research/swing/daily/{SPY,AAPL,AMZN,MSFT,NVDA}.csv  daily bars, 2021-06-01..latest
  - crates/indicators/src/custom/event_calendar.rs  FOMC_DECISION_DAYS (hardcoded below,
    copied verbatim from that file)
  - research/entries/data/earnings_{AAPL,AMZN,MSFT,NVDA}.txt  announcement dates
  - data/iex_v18_<year>_trades.csv  the live short book's 5-year replay (row_type=trade)

Run: python3 research/crash_days/analysis.py
Prints all tables used in the companion .md, in order. No external deps beyond numpy.
"""
import csv
import datetime
import math
import statistics
from collections import defaultdict
from zoneinfo import ZoneInfo

import numpy as np

ROOT = "/home/dylmet/Projects/galactic_trading_firm"
NY = ZoneInfo("America/New_York")
TICKERS = ["SPY", "AAPL", "AMZN", "MSFT", "NVDA"]
NAMES = ["AAPL", "AMZN", "MSFT", "NVDA"]

# copied verbatim from crates/indicators/src/custom/event_calendar.rs
FOMC_DECISION_DAYS = {
    "2022-01-26", "2022-03-16", "2022-05-04", "2022-06-15", "2022-07-27", "2022-09-21", "2022-11-02", "2022-12-14",
    "2023-02-01", "2023-03-22", "2023-05-03", "2023-06-14", "2023-07-26", "2023-09-20", "2023-11-01", "2023-12-13",
    "2024-01-31", "2024-03-20", "2024-05-01", "2024-06-12", "2024-07-31", "2024-09-18", "2024-11-07", "2024-12-18",
    "2025-01-29", "2025-03-19", "2025-05-07", "2025-06-18", "2025-07-30", "2025-09-17", "2025-10-29", "2025-12-10",
    "2026-01-28", "2026-03-18", "2026-04-29", "2026-06-17", "2026-07-29", "2026-09-16",
}


def parse_date(s):
    return datetime.date.fromisoformat(s)


def load_daily(ticker):
    path = f"{ROOT}/research/swing/daily/{ticker}.csv"
    rows = []
    with open(path) as f:
        r = csv.DictReader(f)
        for row in r:
            rows.append((row["date"], float(row["open"]), float(row["high"]),
                         float(row["low"]), float(row["close"]), float(row["volume"])))
    rows.sort(key=lambda x: x[0])
    return rows


def load_intraday(ticker):
    """returns dict date-> sorted list of (time 'HH:MM', o,h,l,c,v)"""
    path = f"{ROOT}/data/bars_iex/{ticker}.csv"
    byday = defaultdict(list)
    with open(path) as f:
        r = csv.DictReader(f)
        for row in r:
            ts = int(row["timestamp"])
            dt = datetime.datetime.fromtimestamp(ts, tz=NY)
            d = dt.date().isoformat()
            byday[d].append((dt.strftime("%H:%M"), float(row["open"]), float(row["high"]),
                              float(row["low"]), float(row["close"]), float(row["volume"])))
    for d in byday:
        byday[d].sort(key=lambda x: x[0])
    return byday


def load_earnings(ticker):
    path = f"{ROOT}/research/entries/data/earnings_{ticker}.txt"
    out = set()
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                out.add(line)
    return out


def load_v18_trades():
    """dict date -> list of trade dicts (row_type=='trade')"""
    out = defaultdict(list)
    for year in [2022, 2023, 2024, 2025, 2026]:
        path = f"{ROOT}/data/iex_v18_{year}_trades.csv"
        with open(path) as f:
            r = csv.DictReader(f)
            for row in r:
                if row["row_type"] != "trade":
                    continue
                out[row["date"]].append(row)
    return out


def build_spliced_daily(ticker, intraday):
    """combined daily series: intraday-derived (open,high,low,close,vol) where bars_iex has
    the date, else the daily file's own OHLCV. Returns sorted list of
    (date, o, h, l, c, v)."""
    daily = load_daily(ticker)
    combined = {}
    for (d, o, h, l, c, v) in daily:
        combined[d] = (o, h, l, c, v)
    for d, bars in intraday.items():
        o = bars[0][1]
        h = max(b[2] for b in bars)
        l = min(b[3] for b in bars)
        c = bars[-1][4]
        v = sum(b[5] for b in bars)
        combined[d] = (o, h, l, c, v)
    dates = sorted(combined.keys())
    return [(d,) + combined[d] for d in dates]


def pct(a, b):
    return (a / b - 1.0) * 100.0


def wilson_note():
    pass


def main():
    print("Loading data...")
    intraday = {t: load_intraday(t) for t in TICKERS}
    daily_spliced = {t: build_spliced_daily(t, intraday[t]) for t in TICKERS}
    earnings = {t: load_earnings(t) for t in NAMES}
    v18 = load_v18_trades()

    spy_series = daily_spliced["SPY"]
    spy_dates = [r[0] for r in spy_series]
    spy_idx = {d: i for i, d in enumerate(spy_dates)}
    spy_close = np.array([r[4] for r in spy_series])
    spy_open = np.array([r[1] for r in spy_series])
    spy_high = np.array([r[2] for r in spy_series])
    spy_low = np.array([r[3] for r in spy_series])

    # analysis universe: dates present in bars_iex for SPY (2022-01-03 on)
    universe = sorted(intraday["SPY"].keys())
    print(f"universe: {universe[0]}..{universe[-1]}, {len(universe)} sessions")

    # ---- daily-return classification (session = bars_iex open->close, matches stress study) ----
    def session_oc_ret(t, d):
        bars = intraday[t].get(d)
        if not bars:
            return None
        return pct(bars[-1][4], bars[0][1])

    def session_cc_ret(t, d):
        i = spy_idx.get(d)
        if i is None or i == 0:
            return None
        return pct(daily_spliced[t if False else "SPY"][i][4], daily_spliced["SPY"][i - 1][4]) if t == "SPY" else None

    # generic close-to-close using a ticker's own spliced series
    ticker_idx = {t: {r[0]: i for i, r in enumerate(daily_spliced[t])} for t in TICKERS}

    def cc_ret(t, d):
        idx = ticker_idx[t]
        i = idx.get(d)
        if i is None or i == 0:
            return None
        return pct(daily_spliced[t][i][4], daily_spliced[t][i - 1][4])

    oc = {d: session_oc_ret("SPY", d) for d in universe}
    cc = {d: cc_ret("SPY", d) for d in universe}

    def classify(ret):
        if ret is None:
            return None
        if ret < -2.0:
            return "<-2%"
        if ret < -1.0:
            return "-2..-1%"
        if ret < -0.3:
            return "-1..-0.3%"
        if ret <= 0.3:
            return "flat"
        return ">+0.3%"

    classes = ["<-2%", "-2..-1%", "-1..-0.3%", "flat", ">+0.3%"]
    print("\n=== Session count by open->close class ===")
    counts_oc = defaultdict(int)
    for d in universe:
        counts_oc[classify(oc[d])] += 1
    for c in classes:
        print(f"  {c:>10}: {counts_oc[c]:4d}")
    print(f"  total: {len(universe)}")

    print("\n=== Session count by close->close class ===")
    counts_cc = defaultdict(int)
    for d in universe:
        counts_cc[classify(cc[d])] += 1
    for c in classes:
        print(f"  {c:>10}: {counts_cc[c]:4d}")

    days_lt1 = sorted([d for d in universe if oc[d] is not None and oc[d] < -1.0])
    days_lt2 = sorted([d for d in universe if oc[d] is not None and oc[d] < -2.0])
    print(f"\nSPY open->close < -1%: {len(days_lt1)} days;  < -2%: {len(days_lt2)} days")
    by_year = defaultdict(int)
    for d in days_lt1:
        by_year[d[:4]] += 1
    print("  by year (<-1%):", dict(sorted(by_year.items())))

    # ============================================================
    # PART 1: PRECURSOR FEATURES
    # ============================================================
    print("\n\n########## PART 1: PRECURSORS ##########")

    # rolling helpers on SPY spliced series, indexed by position i (0-based)
    n = len(spy_series)
    log_ret = np.full(n, np.nan)
    for i in range(1, n):
        log_ret[i] = math.log(spy_close[i] / spy_close[i - 1])

    realized_vol_20 = np.full(n, np.nan)  # annualized %, uses returns [i-20..i-1], known as of close i-1
    for i in range(21, n):
        window = log_ret[i - 20:i]
        realized_vol_20[i] = np.std(window, ddof=1) * math.sqrt(252) * 100

    vol_pctile = np.full(n, np.nan)  # trailing-1yr percentile rank of realized_vol_20[i-1] as of day i
    for i in range(21 + 252, n):
        hist = realized_vol_20[i - 252:i]
        hist = hist[~np.isnan(hist)]
        if len(hist) > 50:
            vol_pctile[i] = (np.sum(hist <= realized_vol_20[i - 1]) / len(hist)) * 100

    ma50 = np.full(n, np.nan)
    ma200 = np.full(n, np.nan)
    for i in range(50, n):
        ma50[i] = np.mean(spy_close[i - 50:i])
    for i in range(200, n):
        ma200[i] = np.mean(spy_close[i - 200:i])

    high20 = np.full(n, np.nan)
    high252 = np.full(n, np.nan)
    for i in range(20, n):
        high20[i] = np.max(spy_high[i - 20:i])
    for i in range(252, n):
        high252[i] = np.max(spy_high[i - 252:i])

    # feature dict: date -> value, computed only over `universe`
    feat = {name: {} for name in [
        "gap", "prior1", "prior3", "prior5", "rv20", "rv20_pct",
        "dist20h", "dist52h", "above50", "above200", "dow", "fomc", "earn_am_or_prevpm",
    ]}
    target1 = {}
    target2 = {}
    for d in universe:
        i = spy_idx[d]
        if i < 1:
            continue
        feat["gap"][d] = pct(spy_open[i], spy_close[i - 1])
        if i >= 2:
            feat["prior1"][d] = pct(spy_close[i - 1], spy_close[i - 2])
        if i >= 4:
            feat["prior3"][d] = pct(spy_close[i - 1], spy_close[i - 4])
        if i >= 6:
            feat["prior5"][d] = pct(spy_close[i - 1], spy_close[i - 6])
        if not math.isnan(realized_vol_20[i]):
            feat["rv20"][d] = realized_vol_20[i]
        if not math.isnan(vol_pctile[i]):
            feat["rv20_pct"][d] = vol_pctile[i]
        if not math.isnan(high20[i]):
            feat["dist20h"][d] = pct(spy_close[i - 1], high20[i])
        if not math.isnan(high252[i]):
            feat["dist52h"][d] = pct(spy_close[i - 1], high252[i])
        if not math.isnan(ma50[i]):
            feat["above50"][d] = 1.0 if spy_close[i - 1] > ma50[i] else 0.0
        if not math.isnan(ma200[i]):
            feat["above200"][d] = 1.0 if spy_close[i - 1] > ma200[i] else 0.0
        feat["dow"][d] = parse_date(d).weekday()  # 0=Mon
        feat["fomc"][d] = 1.0 if d in FOMC_DECISION_DAYS else 0.0
        # earnings: flag if any of the 4 names reports ON d, or reported on the prior
        # calendar day (proxy for "prior evening", since BMO/AMC split isn't in the data)
        prev_i = spy_idx[d] - 1
        prev_d = spy_dates[prev_i] if prev_i >= 0 else None
        flag = 0.0
        for nm in NAMES:
            if d in earnings[nm] or (prev_d and prev_d in earnings[nm]):
                flag = 1.0
        feat["earn_am_or_prevpm"][d] = flag
        target1[d] = 1.0 if (oc[d] is not None and oc[d] < -1.0) else 0.0
        target2[d] = 1.0 if (oc[d] is not None and oc[d] < -2.0) else 0.0

    def dist_by_class(fname):
        buckets = defaultdict(list)
        for d in universe:
            v = feat[fname].get(d)
            c = classify(oc[d])
            if v is not None and c is not None:
                buckets[c].append(v)
        print(f"\n--- {fname}: mean (n) by open->close class ---")
        for c in classes:
            vals = buckets[c]
            if vals:
                print(f"  {c:>10}: mean={statistics.mean(vals):7.3f}  median={statistics.median(vals):7.3f}  n={len(vals)}")
            else:
                print(f"  {c:>10}: n=0")

    for fname in ["gap", "prior1", "prior3", "prior5", "rv20", "rv20_pct", "dist20h", "dist52h", "above50", "above200"]:
        dist_by_class(fname)

    # categorical: dow, fomc, earnings — report rate of target1/target2 per level
    print("\n--- day-of-week: P(close<-1%) / P(close<-2%) ---")
    for dow in range(5):
        ds = [d for d in universe if feat["dow"][d] == dow]
        r1 = sum(target1[d] for d in ds) / len(ds) if ds else float("nan")
        r2 = sum(target2[d] for d in ds) / len(ds) if ds else float("nan")
        print(f"  dow={dow} n={len(ds):4d}  P(<-1%)={r1*100:5.2f}%  P(<-2%)={r2*100:5.2f}%")

    print("\n--- FOMC day vs not ---")
    for flag in [1.0, 0.0]:
        ds = [d for d in universe if feat["fomc"][d] == flag]
        r1 = sum(target1[d] for d in ds) / len(ds) if ds else float("nan")
        print(f"  fomc={flag} n={len(ds):4d}  P(<-1%)={r1*100:5.2f}%")

    print("\n--- earnings (any of 4 names, day-of or prior calendar day) ---")
    for flag in [1.0, 0.0]:
        ds = [d for d in universe if feat["earn_am_or_prevpm"][d] == flag]
        r1 = sum(target1[d] for d in ds) / len(ds) if ds else float("nan")
        print(f"  flag={flag} n={len(ds):4d}  P(<-1%)={r1*100:5.2f}%")

    base_rate1 = sum(target1.values()) / len(target1)
    base_rate2 = sum(target2.values()) / len(target2)
    print(f"\nbase rate P(close<-1%)={base_rate1*100:.2f}%   P(close<-2%)={base_rate2*100:.2f}%")

    def best_threshold(fname, target, direction=None, min_n=30):
        """sweep candidate thresholds (unique feature values), find the one maximizing F1
        for predicting target==1. direction: 'below' means feature<=thr predicts positive,
        'above' means feature>=thr predicts positive; if None, try both and keep the better."""
        pairs = [(feat[fname][d], target[d]) for d in universe if d in feat[fname]]
        if len(pairs) < min_n:
            return None
        vals = sorted(set(v for v, _ in pairs))
        best = None
        dirs = [direction] if direction else ["below", "above"]
        for dr in dirs:
            for thr in vals:
                if dr == "below":
                    pred = [1 if v <= thr else 0 for v, _ in pairs]
                else:
                    pred = [1 if v >= thr else 0 for v, _ in pairs]
                tp = sum(1 for p, (_, t) in zip(pred, pairs) if p == 1 and t == 1.0)
                fp = sum(1 for p, (_, t) in zip(pred, pairs) if p == 1 and t == 0.0)
                fn = sum(1 for p, (_, t) in zip(pred, pairs) if p == 0 and t == 1.0)
                fired = tp + fp
                if fired < 5:
                    continue
                precision = tp / fired if fired else 0
                recall = tp / (tp + fn) if (tp + fn) else 0
                f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0
                if best is None or f1 > best[0]:
                    best = (f1, dr, thr, precision, recall, fired, tp)
        return best

    def loyo_check(fname, target, direction, thr_full):
        """leave-one-year-out: fix direction, refit threshold on 4 years, test on held-out
        year; report each fold's test precision/recall using ITS OWN best-fit threshold
        (honest: threshold chosen without seeing the test year)."""
        years = sorted(set(d[:4] for d in universe))
        fold_results = []
        for hold in years:
            train_pairs = [(feat[fname][d], target[d]) for d in universe if d in feat[fname] and d[:4] != hold]
            test_pairs = [(feat[fname][d], target[d]) for d in universe if d in feat[fname] and d[:4] == hold]
            if len(train_pairs) < 30 or len(test_pairs) < 10:
                continue
            vals = sorted(set(v for v, _ in train_pairs))
            best = None
            for thr in vals:
                if direction == "below":
                    pred = [1 if v <= thr else 0 for v, _ in train_pairs]
                else:
                    pred = [1 if v >= thr else 0 for v, _ in train_pairs]
                tp = sum(1 for p, (_, t) in zip(pred, train_pairs) if p == 1 and t == 1.0)
                fp = sum(1 for p, (_, t) in zip(pred, train_pairs) if p == 1 and t == 0.0)
                fn = sum(1 for p, (_, t) in zip(pred, train_pairs) if p == 0 and t == 1.0)
                fired = tp + fp
                if fired < 3:
                    continue
                precision = tp / fired if fired else 0
                recall = tp / (tp + fn) if (tp + fn) else 0
                f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0
                if best is None or f1 > best[0]:
                    best = (f1, thr)
            if best is None:
                continue
            thr = best[1]
            if direction == "below":
                pred = [1 if v <= thr else 0 for v, _ in test_pairs]
            else:
                pred = [1 if v >= thr else 0 for v, _ in test_pairs]
            tp = sum(1 for p, (_, t) in zip(pred, test_pairs) if p == 1 and t == 1.0)
            fp = sum(1 for p, (_, t) in zip(pred, test_pairs) if p == 1 and t == 0.0)
            fn = sum(1 for p, (_, t) in zip(pred, test_pairs) if p == 0 and t == 1.0)
            fired = tp + fp
            precision = tp / fired if fired else float("nan")
            recall = tp / (tp + fn) if (tp + fn) else float("nan")
            fold_results.append((hold, thr, fired, precision, recall))
        return fold_results

    print("\n=== best single-threshold, full sample (predicting close<-1%) ===")
    for fname in ["gap", "prior1", "prior3", "prior5", "rv20", "rv20_pct", "dist20h", "dist52h"]:
        b = best_threshold(fname, target1)
        if b:
            f1, dr, thr, prec, rec, fired, tp = b
            print(f"  {fname:>10}: {dr:>5} {thr:8.3f}  fired={fired:4d} tp={tp:3d}  "
                  f"precision={prec*100:5.1f}%  recall={rec*100:5.1f}%  lift={prec/base_rate1:4.2f}x")
            loyo = loyo_check(fname, target1, dr, thr)
            precs = [p for *_, p, r in loyo if not math.isnan(p)]
            recs = [r for *_, p, r in loyo if not math.isnan(r)]
            if precs:
                print(f"             LOYO: mean test precision={statistics.mean(precs)*100:5.1f}%  "
                      f"mean test recall={statistics.mean(recs)*100:5.1f}%  "
                      f"per-fold={[ (h, round(t,2), fr, round(p*100,1)) for h,t,fr,p,r in loyo]}")

    print("\n=== best single-threshold, full sample (predicting close<-2%) ===")
    for fname in ["gap", "prior1", "prior3", "prior5", "rv20", "rv20_pct", "dist20h", "dist52h"]:
        b = best_threshold(fname, target2)
        if b:
            f1, dr, thr, prec, rec, fired, tp = b
            print(f"  {fname:>10}: {dr:>5} {thr:8.3f}  fired={fired:4d} tp={tp:3d}  "
                  f"precision={prec*100:5.1f}%  recall={rec*100:5.1f}%  lift={prec/base_rate2:4.2f}x")

    # ============================================================
    # INTRADAY (10:00 / 10:30) FEATURES
    # ============================================================
    print("\n\n########## INTRADAY FEATURES (10:00 / 10:30) ##########")

    def price_at_or_before(bars, t):
        """last bar close at or before time t ('HH:MM'); bars sorted list of (time,o,h,l,c,v)"""
        best = None
        for (bt, o, h, l, c, v) in bars:
            if bt <= t:
                best = c
            else:
                break
        return best

    def bars_up_to(bars, t):
        return [b for b in bars if b[0] <= t]

    def vwap_up_to(bars, t):
        sub = bars_up_to(bars, t)
        num = 0.0
        den = 0.0
        for (bt, o, h, l, c, v) in sub:
            tp = (h + l + c) / 3.0
            num += tp * v
            den += v
        return num / den if den else None

    def atr20_as_of(t_series, idx):
        """average true range, 20d, as of index idx (uses days idx-20..idx-1)"""
        trs = []
        for i in range(max(1, idx - 20), idx):
            h, l, pc = t_series[i][2], t_series[i][3], t_series[i - 1][4]
            trs.append(max(h - l, abs(h - pc), abs(l - pc)))
        return statistics.mean(trs) if trs else None

    intraday_feat = {"1000": {}, "1030": {}}
    for tlabel in ["10:00", "10:30"]:
        key = tlabel.replace(":", "")
        for d in universe:
            spy_bars = intraday["SPY"].get(d)
            if not spy_bars:
                continue
            day_open = spy_bars[0][1]
            price_now = price_at_or_before(spy_bars, tlabel)
            if price_now is None:
                continue
            ret_since_open = pct(price_now, day_open)
            sub = bars_up_to(spy_bars, tlabel)
            range_so_far = max(b[2] for b in sub) - min(b[3] for b in sub)
            i = spy_idx[d]
            atr20 = atr20_as_of(spy_series, i)
            range_vs_atr = range_so_far / atr20 if atr20 else None
            reds = 0
            name_rets = []
            for nm in NAMES:
                nbars = intraday[nm].get(d)
                if not nbars:
                    continue
                nopen = nbars[0][1]
                npx = price_at_or_before(nbars, tlabel)
                if npx is None:
                    continue
                r = pct(npx, nopen)
                name_rets.append(r)
                if r < 0:
                    reds += 1
            pct_red = reds / len(name_rets) if name_rets else None
            mean_name_ret = statistics.mean(name_rets) if name_rets else None
            first15 = pct(price_at_or_before(spy_bars, "09:45"), day_open)
            # new session low in the last 15 minutes vs all-prior-session low
            window_end = tlabel
            window_start_min = int(tlabel[:2]) * 60 + int(tlabel[3:]) - 15
            window_start = f"{window_start_min // 60:02d}:{window_start_min % 60:02d}"
            prior_bars = [b for b in sub if b[0] < window_start]
            last15_bars = [b for b in sub if b[0] >= window_start]
            new_low = None
            if prior_bars and last15_bars:
                prior_low = min(b[3] for b in prior_bars)
                last15_low = min(b[3] for b in last15_bars)
                new_low = 1.0 if last15_low < prior_low else 0.0
            vwap_now = vwap_up_to(spy_bars, tlabel)
            dist_vwap = pct(price_now, vwap_now) if vwap_now else None
            intraday_feat[key][d] = dict(
                ret_since_open=ret_since_open, range_vs_atr=range_vs_atr, pct_red=pct_red,
                mean_name_ret=mean_name_ret, first15=first15, new_low=new_low, dist_vwap=dist_vwap,
            )

    for key, label in [("1000", "10:00"), ("1030", "10:30")]:
        print(f"\n--- distribution by open->close class @ {label} ---")
        for fname in ["ret_since_open", "range_vs_atr", "pct_red", "mean_name_ret", "first15", "dist_vwap"]:
            buckets = defaultdict(list)
            for d, rec in intraday_feat[key].items():
                v = rec.get(fname)
                c = classify(oc[d])
                if v is not None and c is not None:
                    buckets[c].append(v)
            print(f"  {fname}:")
            for c in classes:
                vals = buckets[c]
                if vals:
                    print(f"     {c:>10}: mean={statistics.mean(vals):7.3f}  n={len(vals)}")

    def best_threshold_intraday(key, fname, target, min_n=30):
        pairs = [(intraday_feat[key][d][fname], target[d]) for d in universe
                 if d in intraday_feat[key] and intraday_feat[key][d].get(fname) is not None]
        if len(pairs) < min_n:
            return None
        vals = sorted(set(v for v, _ in pairs))
        best = None
        for dr in ["below", "above"]:
            for thr in vals:
                pred = [1 if (v <= thr if dr == "below" else v >= thr) else 0 for v, _ in pairs]
                tp = sum(1 for p, (_, t) in zip(pred, pairs) if p == 1 and t == 1.0)
                fp = sum(1 for p, (_, t) in zip(pred, pairs) if p == 1 and t == 0.0)
                fn = sum(1 for p, (_, t) in zip(pred, pairs) if p == 0 and t == 1.0)
                fired = tp + fp
                if fired < 5:
                    continue
                precision = tp / fired
                recall = tp / (tp + fn) if (tp + fn) else 0
                f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0
                if best is None or f1 > best[0]:
                    best = (f1, dr, thr, precision, recall, fired, tp)
        return best

    def loyo_check_intraday(key, fname, target, direction, min_n=10):
        years = sorted(set(d[:4] for d in universe))
        fold_results = []
        for hold in years:
            train = [(intraday_feat[key][d][fname], target[d]) for d in universe
                     if d in intraday_feat[key] and intraday_feat[key][d].get(fname) is not None and d[:4] != hold]
            test = [(intraday_feat[key][d][fname], target[d]) for d in universe
                    if d in intraday_feat[key] and intraday_feat[key][d].get(fname) is not None and d[:4] == hold]
            if len(train) < 30 or len(test) < 10:
                continue
            vals = sorted(set(v for v, _ in train))
            best = None
            for thr in vals:
                pred = [1 if (v <= thr if direction == "below" else v >= thr) else 0 for v, _ in train]
                tp = sum(1 for p, (_, t) in zip(pred, train) if p == 1 and t == 1.0)
                fp = sum(1 for p, (_, t) in zip(pred, train) if p == 1 and t == 0.0)
                fn = sum(1 for p, (_, t) in zip(pred, train) if p == 0 and t == 1.0)
                fired = tp + fp
                if fired < 3:
                    continue
                precision = tp / fired
                recall = tp / (tp + fn) if (tp + fn) else 0
                f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0
                if best is None or f1 > best[0]:
                    best = (f1, thr)
            if best is None:
                continue
            thr = best[1]
            pred = [1 if (v <= thr if direction == "below" else v >= thr) else 0 for v, _ in test]
            tp = sum(1 for p, (_, t) in zip(pred, test) if p == 1 and t == 1.0)
            fp = sum(1 for p, (_, t) in zip(pred, test) if p == 1 and t == 0.0)
            fn = sum(1 for p, (_, t) in zip(pred, test) if p == 0 and t == 1.0)
            fired = tp + fp
            precision = tp / fired if fired else float("nan")
            recall = tp / (tp + fn) if (tp + fn) else float("nan")
            fold_results.append((hold, round(thr, 3), fired, round(precision * 100, 1) if fired else None))
        return fold_results

    for key, label in [("1000", "10:00"), ("1030", "10:30")]:
        print(f"\n=== best single-threshold @ {label} (predicting close<-1%) ===")
        for fname in ["ret_since_open", "range_vs_atr", "pct_red", "mean_name_ret", "first15", "dist_vwap"]:
            b = best_threshold_intraday(key, fname, target1)
            if b:
                f1, dr, thr, prec, rec, fired, tp = b
                print(f"  {fname:>16}: {dr:>5} {thr:8.3f}  fired={fired:4d} tp={tp:3d}  "
                      f"precision={prec*100:5.1f}%  recall={rec*100:5.1f}%  lift={prec/base_rate1:4.2f}x")
                loyo = loyo_check_intraday(key, fname, target1, dr)
                precs = [p for *_, p in loyo if p is not None]
                if precs:
                    print(f"                    LOYO mean test precision={statistics.mean(precs):5.1f}%  folds={loyo}")

        print(f"\n=== best single-threshold @ {label} (predicting close<-2%) ===")
        for fname in ["ret_since_open", "range_vs_atr", "pct_red", "mean_name_ret", "first15", "dist_vwap"]:
            b = best_threshold_intraday(key, fname, target2)
            if b:
                f1, dr, thr, prec, rec, fired, tp = b
                print(f"  {fname:>16}: {dr:>5} {thr:8.3f}  fired={fired:4d} tp={tp:3d}  "
                      f"precision={prec*100:5.1f}%  recall={rec*100:5.1f}%  lift={prec/base_rate2:4.2f}x")

    # new_low is binary; handle separately as a rate table
    for key, label in [("1000", "10:00"), ("1030", "10:30")]:
        print(f"\n--- new session low in trailing 15min @ {label}: P(close<-1%) ---")
        for flag in [1.0, 0.0]:
            ds = [d for d in universe if intraday_feat[key].get(d, {}).get("new_low") == flag]
            r1 = sum(target1[d] for d in ds) / len(ds) if ds else float("nan")
            print(f"  new_low={flag} n={len(ds):4d}  P(<-1%)={r1*100:5.2f}%  lift={r1/base_rate1 if ds else float('nan'):4.2f}x")

    # ---- two-feature combination: gap AND rv20_pct (both known at 9:30); ret_since_open@10:00 AND pct_red@10:00 ----
    print("\n=== two-feature combinations ===")

    def combo_eval(pairs_list, target, thr_a, dir_a, thr_b, dir_b):
        tp = fp = fn = 0
        for (a, b, t) in pairs_list:
            hit_a = (a <= thr_a) if dir_a == "below" else (a >= thr_a)
            hit_b = (b <= thr_b) if dir_b == "below" else (b >= thr_b)
            pred = 1 if (hit_a and hit_b) else 0
            if pred == 1 and t == 1.0:
                tp += 1
            elif pred == 1 and t == 0.0:
                fp += 1
            elif pred == 0 and t == 1.0:
                fn += 1
        fired = tp + fp
        precision = tp / fired if fired else float("nan")
        recall = tp / (tp + fn) if (tp + fn) else float("nan")
        return precision, recall, fired, tp

    # gap AND rv20_pct for target1
    pairs_list = []
    for d in universe:
        if d in feat["gap"] and d in feat["rv20_pct"]:
            pairs_list.append((feat["gap"][d], feat["rv20_pct"][d], target1[d]))
    if pairs_list:
        # try a small grid around informative cutoffs
        best_combo = None
        gap_cands = sorted(set(round(v, 2) for v, _, _ in pairs_list))
        rv_cands = sorted(set(round(v, 1) for _, v, _ in pairs_list))
        gap_cands = gap_cands[::max(1, len(gap_cands)//25)]
        rv_cands = rv_cands[::max(1, len(rv_cands)//25)]
        for ga in gap_cands:
            for rv in rv_cands:
                precision, recall, fired, tp = combo_eval(pairs_list, target1, ga, "below", rv, "above")
                if fired >= 5 and not math.isnan(precision):
                    f1 = 2*precision*recall/(precision+recall) if (precision+recall) else 0
                    if best_combo is None or f1 > best_combo[0]:
                        best_combo = (f1, ga, rv, precision, recall, fired, tp)
        if best_combo:
            f1, ga, rv, precision, recall, fired, tp = best_combo
            print(f"  gap<={ga} AND rv20_pct>={rv}: fired={fired} tp={tp} precision={precision*100:.1f}% recall={recall*100:.1f}%")

    # ret_since_open@10:00 AND pct_red@10:00 for target1
    pairs_list2 = []
    for d in universe:
        r = intraday_feat["1000"].get(d, {})
        if r.get("ret_since_open") is not None and r.get("pct_red") is not None:
            pairs_list2.append((r["ret_since_open"], r["pct_red"], target1[d]))
    if pairs_list2:
        best_combo2 = None
        ret_cands = sorted(set(round(v, 2) for v, _, _ in pairs_list2))
        red_cands = sorted(set(round(v, 2) for _, v, _ in pairs_list2))
        ret_cands = ret_cands[::max(1, len(ret_cands)//25)]
        for rc in ret_cands:
            for pc in red_cands:
                precision, recall, fired, tp = combo_eval(pairs_list2, target1, rc, "below", pc, "above")
                if fired >= 5 and not math.isnan(precision):
                    f1 = 2*precision*recall/(precision+recall) if (precision+recall) else 0
                    if best_combo2 is None or f1 > best_combo2[0]:
                        best_combo2 = (f1, rc, pc, precision, recall, fired, tp)
        if best_combo2:
            f1, rc, pc, precision, recall, fired, tp = best_combo2
            print(f"  ret_since_open@10:00<={rc} AND pct_red@10:00>={pc}: fired={fired} tp={tp} "
                  f"precision={precision*100:.1f}% recall={recall*100:.1f}%")

    # ============================================================
    # PART 2: ANATOMY
    # ============================================================
    print("\n\n########## PART 2: ANATOMY ##########")

    checkpoints = ["10:00", "10:30", "11:30", "13:00", "14:00", "15:00"]

    def anatomy_for(days):
        frac_by_cp = {cp: [] for cp in checkpoints}
        low_before_1130 = 0
        low_after_1400 = 0
        low_between = 0
        accelerated = 0
        v_reversed = 0
        vwap_retest_counts = []
        vwap_retest_outcomes_30 = []  # +1 bounced (px above vwap 30 min later), -1 continued down
        vwap_retest_outcomes_60 = []
        name_betas = defaultdict(list)
        name_lead_counts = defaultdict(int)
        detail = []
        for d in days:
            bars = intraday["SPY"].get(d)
            if not bars:
                continue
            day_open = bars[0][1]
            day_close = bars[-1][4]
            move = day_close - day_open
            if abs(move) < 1e-9:
                continue
            for cp in checkpoints:
                px = price_at_or_before(bars, cp)
                if px is not None:
                    frac_by_cp[cp].append((px - day_open) / move)
            # session low timing
            low_val = min(b[3] for b in bars)
            low_time = next(b[0] for b in bars if b[3] == low_val)
            if low_time < "11:30":
                low_before_1130 += 1
                low_bucket = "am"
            elif low_time >= "14:00":
                low_after_1400 += 1
                low_bucket = "pm"
            else:
                low_between += 1
                low_bucket = "mid"
            # accelerated vs V-reversed
            drop = day_open - low_val
            close_from_low_pct = (day_close - low_val) / low_val * 100
            recovered = day_close - low_val
            is_accel = close_from_low_pct <= 0.3
            is_v = (drop > 0) and (recovered > 0.5 * drop)
            if is_accel:
                accelerated += 1
            if is_v:
                v_reversed += 1
            # vwap retests: from below vwap by >0.1% crossing to within 0.1% of vwap
            cum_v = 0.0
            cum_pv = 0.0
            vwaps = []
            for (bt, o, h, l, c, v) in bars:
                tp3 = (h + l + c) / 3.0
                cum_pv += tp3 * v
                cum_v += v
                vwaps.append((bt, c, cum_pv / cum_v if cum_v else c))
            was_below = False
            retests = 0
            outcomes30 = []
            outcomes60 = []
            for idx2, (bt, c, vw) in enumerate(vwaps):
                distpct = (c - vw) / vw * 100
                if distpct < -0.1:
                    was_below = True
                elif was_below and distpct >= -0.1:
                    retests += 1
                    was_below = False
                    # look 30 / 60 minutes later
                    for mins, bucket in [(30, outcomes30), (60, outcomes60)]:
                        target_min = int(bt[:2]) * 60 + int(bt[3:]) + mins
                        tstr = f"{target_min // 60:02d}:{target_min % 60:02d}"
                        later = [x for x in vwaps if x[0] >= tstr]
                        if later:
                            lc, lvw = later[0][1], later[0][2]
                            bucket.append(1 if lc > lvw else -1)
            vwap_retest_counts.append(retests)
            vwap_retest_outcomes_30.extend(outcomes30)
            vwap_retest_outcomes_60.extend(outcomes60)
            # names vs SPY: end-of-day beta proxy + which name led (largest |return|)
            spy_ret = pct(day_close, day_open)
            biggest = None
            for nm in NAMES:
                nbars = intraday[nm].get(d)
                if not nbars:
                    continue
                nret = pct(nbars[-1][4], nbars[0][1])
                if spy_ret != 0:
                    name_betas[nm].append(nret / spy_ret)
                if biggest is None or abs(nret) > abs(biggest[1]):
                    biggest = (nm, nret)
            if biggest:
                name_lead_counts[biggest[0]] += 1
            detail.append(dict(date=d, oc_ret=spy_ret, low_time=low_time, low_bucket=low_bucket,
                                accelerated=is_accel, v_reversed=is_v, vwap_retests=retests,
                                lead_name=biggest[0] if biggest else None))
        return dict(frac_by_cp=frac_by_cp, low_before_1130=low_before_1130, low_after_1400=low_after_1400,
                    low_between=low_between, accelerated=accelerated, v_reversed=v_reversed,
                    vwap_retest_counts=vwap_retest_counts, vwap_retest_outcomes_30=vwap_retest_outcomes_30,
                    vwap_retest_outcomes_60=vwap_retest_outcomes_60, name_betas=name_betas,
                    name_lead_counts=name_lead_counts, detail=detail, n=len(days))

    for label, days in [("close<-1%", days_lt1), ("close<-2%", days_lt2)]:
        print(f"\n--- anatomy: {label} (n={len(days)}) ---")
        a = anatomy_for(days)
        print("  fraction of open->close move completed by checkpoint (median [q1,q3]):")
        for cp in checkpoints:
            vals = a["frac_by_cp"][cp]
            if vals:
                q1, med, q3 = np.percentile(vals, [25, 50, 75])
                print(f"    {cp}: median={med*100:6.1f}%  [{q1*100:6.1f}%, {q3*100:6.1f}%]  n={len(vals)}")
        tot = a["low_before_1130"] + a["low_between"] + a["low_after_1400"]
        print(f"  session low timing: before 11:30={a['low_before_1130']} ({a['low_before_1130']/tot*100:.0f}%)  "
              f"11:30-14:00={a['low_between']} ({a['low_between']/tot*100:.0f}%)  "
              f"after 14:00={a['low_after_1400']} ({a['low_after_1400']/tot*100:.0f}%)")
        print(f"  accelerated (close within 0.3% of low): {a['accelerated']}/{a['n']} ({a['accelerated']/a['n']*100:.0f}%)")
        print(f"  V-reversed (recovered >50% of the drop): {a['v_reversed']}/{a['n']} ({a['v_reversed']/a['n']*100:.0f}%)")
        rc = a["vwap_retest_counts"]
        print(f"  VWAP retests per day: mean={statistics.mean(rc):.2f}  median={statistics.median(rc):.1f}  "
              f"(total retests={sum(rc)})")
        o30 = a["vwap_retest_outcomes_30"]
        o60 = a["vwap_retest_outcomes_60"]
        if o30:
            print(f"  30min after retest: bounced above vwap {sum(1 for x in o30 if x==1)}/{len(o30)} "
                  f"({sum(1 for x in o30 if x==1)/len(o30)*100:.0f}%)")
        if o60:
            print(f"  60min after retest: bounced above vwap {sum(1 for x in o60 if x==1)}/{len(o60)} "
                  f"({sum(1 for x in o60 if x==1)/len(o60)*100:.0f}%)")
        print("  name betas (name return / SPY return) and lead counts:")
        for nm in NAMES:
            bs = a["name_betas"][nm]
            if bs:
                print(f"    {nm}: mean beta={statistics.mean(bs):.2f}  median={statistics.median(bs):.2f}  "
                      f"led on {a['name_lead_counts'][nm]} days")

        if label == "close<-2%":
            print("\n  --- 25 worst days, individually ---")
            rows_sorted = sorted(a["detail"], key=lambda r: r["oc_ret"])
            hdr = f"    {'date':10} {'oc%':>6} {'low_t':>6} {'lowbkt':>6} {'accel':>5} {'vrev':>5} {'vwapRT':>6} {'lead':>5} {'v18_trades':>10} {'v18_pnl':>8}"
            print(hdr)
            for r in rows_sorted:
                trades = v18.get(r["date"], [])
                npnl = sum(float(t["pnl"]) for t in trades)
                ntr = len(trades)
                print(f"    {r['date']:10} {r['oc_ret']:6.2f} {r['low_time']:>6} {r['low_bucket']:>6} "
                      f"{str(r['accelerated']):>5} {str(r['v_reversed']):>5} {r['vwap_retests']:>6} "
                      f"{str(r['lead_name']):>5} {ntr:>10} {npnl:8.2f}")

    # LOYO for the two intraday triggers we highlight for the <-2% target (small-n honesty check)
    print("\n=== LOYO for highlighted <-2% intraday triggers @ 10:30 ===")
    for fname, thr, dr in [("ret_since_open", -1.067, "below"), ("dist_vwap", -0.597, "below")]:
        loyo = loyo_check_intraday("1030", fname, target2, dr)
        print(f"  {fname} {dr} {thr}: folds={loyo}")

    # fixed literal -1.0% threshold at 10:30 (the owner's own phrasing), full transparency
    print("\n=== fixed threshold: SPY return-since-open <= -1.0% by 10:30 (literal, not fitted) ===")
    fixed_fired = [d for d in universe if intraday_feat["1030"].get(d, {}).get("ret_since_open") is not None
                   and intraday_feat["1030"][d]["ret_since_open"] <= -1.0]
    n_fired = len(fixed_fired)
    n_close_lt1 = sum(1 for d in fixed_fired if target1[d] == 1.0)
    n_close_lt2 = sum(1 for d in fixed_fired if target2[d] == 1.0)
    n_false_alarm_03 = sum(1 for d in fixed_fired if oc[d] is not None and oc[d] > -0.3)
    print(f"  fired on {n_fired}/{len(universe)} days ({n_fired/len(universe)*100:.2f}%, "
          f"~{n_fired/5:.1f}/yr over 5 years)")
    print(f"  of those: close<-1% = {n_close_lt1} ({n_close_lt1/n_fired*100:.1f}%)   "
          f"close<-2% = {n_close_lt2} ({n_close_lt2/n_fired*100:.1f}%)   "
          f"closed>-0.3% (false alarm) = {n_false_alarm_03} ({n_false_alarm_03/n_fired*100:.1f}%)")

    # false alarms: <-1% at 10:30 (return since open) but close > -0.3%
    false_alarm_days = []
    for d in universe:
        r = intraday_feat["1030"].get(d, {})
        ret1030 = r.get("ret_since_open")
        if ret1030 is not None and ret1030 < -1.0 and oc[d] is not None and oc[d] > -0.3:
            false_alarm_days.append(d)
    print(f"\n--- false alarms: <-1% at 10:30 but close>-0.3% (n={len(false_alarm_days)}) ---")
    print("  dates:", false_alarm_days)
    if false_alarm_days:
        a = anatomy_for(false_alarm_days)
        for cp in checkpoints:
            vals = a["frac_by_cp"][cp]
            if vals:
                q1, med, q3 = np.percentile(vals, [25, 50, 75])
                print(f"    {cp}: median={med*100:6.1f}%  [{q1*100:6.1f}%, {q3*100:6.1f}%]  n={len(vals)}")
        tot = a["low_before_1130"] + a["low_between"] + a["low_after_1400"]
        if tot:
            print(f"  session low timing: before 11:30={a['low_before_1130']}  11:30-14:00={a['low_between']}  after 14:00={a['low_after_1400']}")
        print(f"  accelerated: {a['accelerated']}/{a['n']}   V-reversed: {a['v_reversed']}/{a['n']}")
        # v18 activity on false alarm days
        tot_tr = sum(len(v18.get(d, [])) for d in false_alarm_days)
        tot_pnl = sum(sum(float(t["pnl"]) for t in v18.get(d, [])) for d in false_alarm_days)
        print(f"  iex_v18 on false-alarm days: {tot_tr} trades, ${tot_pnl:.2f} total pnl")

    # v18 totals on the two tail sets, for cross-check against the stress study numbers
    for label, days in [("<-1%", days_lt1), ("<-2%", days_lt2)]:
        tot_tr = sum(len(v18.get(d, [])) for d in days)
        tot_pnl = sum(sum(float(t["pnl"]) for t in v18.get(d, [])) for d in days)
        print(f"\niex_v18 on SPY {label} days: {tot_tr} trades, ${tot_pnl:.2f} total pnl")

    print("\nDone.")


if __name__ == "__main__":
    main()
