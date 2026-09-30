"""metrics over sweep trade csvs (data/<tag>_<year>_trades.csv), SPY-day buckets and bar coverage.

the maths copies research/entries/summarize.py: P&L, n, win %, profit factor, max drawdown on the
daily curve, per year and over the five years. SPY-day buckets use the same session return the
cross_context trigger sees: last RTH close / first RTH open - 1 per eastern date, from
data/bars_iex/SPY.csv. stdlib only.
"""
from __future__ import annotations

import csv
import datetime as dt
import glob
import math
import re
from collections import defaultdict
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
BARS_DIR = DATA / "bars_iex"
ET = ZoneInfo("America/New_York")
YEARS = ["2022", "2023", "2024", "2025", "2026"]
PF_CAP = 99.99  # json-safe stand-in for "no losing trades"


def sweep_end() -> dt.date:
    """the last date run_cached_sweep.sh replays for the current partial year."""
    try:
        text = (ROOT / "scripts" / "run_cached_sweep.sh").read_text()
        m = re.search(r'\[\[ "\$y" == "2026" \]\] && end="(\d{4}-\d{2}-\d{2})"', text)
        if m:
            return dt.date.fromisoformat(m.group(1))
    except OSError:
        pass
    return dt.date.today() - dt.timedelta(days=1)


# ---------------------------------------------------------------- trades

def load_trades(tag: str, data_dir: Path = DATA) -> list[dict]:
    rows = []
    for y in YEARS:
        for f in glob.glob(str(data_dir / f"{tag}_{y}_trades.csv")):
            with open(f, newline="") as fh:
                for r in csv.DictReader(fh):
                    if r.get("row_type") != "trade":
                        continue
                    r["year"] = y
                    r["pnl"] = float(r["pnl"])
                    rows.append(r)
    return rows


def sweep_files_present(tag: str, data_dir: Path = DATA) -> bool:
    return all((data_dir / f"{tag}_{y}_trades.csv").exists() for y in YEARS)


def stats(rows: list[dict]) -> dict:
    n = len(rows)
    if n == 0:
        return {"pnl": 0.0, "n": 0, "win_rate": 0.0, "pf": 0.0, "dd": 0.0, "gross_win": 0.0, "gross_loss": 0.0}
    pnl = sum(r["pnl"] for r in rows)
    wins = [r["pnl"] for r in rows if r["pnl"] > 0]
    losses = [-r["pnl"] for r in rows if r["pnl"] < 0]
    gw, gl = sum(wins), sum(losses)
    pf = (gw / gl) if gl > 0 else (PF_CAP if gw > 0 else 0.0)
    daily = defaultdict(float)
    for r in rows:
        daily[r["date"]] += r["pnl"]
    eq = peak = dd = 0.0
    for d in sorted(daily):
        eq += daily[d]
        peak = max(peak, eq)
        dd = min(dd, eq - peak)
    return {
        "pnl": round(pnl, 2), "n": n, "win_rate": round(len(wins) / n * 100, 1),
        "pf": round(min(pf, PF_CAP), 3), "dd": round(dd, 2),
        "gross_win": round(gw, 2), "gross_loss": round(gl, 2),
    }


def summarize(tag: str, data_dir: Path = DATA) -> dict:
    rows = load_trades(tag, data_dir)
    out = stats(rows)
    out["tag"] = tag
    out["years"] = {y: stats([r for r in rows if r["year"] == y]) for y in YEARS}
    out["years_positive"] = sum(1 for y in YEARS if out["years"][y]["pnl"] > 0)
    out["min_year_pnl"] = min(out["years"][y]["pnl"] for y in YEARS)
    out["pnl_2026"] = out["years"]["2026"]["pnl"]
    return out


# ---------------------------------------------------------------- bars / sessions

def iter_bars(csv_path: Path):
    with open(csv_path, newline="") as fh:
        rd = csv.reader(fh)
        header = next(rd, None)
        for row in rd:
            if not row or not row[0].isdigit():
                continue
            yield int(row[0]), float(row[1]), float(row[2]), float(row[3]), float(row[4])


