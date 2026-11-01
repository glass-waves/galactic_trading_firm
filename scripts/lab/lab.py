#!/usr/bin/env python3
"""lab — parallel, cached, pre-registered backtest harness (research/lab/PLAN.md).

    lab.py run SPEC.json --window train [--agent desk] [--json] [--quiet]
    lab.py score TAG [TAG...] --window train          # existing data/<tag>_<year>_trades.csv sets
    lab.py compare SPEC_A SPEC_B --window train       # paired day bootstrap A − B (specs or tags)
    lab.py top [--window train] [--agent desk] [-n 30] # ledger leaderboard
    lab.py sessions --window train                    # list the session days of a window
    lab.py cells [--agent desk]                       # how many distinct specs the ledger holds

spec JSON (everything optional except name):
    {"name": "desk/cell", "config_id": 12, "patches": ["research/lab/patches/desk/x.json"],
     "args": ["--entry-threshold", "0.6"], "tickers": "AMZN,AAPL", "lookback": 8, "capital": 10000,
     "bars_dir": "data/bars_iex", "cost_args": [] , "note": "free text"}

windows: train | holdout | target | all | 2025h1 | 2025 | 2024 | ... | YYYY-MM-DD:YYYY-MM-DD.
holdout / target / all need LAB_REVEAL=1 (coordinator only — see PLAN.md).
one backtest process per session day, fanned over the machine with a slot semaphore in /tmp/lab_slots so
several agents can run at once; results are cached per (spec, day) in data/lab_cache/.
"""
from __future__ import annotations

import argparse
import csv
import datetime as dt
import fcntl
import hashlib
import io
import json
import os
import subprocess
import sys
import threading
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "pipeline"))
import metrics as pm  # noqa: E402  (scripts/pipeline/metrics.py: bootstrap, sign-flip, paired bootstrap)

LAB = ROOT / "research" / "lab"
LEDGER = LAB / "ledger.jsonl"
RUNS = LAB / "runs"
CACHE = ROOT / "data" / "lab_cache"
SLOTS = Path("/tmp/lab_slots")
BIN = ROOT / "target" / "release" / "backtest"
EASTERN = ZoneInfo("America/New_York")
N_SLOTS = int(os.environ.get("LAB_SLOTS", os.cpu_count() or 4))

WINDOWS = {
    "train": ("2022-01-03", "2025-06-30"),
    "holdout": ("2025-07-01", "2026-10-30"),
    "target": ("2025-01-01", "2026-10-30"),
    "all": ("2022-01-03", "2026-10-30"),
    "2025h1": ("2025-01-01", "2025-06-30"),
    "2024h2": ("2024-07-01", "2024-12-31"),
    "2024h2_25h1": ("2024-07-01", "2025-06-30"),
    "2023_25h1": ("2023-01-01", "2025-06-30"),
}
REVEAL_ONLY = {"holdout", "target", "all"}
HOLDOUT_START = dt.date(2025, 7, 1)
DEFAULT_SPEC = {"config_id": 12, "patches": [], "args": [], "tickers": None, "lookback": 8, "capital": 10000,
                "bars_dir": "data/bars_iex",
                "cost_args": [], "sizing": ["--sizing-fraction", "0.36", "--max-position-pct", "0.36", "--cross-index", "SPY"]}


# ---------------------------------------------------------------- windows / sessions

def window_bounds(w: str) -> tuple[dt.date, dt.date]:
    if w in WINDOWS:
        a, b = WINDOWS[w]
    elif ":" in w:
        a, b = w.split(":", 1)
    elif len(w) == 4 and w.isdigit():
        a, b = f"{w}-01-01", f"{w}-12-31"
    else:
        sys.exit(f"unknown window {w!r}")
    return dt.date.fromisoformat(a), dt.date.fromisoformat(b)


def check_reveal(w: str, start: dt.date, end: dt.date) -> None:
    if os.environ.get("LAB_REVEAL") == "1":
        return
    if w in REVEAL_ONLY or end >= HOLDOUT_START:
        sys.exit(f"window {w} touches the holdout (from {HOLDOUT_START}); refused without LAB_REVEAL=1 (PLAN.md)")


