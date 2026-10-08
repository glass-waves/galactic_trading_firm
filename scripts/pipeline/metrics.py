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


def max_position_fraction(rows: list[dict], capital: float = 10000.0) -> float:
    """largest observed position size x entry_price / capital (the sizing-config gate's cap check).
    rows missing or unparseable size/entry_price are skipped rather than failing the whole metric."""
    if not capital:
        return 0.0
    fracs = []
    for r in rows:
        try:
            size = float(r.get("size"))
            price = float(r.get("entry_price"))
        except (TypeError, ValueError):
            continue
        fracs.append(size * price / capital)
    return round(max(fracs), 4) if fracs else 0.0


def summarize(tag: str, data_dir: Path = DATA) -> dict:
    rows = load_trades(tag, data_dir)
    out = stats(rows)
    out["tag"] = tag
    out["max_position_fraction"] = max_position_fraction(rows)
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


def concurrency_exceedance(rows: list[dict], cap: int = 3) -> dict:
    """for a combined set of trade rows (each with entry_time/exit_time, ISO8601 UTC strings that
    sort lexically in time order), the eastern dates on which the number of concurrently open
    positions would exceed `cap` at any instant — the live engine's position cap. a close at the
    same timestamp as an open is processed first (frees the slot before it is reused), so a
    back-to-back flip is not double-counted. info only: this is never a pass/fail gate check (see
    the additive-ticker gate — the replay runs each ticker set independently, so a straight
    concatenation of two tags' trades is exact except for this cap)."""
    by_day: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for r in rows:
        et, xt = r.get("entry_time"), r.get("exit_time")
        if not et or not xt:
            continue
        by_day[r["date"]].append((et, xt))
    bad_days = []
    max_concurrent = 0
    for d, intervals in by_day.items():
        events = []
        for et, xt in intervals:
            events.append((et, 1))  # open
            events.append((xt, 0))  # close — sorts before an open at the same timestamp
        events.sort()
        cur = 0
        day_max = 0
        for _, tag in events:
            if tag == 0:
                cur -= 1
            else:
                cur += 1
                day_max = max(day_max, cur)
        max_concurrent = max(max_concurrent, day_max)
        if day_max > cap:
            bad_days.append(d)
    return {"cap": cap, "exceedance_days": sorted(bad_days), "n_exceedance_days": len(bad_days),
            "max_concurrent_observed": max_concurrent}


def spy_session_list(spy_csv: Path = BARS_DIR / "SPY.csv", start: dt.date | None = None,
                      end: dt.date | None = None) -> list[str]:
    """ISO date strings for every SPY session in [start, end] (default 2022-01-01..sweep_end(),
    the same window every other 5y metric uses) — the standalone-strategy gate's daily series are
    built over this union so a day with no trades on either side still counts as a zero, not a gap."""
    start = start or dt.date(2022, 1, 1)
    end = end or sweep_end()
    return [d.isoformat() for d in sessions(spy_csv) if start <= d <= end]


def daily_series(rows: list[dict], session_list: list[str]) -> dict[str, float]:
    """daily P&L for every date in `session_list`, 0.0 where `rows` has no trade that day."""
    pnl = daily_pnl(rows)
    return {d: round(pnl.get(d, 0.0), 2) for d in session_list}


def pearson_corr(a: dict[str, float], b: dict[str, float]) -> float:
    """Pearson correlation of two daily series over their shared dates. 0.0 when undefined (fewer
    than 2 shared dates, or either series has zero variance over them)."""
    dates = sorted(set(a) & set(b))
    if len(dates) < 2:
        return 0.0
    xs = [a[d] for d in dates]
    ys = [b[d] for d in dates]
    n = len(xs)
    mx, my = sum(xs) / n, sum(ys) / n
    cov = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    vx = sum((x - mx) ** 2 for x in xs)
    vy = sum((y - my) ** 2 for y in ys)
    if vx <= 0 or vy <= 0:
        return 0.0
    return round(cov / math.sqrt(vx * vy), 4)


def combine_series(a: dict[str, float], b: dict[str, float]) -> dict[str, float]:
    """sum of two daily series over the union of their dates (missing on one side = 0 there)."""
    return {d: round(a.get(d, 0.0) + b.get(d, 0.0), 2) for d in (set(a) | set(b))}


def drawdown_on_series(series: dict[str, float]) -> float:
    """max drawdown of the equity curve built by walking `series` in date order — the same
    accumulate-then-peak maths stats() uses on daily_pnl(rows), but over an already-built series
    (e.g. a combined book's) rather than recomputed from trade rows."""
    eq = peak = dd = 0.0
    for d in sorted(series):
        eq += series[d]
        peak = max(peak, eq)
        dd = min(dd, eq - peak)
    return round(dd, 2)


