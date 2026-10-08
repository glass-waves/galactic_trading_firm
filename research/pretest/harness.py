#!/usr/bin/env python3
"""stage-0 pretest harness: cheap python replicas of documented anomalies on the cached bars.

usage: harness.py [CELL_ID ...]        (no args = every cell in cells.py)
writes research/pretest/results/<cell>.json and research/pretest/results/summary.md.

conventions (same as research/entries/summarize.py and research/index_momentum/sim.py):
- minute grid: one row per RTH session, 390 slots, slot m = the bar 09:30+m ET (NaN when IEX has no bar).
- a clock time is a boundary T = minutes since 09:30 (0..390). entering at T fills at the open of the
  first bar >= T (or the session close when none); exiting at T fills at the close of the last bar < T.
- sizing: 36 % of $10,000 per trade in whole shares. costs per leg: 3 bps + $0.005/share.
- stops/targets are checked on bar high/low from the entry bar on; a gap through fills at the bar open;
  if stop and target are both inside one bar the stop wins (conservative).
- a trade's P&L is booked on its exit date; years are 2022..2026 (2026 partial)."""
import csv
import datetime as dt
import json
import os
import sys
import zoneinfo
from collections import defaultdict, namedtuple

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ET = zoneinfo.ZoneInfo("America/New_York")
NOTIONAL, COST_BPS, COST_SH = 0.36 * 10_000, 3.0, 0.005
YEARS = [2022, 2023, 2024, 2025, 2026]

# ---------------------------------------------------------------- (a) loaders
class Bars:
    """one symbol: dates (list of dt.date), o/h/l/c/v arrays shaped (sessions, 390)."""

    def __init__(self, sym, dates, grid):
        self.sym, self.dates = sym, dates
        self.o, self.h, self.l, self.c, self.v = (grid[:, :, k] for k in range(5))
        self.index = {d: i for i, d in enumerate(dates)}
        self.has = ~np.isnan(self.c)

    def entry_px(self, i, t):
        idx = np.flatnonzero(self.has[i, t:])
        if len(idx):
            return self.o[i, t + idx[0]], t + idx[0]
        return self.exit_px(i, 390), 389

    def exit_px(self, i, t):
        idx = np.flatnonzero(self.has[i, :t])
        return (self.c[i, idx[-1]] if len(idx) else np.nan)

    def session_open(self, i):
        return self.entry_px(i, 0)[0]

    def session_close(self, i):
        return self.exit_px(i, 390)

    def full(self, i):
        """a regular full session (not a 13:00 half day)."""
        return self.has[i, 380:].any()