def sessions(bars_dir: Path, start: dt.date, end: dt.date) -> list[dt.date]:
    """eastern session dates present in the SPY bar cache within [start, end]."""
    spy = bars_dir / "SPY.csv"
    days: set[dt.date] = set()
    last = None
    with open(spy) as fh:
        next(fh)
        for line in fh:
            ts = int(line.split(",", 1)[0])
            d = dt.datetime.fromtimestamp(ts, EASTERN).date()
            if d != last:
                if start <= d <= end:
                    days.add(d)
                last = d
    return sorted(days)


# ---------------------------------------------------------------- specs

def load_spec(path: str) -> dict:
    p = Path(path)
    spec = dict(DEFAULT_SPEC)
    spec.update(json.loads(p.read_text()))
    spec.setdefault("name", p.stem)
    spec["_path"] = str(p)
    return spec


def spec_fingerprint(spec: dict) -> tuple[str, dict]:
    """hash of everything that changes a day's replay: binary, bars dir, config row, patches (content),
    args, tickers, costs, sizing, capital, lookback. returns (hash, the resolved material)."""
    st = BIN.stat()
    material = {
        "bin": f"{int(st.st_mtime)}:{st.st_size}",
        "bars_dir": spec["bars_dir"],
        "config_id": spec["config_id"],
        "patches": [(p, hashlib.sha1(Path(ROOT / p).read_bytes()).hexdigest()) for p in spec["patches"]],
        "args": list(map(str, spec["args"])),
        "tickers": spec.get("tickers"),
        "cost_args": list(map(str, spec["cost_args"])),
        "sizing": list(map(str, spec["sizing"])),
        "capital": spec["capital"],
        "lookback": spec["lookback"],
    }
    h = hashlib.sha1(json.dumps(material, sort_keys=True).encode()).hexdigest()[:16]
    return h, material


def backtest_cmd(spec: dict, day: dt.date) -> list[str]:
    cmd = [str(BIN), "--date", day.isoformat(), "--lookback-days", str(spec["lookback"]),
           "--capital", str(spec["capital"]), "--output-trades-csv", "--bars-dir", str(ROOT / spec["bars_dir"]),
           "--config-id", str(spec["config_id"])]
    cmd += list(map(str, spec["cost_args"]))
    cmd += list(map(str, spec["sizing"]))
    if spec.get("tickers"):
        cmd += ["--tickers", spec["tickers"]]
    for p in spec["patches"]:
        cmd += ["--patch-json", str(ROOT / p)]
    cmd += list(map(str, spec["args"]))
    return cmd


# ---------------------------------------------------------------- slot semaphore