def sessions(csv_path: Path) -> list[dt.date]:
    """distinct eastern dates with at least one bar, sorted."""
    seen = set()
    for ts, *_ in iter_bars(csv_path):
        seen.add(dt.datetime.fromtimestamp(ts, tz=ET).date())
    return sorted(seen)


def spy_day_returns(spy_csv: Path = BARS_DIR / "SPY.csv") -> dict[str, float]:
    """eastern date -> last close / first open - 1 (the cross_context session return at the close)."""
    first_open: dict[dt.date, float] = {}
    last_close: dict[dt.date, float] = {}
    for ts, o, h, l, c in iter_bars(spy_csv):
        d = dt.datetime.fromtimestamp(ts, tz=ET).date()
        if d not in first_open:
            first_open[d] = o
        last_close[d] = c
    return {d.isoformat(): (last_close[d] / first_open[d] - 1.0) for d in first_open if first_open[d] > 0}


def daily_pnl(rows: list[dict]) -> dict[str, float]:
    out = defaultdict(float)
    for r in rows:
        out[r["date"]] += r["pnl"]
    return dict(out)


def spy_buckets(rows: list[dict], spy_ret: dict[str, float], threshold: float = -0.01,
                start: dt.date | None = None, end: dt.date | None = None) -> dict:
    """split trades into SPY-stress days (session return < threshold) and ordinary days.
    stress_days counts every such SPY session in [start, end], traded or not."""
    start = start or dt.date(2022, 1, 1)
    end = end or sweep_end()
    stress_dates = {d for d, r in spy_ret.items() if r < threshold and start <= dt.date.fromisoformat(d) <= end}
    pnl_by_day = daily_pnl(rows)
    stress_rows = [r for r in rows if r["date"] in stress_dates]
    ordinary_rows = [r for r in rows if r["date"] not in stress_dates]
    stress_day_pnls = {d: pnl_by_day.get(d, 0.0) for d in stress_dates}
    traded = [d for d in stress_dates if d in pnl_by_day]
    s, o = stats(stress_rows), stats(ordinary_rows)
    return {
        "threshold": threshold,
        "stress_days": len(stress_dates),
        "stress_days_traded": len(traded),
        "stress_pnl": s["pnl"], "stress_trades": s["n"], "stress_pf": s["pf"],
        "stress_pnl_per_day": round(s["pnl"] / len(stress_dates), 2) if stress_dates else 0.0,
        "stress_worst_day": round(min(stress_day_pnls.values()), 2) if stress_day_pnls else 0.0,
        "stress_best_day": round(max(stress_day_pnls.values()), 2) if stress_day_pnls else 0.0,
        "ordinary_pnl": o["pnl"], "ordinary_trades": o["n"], "ordinary_pf": o["pf"],
    }


def coverage(ticker: str, bars_dir: Path = BARS_DIR, spy_csv: Path | None = None,
             end: dt.date | None = None) -> dict:
    """bar-cache coverage of <ticker> against SPY's sessions.
    rule (plan §9 B): >= 240 sessions in every full year, >= 95 % of SPY's sessions otherwise,
    no run of more than 5 consecutive missing SPY sessions, and no missing sessions in the
    last 5 of SPY's (the cache is current)."""
    spy_csv = spy_csv or bars_dir / "SPY.csv"
    path = bars_dir / f"{ticker}.csv"
    res = {"ticker": ticker, "path": str(path), "exists": path.exists(), "ok": False, "years": {}, "problems": []}
    if not path.exists():
        res["problems"].append("no bar file")
        return res
    spy = sessions(spy_csv)
    if end:
        spy = [d for d in spy if d <= end]
    have = set(sessions(path))
    if not have:
        res["problems"].append("bar file has no sessions")
        return res
    res["first"] = min(have).isoformat()
    res["last"] = max(have).isoformat()
    res["sessions"] = len(have)
    res["spy_sessions"] = len(spy)
    by_year_spy = defaultdict(list)
    for d in spy:
        by_year_spy[str(d.year)].append(d)
    for y, days in sorted(by_year_spy.items()):
        n_spy = len(days)
        n = sum(1 for d in days if d in have)
        full = n_spy >= 240
        need = 240 if full else math.ceil(0.95 * n_spy)
        ok = n >= need
        res["years"][y] = {"sessions": n, "spy_sessions": n_spy, "required": need, "ok": ok}
        if not ok:
            res["problems"].append(f"{y}: {n} sessions < {need} required")
    missing = [d for d in spy if d not in have]
    res["missing"] = len(missing)
    # longest run of consecutive missing SPY sessions
    idx = {d: i for i, d in enumerate(spy)}
    best = run = 0
    prev = None
    for d in missing:
        run = run + 1 if prev is not None and idx[d] == idx[prev] + 1 else 1
        best = max(best, run)
        prev = d
    res["max_gap"] = best
    if best > 5:
        res["problems"].append(f"hole: {best} consecutive missing sessions")
    tail = spy[-5:]
    if any(d not in have for d in tail):
        res["problems"].append(f"stale: missing sessions among SPY's last 5 ({tail[0]}..{tail[-1]})")
    res["ok"] = not res["problems"]
    return res