def load_minute(sym):
    src, cache = f"{ROOT}/data/bars_iex/{sym}.csv", f"{HERE}/cache/{sym}.npz"
    if os.path.exists(cache) and os.path.getmtime(cache) >= os.path.getmtime(src):
        z = np.load(cache)
        return Bars(sym, [dt.date.fromordinal(int(x)) for x in z["dates"]], z["grid"])
    a = np.loadtxt(src, delimiter=",", skiprows=1)
    ts = a[:, 0].astype(np.int64)
    uday = np.unique(ts // 86400)
    off = {d: int(dt.datetime.fromtimestamp(d * 86400 + 54000, ET).utcoffset().total_seconds()) for d in uday}
    local = ts + np.array([off[d] for d in ts // 86400])
    day, minute = local // 86400, (local % 86400) // 60 - 570
    keep = (minute >= 0) & (minute < 390)
    day, minute, a = day[keep], minute[keep], a[keep]
    udays = np.unique(day)
    grid = np.full((len(udays), 390, 5), np.nan)
    grid[np.searchsorted(udays, day), minute] = a[:, 1:6]
    ords = np.array([(dt.date(1970, 1, 1) + dt.timedelta(days=int(d))).toordinal() for d in udays])
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    np.savez(cache, dates=ords, grid=grid)
    return Bars(sym, [dt.date.fromordinal(int(x)) for x in ords], grid)


def load_daily(sym):
    """research/swing/daily/<SYM>.csv (2021-06 on, ~59 names) -> dates, dict of arrays."""
    rows = list(csv.DictReader(open(f"{ROOT}/research/swing/daily/{sym}.csv")))
    return [dt.date.fromisoformat(r["date"]) for r in rows], {
        k: np.array([float(r[k]) for r in rows]) for k in ("open", "high", "low", "close", "volume")}


_BARS = {}
def bars(sym):
    if sym not in _BARS:
        _BARS[sym] = load_minute(sym)
    return _BARS[sym]

# ---------------------------------------------------------------- (b) rule interface
# a rule is f(b: Bars, i: int, p: dict) -> list[Order], called once per session i; it may read any
# session <= i but must only use data available at its own entry time.
# exit is a (session index, clock boundary): same session = intraday; later session = overnight hold.
Order = namedtuple("Order", "si t_in side xi t_out stop target tag", defaults=(None, None, ""))


def execute(b, o):
    e, m0 = b.entry_px(o.si, o.t_in)
    if not np.isfinite(e):
        return None
    x, xd = b.exit_px(o.xi, o.t_out), o.xi
    if o.stop is not None or o.target is not None:
        for k in range(o.si, o.xi + 1):
            lo, hi = (m0 if k == o.si else 0), (o.t_out if k == o.xi else 390)
            h, l, op = b.h[k, lo:hi], b.l[k, lo:hi], b.o[k, lo:hi]
            ok = ~np.isnan(h)
            stop_hit = ok & ((l <= o.stop) if o.side > 0 else (h >= o.stop)) if o.stop is not None else np.zeros_like(ok)
            tgt_hit = ok & ((h >= o.target) if o.side > 0 else (l <= o.target)) if o.target is not None else np.zeros_like(ok)
            hit = np.flatnonzero(stop_hit | tgt_hit)
            if len(hit):
                j = hit[0]
                lvl = o.stop if stop_hit[j] else o.target
                gapped = (op[j] - lvl) * o.side * (1 if stop_hit[j] else -1) < 0
                x, xd = (op[j] if gapped else lvl), k
                break
    if not np.isfinite(x):
        return None
    return fill(b.sym, o.side, b.dates[o.si], b.dates[xd], e, x, o.tag)

# ---------------------------------------------------------------- (c) cost model
def cost(e, x, shares):
    return (COST_BPS / 1e4 * (e + x) + 2 * COST_SH) * shares


def fill(sym, side, d_in, d_out, e, x, tag=""):
    sh = int(NOTIONAL / e)
    if sh <= 0:
        return None
    gross = side * (x - e) * sh
    pnl = gross - cost(e, x, sh)
    return {"sym": sym, "side": side, "entry_date": d_in.isoformat(), "date": d_out.isoformat(),
            "year": d_in.year, "entry": round(float(e), 4), "exit": round(float(x), 4), "shares": sh,
            "pnl": float(pnl), "bps": float(pnl / (e * sh) * 1e4), "gross_bps": float(gross / (e * sh) * 1e4),
            "overnight": d_out != d_in, "tag": tag}

# ---------------------------------------------------------------- (d) metrics
def metrics(trades, sessions):
    """sessions = number of sessions in the sample (for the active-day share)."""
    if not trades:
        return {"n": 0, "pnl": 0.0, "pf": 0.0, "years_pos": 0, "per_year": {}}
    pnl = np.array([t["pnl"] for t in trades])
    w, l = pnl[pnl > 0].sum(), -pnl[pnl < 0].sum()
    daily = defaultdict(float)
    for t in trades:
        daily[t["date"]] += t["pnl"]
    eq = np.cumsum([daily[d] for d in sorted(daily)])
    dd = float(np.min(eq - np.maximum.accumulate(np.maximum(eq, 0)))) if len(eq) else 0.0
    per_year = {}
    for y in YEARS:
        v = [t["pnl"] for t in trades if t["year"] == y]
        if v:
            yw, yl = sum(x for x in v if x > 0), -sum(x for x in v if x < 0)
            per_year[y] = {"pnl": round(sum(v), 1), "n": len(v), "pf": round(yw / yl, 2) if yl else 99.0,
                           "bps": round(float(np.mean([t["bps"] for t in trades if t["year"] == y])), 2)}
    span = 4.75                                   # 2022-01-03 .. 2026-10: the cache's length in years
    return {"n": len(trades), "pnl": round(float(pnl.sum()), 1), "win": round(float((pnl > 0).mean() * 100), 1),
            "pf": round(float(w / l), 3) if l else 99.0, "bps": round(float(np.mean([t["bps"] for t in trades])), 2),
            "gross_bps": round(float(np.mean([t["gross_bps"] for t in trades])), 2),
            "trades_yr": round(len(trades) / span, 1), "active_share": round(len(daily) / sessions, 4),
            "max_dd": round(dd, 1), "worst_day": round(min(daily.values()), 1),
            "pnl_ex_top3": round(float(np.sort(pnl)[:-3].sum()), 1),
            "years_pos": sum(1 for y in per_year.values() if y["pnl"] > 0), "per_year": per_year,
            "overnight": any(t["overnight"] for t in trades)}


def loyo(by_param):
    """by_param: {param_label: metrics}. for each year pick the label with the best P&L on the
    other four years; report the held-out year's P&L under that pick."""
    out, total = {}, 0.0
    for y in YEARS:
        def rest(m):
            return sum(v["pnl"] for k, v in m["per_year"].items() if int(k) != y)
        pick = max(by_param, key=lambda k: rest(by_param[k]))
        held = by_param[pick]["per_year"].get(y, {"pnl": 0.0})["pnl"]
        out[y] = {"pick": pick, "pnl": held}
        total += held
    return {"years": out, "pnl": round(total, 1), "years_pos": sum(1 for v in out.values() if v["pnl"] > 0)}


def kill(m, years_pos, pf):
    """the standard pre-registered bar: net positive in >= years_pos of 5 years and PF > pf."""
    fails = []
    if m["years_pos"] < years_pos:
        fails.append(f"{m['years_pos']}/5 years positive (needs {years_pos})")
    if m["pf"] <= pf:
        fails.append(f"PF {m['pf']:.2f} (needs > {pf})")
    return (not fails), ("; ".join(fails) or f"PF {m['pf']:.2f}, {m['years_pos']}/5 years")

# ---------------------------------------------------------------- (e) runner + outputs
def run_rule(rule, syms, p):
    trades, sessions = [], 0
    for s in syms:
        b = bars(s)
        sessions = max(sessions, len(b.dates))
        for i in range(len(b.dates)):
            for o in rule(b, i, p) or []:
                t = execute(b, o)
                if t:
                    trades.append(t)
    return trades, sessions


def plabel(p):
    return ",".join(f"{k}={v}" for k, v in p.items()) or "paper"


def run_cell(cell):
    res = {"id": cell["id"], "anomaly": cell["anomaly"], "kill": cell["kill_text"], "variants": []}
    for var in cell["variants"]:
        for inst in var.get("instruments", cell["instruments"]):
            syms = inst.split("+")
            grid = var.get("grid") or [{}]
            by_p, daily = {}, None
            for k, p in enumerate(grid):
                trades, sessions = run_rule(var["rule"], syms, p)
                by_p[plabel(p)] = metrics(trades, sessions)
                if k == 0:
                    daily = defaultdict(float)
                    for t in trades:
                        daily[t["date"]] += t["pnl"]
            row = {"variant": var["name"], "instrument": inst, "flag": var.get("flag"),
                   "tradable": var.get("flag") is None, "primary": by_p[plabel(grid[0])], "grid": by_p,
                   "daily_pnl": {d: round(v, 2) for d, v in sorted(daily.items())}}
            if len(grid) > 1:
                row["loyo"] = loyo(by_p)
            row["pass"], row["why"] = cell["verdict_fn"](row)
            res["variants"].append(row)
    if cell.get("extra"):
        res["extra"] = cell["extra"]()
    res["verdict"], res["verdict_text"] = cell["verdict"](res)
    with open(f"{HERE}/results/{cell['id']}.json", "w") as f:
        json.dump(res, f, indent=1, default=str)
    return res


def summary_md(results):
    out = ["# pretest summary (generated by harness.py)", "",
           "| cell | variant | inst | grid | n | net bps | PF | yrs+ | P&L | ex-top3 | LOYO (yrs+) | pass |",
           "|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in results:
        for v in r["variants"]:
            lo = v.get("loyo")
            for k, (lab, m) in enumerate(v["grid"].items()):
                head = (f"{v['variant']}{' (' + v['flag'] + ')' if v['flag'] else ''} | {v['instrument']}" if k == 0 else " | ")
                loy = f"{lo['pnl']:+.0f} ({lo['years_pos']}/5)" if lo and k == 0 else ""
                out.append(f"| {r['id'] if k == 0 else ''} | {head} | {lab} | {m['n']} | {m.get('bps', 0):+.1f} | {m['pf']:.2f} | "
                           f"{m['years_pos']}/5 | {m['pnl']:+.0f} | {m.get('pnl_ex_top3', 0):+.0f} | {loy} | {('yes' if v['pass'] else 'no') if k == 0 else ''} |")
        out.append(f"\n**{r['id']}: {r['verdict']}** — {r['verdict_text']}\n")
        out.append("| cell | variant | inst | grid | n | net bps | PF | yrs+ | P&L | ex-top3 | LOYO (yrs+) | pass |")
        out.append("|---|---|---|---|---|---|---|---|---|---|---|---|")
    out = out[:-2]
    open(f"{HERE}/results/summary.md", "w").write("\n".join(out) + "\n")
    return "\n".join(out)


def main():
    sys.path.insert(0, HERE)
    from cells import CELLS
    want = sys.argv[1:] or [c["id"] for c in CELLS]
    results = [run_cell(c) for c in CELLS if c["id"] in want]
    print(summary_md(results))


if __name__ == "__main__":
    main()