class Slot:
    """one of N machine-wide slots (flock on /tmp/lab_slots/slot_i). fair enough for a shared box."""

    def __init__(self) -> None:
        SLOTS.mkdir(exist_ok=True)
        self.fh = None

    def __enter__(self):
        delay = 0.02
        while True:
            for i in range(N_SLOTS):
                fh = open(SLOTS / f"slot_{i}", "w")
                try:
                    fcntl.flock(fh, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    self.fh = fh
                    return self
                except OSError:
                    fh.close()
            time.sleep(delay)
            delay = min(delay * 1.5, 0.5)

    def __exit__(self, *exc):
        fcntl.flock(self.fh, fcntl.LOCK_UN)
        self.fh.close()


# ---------------------------------------------------------------- running

_env_lock = threading.Lock()
_env: dict | None = None


def env() -> dict:
    global _env
    with _env_lock:
        if _env is None:
            e = dict(os.environ)
            envf = ROOT / ".env"
            if envf.exists():
                for line in envf.read_text().splitlines():
                    line = line.strip()
                    if line and not line.startswith("#") and "=" in line:
                        k, v = line.split("=", 1)
                        e.setdefault(k.strip(), v.strip().strip('"').strip("'"))
            _env = e
        return _env


def run_day(spec: dict, fp: str, day: dt.date) -> tuple[dt.date, str, str | None]:
    """returns (day, csv_text_of_trade_rows, error). cached in data/lab_cache/<fp>/<day>.csv"""
    cdir = CACHE / fp
    cdir.mkdir(parents=True, exist_ok=True)
    out = cdir / f"{day.isoformat()}.csv"
    errf = cdir / f"{day.isoformat()}.err"
    if out.exists():
        return day, out.read_text(), None
    if errf.exists():
        return day, "", errf.read_text()
    cmd = ["nice", "-n", "19"] + backtest_cmd(spec, day)  # the box is a desktop: yield to the owner
    with Slot():
        try:
            p = subprocess.run(cmd, capture_output=True, text=True, timeout=600, env=env(), cwd=ROOT)
        except subprocess.TimeoutExpired:
            errf.write_text("timeout")
            return day, "", "timeout"
    text = p.stdout
    ok = p.returncode == 0 and "summary," in text
    if not ok:
        err = (p.stderr or text).strip().splitlines()
        err = "\n".join(err[-5:]) if err else f"exit {p.returncode}"
        if "no data" in err:  # holiday / missing bars: a skipped day, not an error
            out.write_text("")
            return day, "", None
        errf.write_text(err)
        return day, "", err
    rows = "\n".join(l for l in text.splitlines() if l.startswith("trade,") or l.startswith("row_type,"))
    tmp = out.with_suffix(".tmp")
    tmp.write_text(rows + ("\n" if rows else ""))
    os.replace(tmp, out)
    return day, rows, None


HEADER = ("row_type,date,ticker,direction,entry_time,exit_time,entry_price,exit_price,size,pnl,pnl_pct,"
          "hold_duration_ms,exit_reason,entry_reason,candle_pattern,entry_composite,exit_composite,entry_1m,"
          "entry_5m,entry_1h,exit_1m,exit_5m,exit_1h,max_composite,max_composite_time,positive_ticks,total_ticks")


def parse_rows(csv_text: str) -> list[dict]:
    rows = []
    if not csv_text.strip():
        return rows
    lines = [l for l in csv_text.splitlines() if l.strip()]
    if not lines[0].startswith("row_type,"):
        lines.insert(0, HEADER)
    for r in csv.DictReader(io.StringIO("\n".join(lines))):
        if r.get("row_type") != "trade":
            continue
        r["pnl"] = float(r["pnl"])
        r["year"] = r["date"][:4]
        rows.append(r)
    return rows


def run_spec(spec: dict, days: list[dt.date], quiet: bool = False) -> tuple[list[dict], dict]:
    fp, material = spec_fingerprint(spec)
    rows: list[dict] = []
    errors: dict[str, str] = {}
    done = 0
    t0 = time.time()
    with ThreadPoolExecutor(max_workers=N_SLOTS) as ex:
        for day, text, err in ex.map(lambda d: run_day(spec, fp, d), days):
            done += 1
            if err:
                errors[day.isoformat()] = err
            else:
                rows.extend(parse_rows(text))
            if not quiet and (done % 50 == 0 or done == len(days)):
                print(f"  [{spec['name']}] {done}/{len(days)} days, {len(rows)} trades, {time.time() - t0:.0f}s",
                      file=sys.stderr)
    rows.sort(key=lambda r: r["entry_time"])
    return rows, {"fingerprint": fp, "material": material, "errors": errors, "n_days": len(days)}


# ---------------------------------------------------------------- metrics

def _pf(pnls) -> float:
    gw = sum(x for x in pnls if x > 0)
    gl = -sum(x for x in pnls if x < 0)
    return round(gw / gl, 3) if gl > 0 else (99.0 if gw > 0 else 0.0)


def _block(rows: list[dict]) -> dict:
    pnls = [r["pnl"] for r in rows]
    s = pm.stats(rows) if rows else {"pnl": 0.0, "n": 0, "win_rate": 0.0, "pf": 0.0, "dd": 0.0}
    return {"pnl": s["pnl"], "n": s["n"], "pf": _pf(pnls), "win_rate": s["win_rate"], "dd": s["dd"]}


def _group(rows: list[dict], key) -> dict:
    g = defaultdict(list)
    for r in rows:
        g[key(r)].append(r)
    return {k: _block(v) for k, v in sorted(g.items())}


def summarize(rows: list[dict], days: list[dt.date], start: dt.date, end: dt.date) -> dict:
    m = {"window": [start.isoformat(), end.isoformat()], "sessions": len(days)}
    m["total"] = _block(rows)
    months = max(1, (end.year - start.year) * 12 + end.month - start.month + 1)
    m["trades_per_month"] = round(len(rows) / months, 1)
    m["by_year"] = _group(rows, lambda r: r["year"])
    m["by_half"] = _group(rows, lambda r: r["date"][:4] + ("h1" if int(r["date"][5:7]) <= 6 else "h2"))
    m["by_ticker"] = _group(rows, lambda r: r["ticker"])
    m["by_direction"] = _group(rows, lambda r: r["direction"])
    m["by_entry_reason"] = _group(rows, lambda r: r.get("entry_reason", ""))
    m["by_exit_reason"] = _group(rows, lambda r: r.get("exit_reason", ""))
    m["by_month"] = {k: v["pnl"] for k, v in _group(rows, lambda r: r["date"][:7]).items()}
    if rows:
        daily = pm.daily_pnl(rows)
        m["worst_day"] = round(min(daily.values()), 2)
        m["best_day"] = round(max(daily.values()), 2)
        m["active_days"] = len(daily)
        m["sign_flip_p"] = pm.sign_flip_p(daily, n=3000, seed=0)
        m["pf_ci90"] = pm.bootstrap_pf(rows, n=1000, seed=0)
        m["pnl_ci90"] = pm.bootstrap_pnl(rows, n=1000, seed=0)
        years_present = sorted({r["year"] for r in rows})
        if len(years_present) > 1:
            ypnl = {y: sum(r["pnl"] for r in rows if r["year"] == y) for y in years_present}
            best = max(ypnl, key=ypnl.get)
            exb = _block([r for r in rows if r["year"] != best])
            exb["excluded_year"] = best
            m["ex_best_year"] = exb
        m["avg_hold_min"] = round(sum(float(r["hold_duration_ms"]) for r in rows) / len(rows) / 60000, 1)
        m["avg_pnl_per_trade"] = round(m["total"]["pnl"] / len(rows), 2)
    return m


def fmt_block(name: str, b: dict) -> str:
    return f"{name:<22} {b['pnl']:>8.0f} {b['n']:>5} {b['pf']:>6.2f} {b['win_rate']:>5.1f}% dd {b['dd']:>7.0f}"


def print_summary(spec_name: str, m: dict, errors: dict) -> None:
    t = m["total"]
    print(f"== {spec_name}  window {m['window'][0]}..{m['window'][1]}  ({m['sessions']} sessions)")
    print(f"   P&L {t['pnl']:+.0f}  trades {t['n']}  PF {t['pf']:.2f}  win {t['win_rate']:.1f}%  dd {t['dd']:.0f}"
          f"  trades/mo {m['trades_per_month']}")
    if t["n"]:
        print(f"   sign-flip p {m['sign_flip_p']}  PF ci90 {m['pf_ci90']}  P&L ci90 {m['pnl_ci90']}"
              f"  worst day {m['worst_day']:.0f}  best day {m['best_day']:.0f}  avg hold {m['avg_hold_min']} min"
              f"  $/trade {m['avg_pnl_per_trade']}")
        if "ex_best_year" in m:
            e = m["ex_best_year"]
            print(f"   ex-best-year ({e['excluded_year']}): P&L {e['pnl']:+.0f} / {e['n']} / PF {e['pf']:.2f}")
    for title, key in (("by year", "by_year"), ("by half", "by_half"), ("by ticker", "by_ticker"),
                       ("by entry", "by_entry_reason"), ("by exit", "by_exit_reason"), ("by direction", "by_direction")):
        if m[key]:
            print(f"   -- {title}")
            for k, b in m[key].items():
                print("   " + fmt_block(str(k)[:22], b))
    if errors:
        print(f"   !! {len(errors)} day(s) failed, e.g. {next(iter(errors.items()))}")


# ---------------------------------------------------------------- ledger

def ledger_append(entry: dict) -> None:
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    with open(LEDGER, "a") as fh:
        fcntl.flock(fh, fcntl.LOCK_EX)
        fh.write(json.dumps(entry, sort_keys=True) + "\n")
        fcntl.flock(fh, fcntl.LOCK_UN)


def ledger_rows() -> list[dict]:
    if not LEDGER.exists():
        return []
    return [json.loads(l) for l in LEDGER.read_text().splitlines() if l.strip()]


def compact(m: dict) -> dict:
    t = m["total"]
    out = {"pnl": t["pnl"], "n": t["n"], "pf": t["pf"], "dd": t["dd"], "trades_per_month": m["trades_per_month"],
           "sign_flip_p": m.get("sign_flip_p"), "worst_day": m.get("worst_day"),
           "by_year": {y: [b["pnl"], b["n"], b["pf"]] for y, b in m["by_year"].items()}}
    if "ex_best_year" in m:
        out["ex_best_year_pf"] = m["ex_best_year"]["pf"]
    return out


# ---------------------------------------------------------------- commands

def cmd_run(a) -> None:
    spec = load_spec(a.spec)
    start, end = window_bounds(a.window)
    check_reveal(a.window, start, end)
    days = sessions(ROOT / spec["bars_dir"], start, end)
    rows, info = run_spec(spec, days, quiet=a.quiet)
    m = summarize(rows, days, start, end)
    if not a.json:
        print_summary(spec["name"], m, info["errors"])
    else:
        print(json.dumps({"spec": spec["name"], "metrics": m, "errors": info["errors"]}, indent=1))
    RUNS.mkdir(parents=True, exist_ok=True)
    safe = spec["name"].replace("/", "__")
    (RUNS / f"{safe}.{a.window}.json").write_text(json.dumps(
        {"spec": {k: v for k, v in spec.items() if not k.startswith("_")}, "spec_path": spec["_path"],
         "fingerprint": info["fingerprint"], "window": a.window, "metrics": m, "errors": info["errors"]}, indent=1))
    with open(RUNS / f"{safe}.{a.window}.trades.csv", "w", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(HEADER.split(","))
        for r in rows:
            w.writerow([r.get(c, "") for c in HEADER.split(",")])
    ledger_append({"ts": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"), "agent": a.agent,
                   "name": spec["name"], "spec_path": spec["_path"], "fingerprint": info["fingerprint"],
                   "window": a.window, "note": spec.get("note", ""), **compact(m)})


def tag_rows(tag: str, start: dt.date, end: dt.date) -> list[dict]:
    rows = pm.load_trades(tag)
    for r in rows:
        r["year"] = r["date"][:4]
    return [r for r in rows if start.isoformat() <= r["date"] <= end.isoformat()]


def cmd_score(a) -> None:
    start, end = window_bounds(a.window)
    check_reveal(a.window, start, end)
    days = sessions(ROOT / "data" / "bars_iex", start, end)
    for tag in a.tags:
        rows = tag_rows(tag, start, end)
        m = summarize(rows, days, start, end)
        if a.brief:
            t = m["total"]
            yrs = " ".join(f"{y[2:]}:{b['pnl']:+.0f}/{b['n']}/{b['pf']:.2f}" for y, b in m["by_year"].items())
            print(f"{tag:<36} {t['pnl']:>+7.0f} {t['n']:>5} PF {t['pf']:>5.2f} dd {t['dd']:>6.0f} "
                  f"p {m.get('sign_flip_p', '-')!s:<5} exb {m.get('ex_best_year', {}).get('pf', '-')!s:<5} {yrs}")
        else:
            print_summary(tag, m, {})


def _rows_for(ref: str, start: dt.date, end: dt.date) -> tuple[str, list[dict]]:
    if ref.endswith(".json"):
        spec = load_spec(ref)
        days = sessions(ROOT / spec["bars_dir"], start, end)
        rows, _ = run_spec(spec, days, quiet=True)
        return spec["name"], rows
    return ref, tag_rows(ref, start, end)


def cmd_compare(a) -> None:
    start, end = window_bounds(a.window)
    check_reveal(a.window, start, end)
    na, ra = _rows_for(a.a, start, end)
    nb, rb = _rows_for(a.b, start, end)
    days = [d.isoformat() for d in sessions(ROOT / "data" / "bars_iex", start, end)]
    p = pm.paired_day_bootstrap(ra, rb, session_list=days, n=5000, seed=0)
    sa, sb = _block(ra), _block(rb)
    print(f"A {na}: {sa['pnl']:+.0f} / {sa['n']} / PF {sa['pf']:.2f}    B {nb}: {sb['pnl']:+.0f} / {sb['n']} / PF {sb['pf']:.2f}")
    print(f"paired day bootstrap A−B over {p['n_days']} sessions: P&L diff {p['pnl_diff']:+.0f}, "
          f"PF diff {p['pf_diff']:+.3f}, P(diff ≤ 0) {p['p_le_0']:.3f}")
    print("  P&L diff quantiles: " + ", ".join(f"q{int(q * 100):02d} {v:+.0f}" for q, v in p["pnl_quantiles"]))
    corr = pm.pearson_corr(pm.daily_series(ra, days), pm.daily_series(rb, days))
    print(f"  daily P&L correlation {corr:+.2f}")


def cmd_top(a) -> None:
    rows = ledger_rows()
    if a.window:
        rows = [r for r in rows if r["window"] == a.window]
    if a.agent:
        rows = [r for r in rows if r.get("agent") == a.agent]
    best: dict[tuple, dict] = {}
    for r in rows:  # latest entry per (fingerprint, window)
        best[(r["fingerprint"], r["window"])] = r
    rows = sorted(best.values(), key=lambda r: (r["pf"] if r["n"] >= a.min_trades else -1, r["pnl"]), reverse=True)
    print(f"{'name':<40} {'window':<10} {'pnl':>7} {'n':>5} {'pf':>5} {'dd':>6} {'p':>5} {'exb':>5}  by year")
    for r in rows[: a.n]:
        yrs = " ".join(f"{y[2:]}:{v[0]:+.0f}/{v[1]}/{v[2]:.2f}" for y, v in r["by_year"].items())
        print(f"{r['name'][:40]:<40} {r['window']:<10} {r['pnl']:>+7.0f} {r['n']:>5} {r['pf']:>5.2f} {r['dd']:>6.0f} "
              f"{r.get('sign_flip_p') if r.get('sign_flip_p') is not None else '-':>5} "
              f"{r.get('ex_best_year_pf', '-')!s:>5}  {yrs}")


def cmd_sessions(a) -> None:
    start, end = window_bounds(a.window)
    for d in sessions(ROOT / "data" / "bars_iex", start, end):
        print(d.isoformat())


def cmd_cells(a) -> None:
    rows = ledger_rows()
    if a.agent:
        rows = [r for r in rows if r.get("agent") == a.agent]
    print(f"{len({r['fingerprint'] for r in rows})} distinct specs, {len(rows)} runs"
          + (f" for agent {a.agent}" if a.agent else ""))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run"); r.add_argument("spec"); r.add_argument("--window", default="train")
    r.add_argument("--agent", default=os.environ.get("LAB_AGENT", "coordinator"))
    r.add_argument("--json", action="store_true"); r.add_argument("--quiet", action="store_true"); r.set_defaults(f=cmd_run)
    s = sub.add_parser("score"); s.add_argument("tags", nargs="+"); s.add_argument("--window", default="train")
    s.add_argument("--brief", action="store_true"); s.set_defaults(f=cmd_score)
    c = sub.add_parser("compare"); c.add_argument("a"); c.add_argument("b"); c.add_argument("--window", default="train")
    c.set_defaults(f=cmd_compare)
    t = sub.add_parser("top"); t.add_argument("--window"); t.add_argument("--agent"); t.add_argument("-n", type=int, default=30)
    t.add_argument("--min-trades", type=int, default=100); t.set_defaults(f=cmd_top)
    se = sub.add_parser("sessions"); se.add_argument("--window", default="train"); se.set_defaults(f=cmd_sessions)
    ce = sub.add_parser("cells"); ce.add_argument("--agent"); ce.set_defaults(f=cmd_cells)
    a = ap.parse_args()
    a.f(a)


if __name__ == "__main__":
    main()