def ex_best_year_pf(rows: list[dict]) -> dict:
    """stats() with the candidate's single best (highest-P&L) year excluded — guards against a
    one-year wonder (a candidate whose whole edge is one favorable year). a year with no trades
    has P&L 0 and is only "best" if every year is <= 0."""
    year_pnl = {y: sum(r["pnl"] for r in rows if r["year"] == y) for y in YEARS}
    best_year = max(year_pnl, key=lambda y: year_pnl[y])
    out = stats([r for r in rows if r["year"] != best_year])
    out["excluded_year"] = best_year
    out["excluded_year_pnl"] = round(year_pnl[best_year], 2)
    return out


def standalone_metrics(tag: str, baseline_tag: str = "iex_v18", data_dir: Path = DATA) -> dict:
    """everything the standalone-strategy gate needs beyond the candidate's own summarize(): a
    daily P&L series for the candidate and the baseline over the union of SPY sessions (zero-filled
    for days without trades — spy_session_list()/daily_series()), their Pearson correlation
    (pearson_corr()), the combined book (baseline_rows + candidate rows — a plain concatenation is
    exact because the live replay runs every book independently, same as additive_ticker's
    'combined') with its PF and per-year stats from stats(), but its drawdown computed on the
    combined *daily* series (drawdown_on_series()) rather than re-derived per-trade, active-day
    counts for the candidate/baseline/combined (days with >= 1 trade), the candidate's PF with its
    best year excluded (ex_best_year_pf() — guards a one-year wonder), and concurrency_exceedance()
    over the combined set (info only, same rationale as additive_ticker's)."""
    rows = load_trades(tag, data_dir)
    baseline_rows = load_trades(baseline_tag, data_dir)
    session_list = spy_session_list()
    cand_series = daily_series(rows, session_list)
    base_series = daily_series(baseline_rows, session_list)
    corr = pearson_corr(cand_series, base_series)
    combined_rows = baseline_rows + rows
    combined_series = combine_series(cand_series, base_series)
    combined = stats(combined_rows)
    combined["dd"] = drawdown_on_series(combined_series)
    combined["years"] = {y: stats([r for r in combined_rows if r["year"] == y]) for y in YEARS}
    baseline_active_days = len(daily_pnl(baseline_rows))
    candidate_active_days = len(daily_pnl(rows))
    combined_active_days = len(set(daily_pnl(baseline_rows)) | set(daily_pnl(rows)))
    concurrency = concurrency_exceedance(combined_rows)
    return {
        "baseline_tag": baseline_tag,
        "daily_corr": corr,
        "combined": combined,
        "baseline_active_days": baseline_active_days,
        "candidate_active_days": candidate_active_days,
        "combined_active_days": combined_active_days,
        "ex_best_year": ex_best_year_pf(rows),
        "concurrency": concurrency,
    }


def marginal_metrics(tag: str, baseline_tag: str, data_dir: Path = DATA) -> dict:
    """split <tag>'s trades against <baseline_tag>'s trade set (see split_marginal()) and compute
    5y + per-year stats for both halves, plus the added half's worst single trade and worst day —
    what the additive-config gate checks over. also computes 'combined' (baseline_rows + added_rows,
    i.e. the book as if <tag> traded alongside the baseline — exact for a candidate whose trades
    never share a trade_key() with the baseline's, e.g. a different ticker) and 'concurrency'
    (concurrency_exceedance() over that combined set) — what the additive-ticker gate checks over."""
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
    combined_rows = baseline_rows + added_rows
    combined = stats(combined_rows)
    combined["years"] = {y: stats([r for r in combined_rows if r["year"] == y]) for y in YEARS}
    concurrency = concurrency_exceedance(combined_rows)
    return {"baseline_tag": baseline_tag, "base": base, "added": added, "combined": combined, "concurrency": concurrency}


def full_metrics(tag: str, tickers: list[str] | None = None, data_dir: Path = DATA,
                 spy_ret: dict[str, float] | None = None, baseline_tag: str | None = None,
                 want_standalone: bool = False) -> dict:
    """everything a gate may look at for one sweep tag. baseline_tag, when given, adds the
    base/added marginal split against that tag's trades (metrics['marginal']) — or, when
    want_standalone is also set (the standalone-strategy gate), metrics['standalone']
    (standalone_metrics()) instead: a different, more expensive computation (daily series,
    correlation, combined drawdown) that only that gate needs."""
    m = summarize(tag, data_dir)
    rows = load_trades(tag, data_dir)
    spy_ret = spy_ret if spy_ret is not None else (spy_day_returns() if (BARS_DIR / "SPY.csv").exists() else {})
    m["stress"] = spy_buckets(rows, spy_ret, -0.01) if spy_ret else None
    m["stress_2pct"] = spy_buckets(rows, spy_ret, -0.02) if spy_ret else None
    if tickers:
        m["tickers"] = list(tickers)
    if baseline_tag:
        if want_standalone:
            m["standalone"] = standalone_metrics(tag, baseline_tag, data_dir)
        else:
            m["marginal"] = marginal_metrics(tag, baseline_tag, data_dir)
    return m


def fmt_stats(s: dict) -> str:
    if not s or s.get("n", 0) == 0:
        return "—"
    return f"{s['pnl']:+.0f} / {s['n']} / PF {s['pf']:.2f}"