def trade_key(r: dict) -> tuple:
    """identity of a trade for base/added matching: (date, ticker, entry_time, direction)."""
    return (r.get("date"), r.get("ticker"), r.get("entry_time"), r.get("direction"))


def split_marginal(rows: list[dict], baseline_rows: list[dict]) -> tuple[list[dict], list[dict]]:
    """split a candidate's trade rows into (base, added) against a baseline's trade set, matched
    on trade_key(). 'base' is every row whose key also appears in the baseline (unchanged for a
    purely additive candidate); 'added' is everything else — the new window(s)."""
    base_keys = {trade_key(r) for r in baseline_rows}
    base = [r for r in rows if trade_key(r) in base_keys]
    added = [r for r in rows if trade_key(r) not in base_keys]
    return base, added


def worst_trade(rows: list[dict]) -> float:
    return round(min((r["pnl"] for r in rows), default=0.0), 2)


def worst_day(rows: list[dict]) -> float:
    d = daily_pnl(rows)
    return round(min(d.values()), 2) if d else 0.0


def marginal_metrics(tag: str, baseline_tag: str, data_dir: Path = DATA) -> dict:
    """split <tag>'s trades against <baseline_tag>'s trade set (see split_marginal()) and compute
    5y + per-year stats for both halves, plus the added half's worst single trade and worst day —
    what the additive-config gate checks over."""
    rows = load_trades(tag, data_dir)
    baseline_rows = load_trades(baseline_tag, data_dir)
    base_rows, added_rows = split_marginal(rows, baseline_rows)
    base = stats(base_rows)
    base["years"] = {y: stats([r for r in base_rows if r["year"] == y]) for y in YEARS}
    added = stats(added_rows)
    added["years"] = {y: stats([r for r in added_rows if r["year"] == y]) for y in YEARS}
    added["worst_year_pnl"] = min(added["years"][y]["pnl"] for y in YEARS)
    added["worst_trade"] = worst_trade(added_rows)
    added["worst_day"] = worst_day(added_rows)
    return {"baseline_tag": baseline_tag, "base": base, "added": added}


def full_metrics(tag: str, tickers: list[str] | None = None, data_dir: Path = DATA,
                 spy_ret: dict[str, float] | None = None, baseline_tag: str | None = None) -> dict:
    """everything a gate may look at for one sweep tag. baseline_tag, when given, adds the
    base/added marginal split against that tag's trades (metrics['marginal'])."""
    m = summarize(tag, data_dir)
    rows = load_trades(tag, data_dir)
    spy_ret = spy_ret if spy_ret is not None else (spy_day_returns() if (BARS_DIR / "SPY.csv").exists() else {})
    m["stress"] = spy_buckets(rows, spy_ret, -0.01) if spy_ret else None
    m["stress_2pct"] = spy_buckets(rows, spy_ret, -0.02) if spy_ret else None
    if tickers:
        m["tickers"] = list(tickers)
    if baseline_tag:
        m["marginal"] = marginal_metrics(tag, baseline_tag, data_dir)
    return m


def fmt_stats(s: dict) -> str:
    if not s or s.get("n", 0) == 0:
        return "—"
    return f"{s['pnl']:+.0f} / {s['n']} / PF {s['pf']:.2f}"
