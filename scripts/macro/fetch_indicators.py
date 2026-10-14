#!/usr/bin/env python3
"""recession watch — daily macro indicator fetch + rubric scoring (no LLM).

pulls free public series (FRED CSV endpoints, no API key) plus our own daily bar cache
(research/swing/daily/<SYM>.csv) and computes a rubric-based BASE score (0-8, see
docs/macro/README.md for the full rubric and thresholds). writes:

  data/macro/<date>.json   — today's components, values, thresholds hit, base score, momentum
  data/macro/history.csv   — append/update one row per date (upsert by date; idempotent)
  data/macro/series/<SERIES>.csv — raw per-series cache, refetched only if > 20h old

this script is INFORMATION ONLY. it never reads from or writes to any strategy config, any
database table, or anything under crates/ or migrations/. the BASE score it produces is not a
forecast — see docs/macro/README.md's caveats. the qualitative 0-2 adjustment and the FINAL
score are added later by the `recession-watch` claude skill, not by this script.

usage:
    python3 scripts/macro/fetch_indicators.py              # fetch, compute, write
    python3 scripts/macro/fetch_indicators.py --dry-run    # fetch + compute, write nothing
    python3 scripts/macro/fetch_indicators.py --date 2026-10-10   # override "today" (testing)

python3 stdlib + numpy only. never crashes on a missing/failed series — marks it "n/a" and
keeps going; a missing component scores 0 (conservative) and is flagged in the output.
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import io
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
MACRO_DIR = ROOT / "data" / "macro"
SERIES_DIR = MACRO_DIR / "series"
SWING_DAILY = ROOT / "research" / "swing" / "daily"
HISTORY_CSV = MACRO_DIR / "history.csv"

FRED_SERIES = ["T10Y3M", "T10Y2Y", "BAMLH0A0HYM2", "SAHMREALTIME", "ICSA", "UNRATE",
               "VIXCLS", "DGS10", "DFF", "VXVCLS"]
FRED_URL = "https://fred.stlouisfed.org/graph/fredgraph.csv?id={}"
CACHE_MAX_AGE_S = 20 * 3600
FETCH_TIMEOUT_S = 15

HISTORY_FIELDS = ["date", "base", "yield_curve", "credit", "labor", "equity_stress",
                  "breadth", "qualitative", "final"]


# ---------- fetch / cache ----------

def _parse_fred_csv(text: str) -> list[tuple[str, float | None]]:
    rows = []
    r = csv.reader(io.StringIO(text))
    next(r, None)  # header
    for row in r:
        if len(row) < 2:
            continue
        d, v = row[0].strip(), row[1].strip()
        if not d:
            continue
        try:
            rows.append((d, float(v)))
        except ValueError:
            rows.append((d, None))
    return rows


def fetch_series(series_id: str, dry_run: bool) -> tuple[list[tuple[str, float | None]] | None, str]:
    """returns (points, status). status in fetched|cached|stale_cache|n/a."""
    cache_path = SERIES_DIR / f"{series_id}.csv"
    cached_text = None
    if cache_path.exists():
        age = time.time() - cache_path.stat().st_mtime
        if age < CACHE_MAX_AGE_S:
            try:
                cached_text = cache_path.read_text()
                return _parse_fred_csv(cached_text), "cached"
            except OSError:
                pass
        else:
            try:
                cached_text = cache_path.read_text()
            except OSError:
                cached_text = None

    try:
        req = urllib.request.Request(FRED_URL.format(series_id), headers={"User-Agent": "Mozilla/5.0"})
        with urllib.request.urlopen(req, timeout=FETCH_TIMEOUT_S) as resp:
            text = resp.read().decode("utf-8", errors="replace")
        if not dry_run:
            SERIES_DIR.mkdir(parents=True, exist_ok=True)
            cache_path.write_text(text)
        return _parse_fred_csv(text), "fetched"
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, OSError) as e:
        print(f"  warn: fetch {series_id} failed ({e}); ", end="", file=sys.stderr)
        if cached_text is not None:
            print("using stale cache", file=sys.stderr)
            return _parse_fred_csv(cached_text), "stale_cache"
        print("no cache available, marking n/a", file=sys.stderr)
        return None, "n/a"


def latest(points: list[tuple[str, float | None]] | None) -> tuple[str | None, float | None]:
    if not points:
        return None, None
    for d, v in reversed(points):
        if v is not None:
            return d, v
    return None, None


def value_near(points: list[tuple[str, float | None]], target_date: dt.date) -> float | None:
    """value of the point whose date is <= target_date, closest to it (walking back)."""
    best = None
    for d, v in points:
        if v is None:
            continue
        pd = dt.date.fromisoformat(d)
        if pd <= target_date:
            best = v
        else:
            break
    return best


# ---------- daily bar cache (research/swing/daily) ----------

def load_daily_closes(symbol: str) -> list[tuple[str, float]] | None:
    path = SWING_DAILY / f"{symbol}.csv"
    if not path.exists():
        return None
    try:
        out = []
        with open(path, newline="") as f:
            r = csv.DictReader(f)
            for row in r:
                try:
                    out.append((row["date"], float(row["close"])))
                except (KeyError, ValueError):
                    continue
        out.sort(key=lambda t: t[0])
        return out or None
    except OSError:
        return None


# ---------- rubric components ----------

def score_yield_curve(points: list[tuple[str, float | None]] | None) -> dict:
    if not points:
        return {"score": 0, "status": "n/a", "detail": "T10Y3M unavailable"}
    pts = [(d, v) for d, v in points if v is not None]
    if not pts:
        return {"score": 0, "status": "n/a", "detail": "T10Y3M unavailable"}
    last_date_s, last_val = pts[-1]
    last_date = dt.date.fromisoformat(last_date_s)

    # current streak of consecutive negative (inverted) readings ending today
    i = len(pts) - 1
    cur_streak_days = 0
    if last_val < 0:
        j = i
        while j > 0 and pts[j - 1][1] < 0:
            j -= 1
        cur_streak_days = (last_date - dt.date.fromisoformat(pts[j][0])).days

    detail = f"T10Y3M={last_val:+.2f} on {last_date_s}"
    if last_val < 0:
        if cur_streak_days >= 90:
            return {"score": 2, "status": "ok", "value": last_val,
                     "detail": f"{detail}; inverted {cur_streak_days}d (>=3mo) -> 2"}
        return {"score": 1, "status": "ok", "value": last_val,
                 "detail": f"{detail}; inverted {cur_streak_days}d (<3mo) -> 1"}

    # not currently inverted: did a >=3mo inversion streak END within the last 6 months?
    streaks = []
    k = 0
    n = len(pts)
    while k < n:
        if pts[k][1] < 0:
            m = k
            while m + 1 < n and pts[m + 1][1] < 0:
                m += 1
            s_start = dt.date.fromisoformat(pts[k][0])
            s_end = dt.date.fromisoformat(pts[m][0])
            streaks.append(((s_end - s_start).days, s_end))
            k = m + 1
        else:
            k += 1
    recent_big = [s for s in streaks if s[0] >= 90 and (last_date - s[1]).days <= 183]
    if recent_big:
        length, end = max(recent_big, key=lambda s: s[1])
        return {"score": 2, "status": "ok", "value": last_val,
                 "detail": f"{detail}; un-inverted after a {length}d inversion ending {end} (<=6mo ago) -> 2"}
    if abs(last_val) < 0.25:
        return {"score": 1, "status": "ok", "value": last_val, "detail": f"{detail}; flat (<25bps) -> 1"}
    return {"score": 0, "status": "ok", "value": last_val, "detail": f"{detail}; not inverted, not flat -> 0"}


def score_credit(points: list[tuple[str, float | None]] | None) -> dict:
    if not points:
        return {"score": 0, "status": "n/a", "detail": "BAMLH0A0HYM2 unavailable"}
    pts = [(d, v) for d, v in points if v is not None]
    if not pts:
        return {"score": 0, "status": "n/a", "detail": "BAMLH0A0HYM2 unavailable"}
    last_date_s, last_pct = pts[-1]
    last_bps = last_pct * 100.0
    last_date = dt.date.fromisoformat(last_date_s)
    ago_pct = value_near(pts, last_date - dt.timedelta(days=90))
    delta_bps = (last_bps - ago_pct * 100.0) if ago_pct is not None else None
    detail = f"HY OAS={last_bps:.0f}bps on {last_date_s}"
    if delta_bps is not None:
        detail += f", 3mo change {delta_bps:+.0f}bps"
    if last_bps >= 500:
        score = 2
        detail += " -> 2 (>=500bps)"
    elif 400 <= last_bps < 500 or (delta_bps is not None and delta_bps >= 100):
        score = 1
        detail += " -> 1 (400-500bps or +100bps/3mo)"
    else:
        score = 0
        detail += " -> 0"
    return {"score": score, "status": "ok", "value": last_bps, "delta_3m_bps": delta_bps, "detail": detail}


def score_labor(sahm_pts, icsa_pts) -> dict:
    sahm_date, sahm_val = latest(sahm_pts)
    sahm_score = 0
    sahm_detail = "SAHMREALTIME n/a"
    if sahm_val is not None:
        if sahm_val >= 0.5:
            sahm_score = 2
        elif sahm_val >= 0.3:
            sahm_score = 1
        sahm_detail = f"Sahm={sahm_val:.2f} on {sahm_date} -> {sahm_score}"

    claims_add = 0
    claims_detail = "ICSA n/a"
    pct_change = None
    if icsa_pts:
        pts = [(d, v) for d, v in icsa_pts if v is not None]
        if len(pts) >= 4:
            last_date = dt.date.fromisoformat(pts[-1][0])
            avg_now = float(np.mean([v for _, v in pts[-4:]]))
            year_ago_cut = last_date - dt.timedelta(days=364)
            prior = [v for d, v in pts if dt.date.fromisoformat(d) <= year_ago_cut]
            if len(prior) >= 4:
                avg_year_ago = float(np.mean(prior[-4:]))
                if avg_year_ago:
                    pct_change = (avg_now - avg_year_ago) / avg_year_ago * 100.0
                    claims_add = 1 if pct_change >= 20 else 0
                    claims_detail = f"claims 4wk avg {avg_now:.0f} vs 1y ago {avg_year_ago:.0f} ({pct_change:+.1f}%) -> +{claims_add}"
    score = min(2, sahm_score + claims_add)
    return {"score": score, "status": "ok" if sahm_val is not None or pct_change is not None else "n/a",
            "sahm_value": sahm_val, "claims_pct_change_yoy": pct_change,
            "detail": f"{sahm_detail}; {claims_detail}; capped total -> {score}"}


def score_equity_stress(spy_closes, vix_pts, vix3m_pts) -> dict:
    if not spy_closes or len(spy_closes) < 20:
        return {"score": 0, "status": "n/a", "detail": "SPY daily cache unavailable/too short"}
    dates = [d for d, _ in spy_closes]
    closes = np.array([c for _, c in spy_closes], dtype=float)
    last_close = closes[-1]
    last_date_s = dates[-1]
    window = closes[-252:] if len(closes) >= 252 else closes
    roll_max = float(window.max())
    dd_pct = (roll_max - last_close) / roll_max * 100.0 if roll_max else 0.0

    sessions_below_200 = 0
    sma200_val = None
    if len(closes) >= 200:
        sma_series = np.array([closes[i - 199:i + 1].mean() for i in range(199, len(closes))])
        aligned_closes = closes[199:]
        i = len(sma_series) - 1
        while i >= 0 and aligned_closes[i] < sma_series[i]:
            sessions_below_200 += 1
            i -= 1
        sma200_val = float(sma_series[-1])

    base = 0
    if dd_pct >= 15:
        base = 2
    elif dd_pct >= 8 or sessions_below_200 > 20:
        base = 1

    vix_date, vix_val = latest(vix_pts)
    vix_add = 1 if (vix_val is not None and vix_val >= 30) else 0
    vix3m_date, vix3m_val = latest(vix3m_pts)
    term_add = 1 if (vix_val is not None and vix3m_val is not None and vix3m_val < vix_val) else 0
    score = min(2, base + vix_add + term_add)
    detail = (f"SPY dd={dd_pct:.1f}% (close {last_close:.2f} vs {len(window)}d high {roll_max:.2f}) on {last_date_s}; "
              f"{sessions_below_200} consecutive sessions below 200dma; base={base}; "
              f"VIX={vix_val if vix_val is not None else 'n/a'} (+{vix_add}); "
              f"VIX3M={vix3m_val if vix3m_val is not None else 'n/a'} vs VIX inverted (+{term_add}); capped -> {score}")
    return {"score": score, "status": "ok", "drawdown_pct": dd_pct, "sessions_below_200dma": sessions_below_200,
            "sma200": sma200_val, "vix": vix_val, "vix3m": vix3m_val, "detail": detail}


def score_breadth(symbols: list[str]) -> dict:
    total = 0
    above_50 = 0
    above_200 = 0
    skipped = []
    for sym in symbols:
        rows = load_daily_closes(sym)
        if not rows or len(rows) < 200:
            skipped.append(sym)
            continue
        closes = np.array([c for _, c in rows], dtype=float)
        last_close = closes[-1]
        sma50 = closes[-50:].mean()
        sma200 = closes[-200:].mean()
        total += 1
        if last_close > sma50:
            above_50 += 1
        if last_close > sma200:
            above_200 += 1
    if total == 0:
        return {"score": 0, "status": "n/a", "detail": "no usable daily caches", "skipped": skipped}
    pct_above_50 = above_50 / total * 100.0
    pct_above_200 = above_200 / total * 100.0
    if pct_above_200 < 20:
        score = 2
    elif pct_above_200 < 30:
        score = 1
    else:
        score = 0
    detail = (f"{above_200}/{total} ({pct_above_200:.0f}%) above 200dma, "
              f"{above_50}/{total} ({pct_above_50:.0f}%) above 50dma -> {score}")
    if skipped:
        detail += f"; skipped (insufficient history): {','.join(skipped)}"
    return {"score": score, "status": "ok", "pct_above_50dma": pct_above_50, "pct_above_200dma": pct_above_200,
            "symbols_used": total, "symbols_skipped": skipped, "detail": detail}


# ---------- history.csv (upsert by date) ----------

def read_history() -> list[dict]:
    if not HISTORY_CSV.exists():
        return []
    with open(HISTORY_CSV, newline="") as f:
        return list(csv.DictReader(f))


def write_history(rows: list[dict]) -> None:
    MACRO_DIR.mkdir(parents=True, exist_ok=True)
    rows.sort(key=lambda r: r["date"])
    with open(HISTORY_CSV, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=HISTORY_FIELDS)
        w.writeheader()
        for r in rows:
            w.writerow({k: r.get(k, "") for k in HISTORY_FIELDS})


def compute_momentum(history_rows: list[dict], today: str, today_base: int) -> dict:
    prior = sorted([r for r in history_rows if r["date"] != today], key=lambda r: r["date"])

    def base_n_ago(n):
        if len(prior) >= n:
            try:
                return int(prior[-n]["base"])
            except (ValueError, TypeError):
                return None
        return None

    b5, b20 = base_n_ago(5), base_n_ago(20)
    return {
        "base_5_sessions_ago": b5, "base_20_sessions_ago": b20,
        "delta_5": (today_base - b5) if b5 is not None else None,
        "delta_20": (today_base - b20) if b20 is not None else None,
    }


# ---------- main ----------

UNIVERSE_59 = sorted(p.stem for p in SWING_DAILY.glob("*.csv") if p.stem != "SPY") if SWING_DAILY.exists() else []


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--date", default=None, help="override 'today' (YYYY-MM-DD), for testing")
    args = ap.parse_args()

    today = args.date or dt.date.today().isoformat()

    fetched, failed = [], []
    series_points = {}
    for sid in FRED_SERIES:
        pts, status = fetch_series(sid, args.dry_run)
        series_points[sid] = pts
        if status in ("fetched", "cached"):
            fetched.append(f"{sid}:{status}")
        elif status == "stale_cache":
            fetched.append(f"{sid}:stale_cache")
        else:
            failed.append(sid)

    yc = score_yield_curve(series_points["T10Y3M"])
    cr = score_credit(series_points["BAMLH0A0HYM2"])
    lb = score_labor(series_points["SAHMREALTIME"], series_points["ICSA"])
    spy_closes = load_daily_closes("SPY")
    eq = score_equity_stress(spy_closes, series_points["VIXCLS"], series_points["VXVCLS"])
    br = score_breadth(UNIVERSE_59)

    base = min(8, yc["score"] + cr["score"] + lb["score"] + eq["score"] + br["score"])

    history_rows = read_history()
    momentum = compute_momentum(history_rows, today, base)

    spy_as_of = spy_closes[-1][0] if spy_closes else None

    result = {
        "date": today,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "base": base,
        "qualitative": None,
        "final": None,
        "components": {
            "yield_curve": yc,
            "credit": cr,
            "labor": lb,
            "equity_stress": eq,
            "breadth": br,
        },
        "momentum": momentum,
        "series_fetched": fetched,
        "series_failed": failed,
        "swing_cache_as_of": spy_as_of,
        "note": "BASE only (0-8). qualitative (0-2) and FINAL are added later by the recession-watch skill.",
    }

    print(json.dumps(result, indent=2, default=str))
    print(f"\nBASE SCORE: {base}  (yield_curve={yc['score']} credit={cr['score']} "
          f"labor={lb['score']} equity_stress={eq['score']} breadth={br['score']})", file=sys.stderr)
    if failed:
        print(f"series failed/unavailable: {failed}", file=sys.stderr)
    if spy_as_of and spy_as_of < today:
        print(f"note: research/swing/daily cache is as of {spy_as_of} (stale vs today {today}); "
              f"equity_stress/breadth reflect that lag", file=sys.stderr)

    if args.dry_run:
        print("\n--dry-run: nothing written", file=sys.stderr)
        return 0

    MACRO_DIR.mkdir(parents=True, exist_ok=True)
    out_path = MACRO_DIR / f"{today}.json"
    out_path.write_text(json.dumps(result, indent=2, default=str))

    row = {
        "date": today, "base": base,
        "yield_curve": yc["score"], "credit": cr["score"], "labor": lb["score"],
        "equity_stress": eq["score"], "breadth": br["score"],
        "qualitative": "", "final": "",
    }
    rows = [r for r in history_rows if r["date"] != today]
    rows.append(row)
    write_history(rows)
    print(f"\nwrote {out_path} and updated {HISTORY_CSV}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
