#!/usr/bin/env python3
"""walk-forward selection over the existing IEX grid cells (no new sweeps).

each cell is a five-year per-trade file set data/<tag>_<year>_trades.csv. the short cells are
the SPY band x VPIN floor grid; the long options are trade subsets that can be added to any
short cell because the replay runs tickers independently (shorts are identical across cells).
every quarter Q from 2023Q1, pick the combo with the best objective over the trailing 12 months
(minimum trade count), trade it in Q, concatenate. prints the out-of-sample record next to the
static v18 cell and a "fit on 2026 only" exhibit.

usage: walk_forward.py [--objective pf|pnl|sharpe] [--min-trades N] [--lookback-months M]
"""
import argparse, csv, glob, math
from collections import defaultdict
from datetime import date

SHORT_CELLS = {  # label -> tag
    "±0.2% · no VPIN": "e_iex_b0.4_novpin", "±0.2% · 0.12": "e_iex_b0.4_v0.12", "±0.2% · 0.15": "e_iex_b0.4_v0.15",
    "±0.2% · 0.18": "e_iex_b0.4_v0.18", "±0.2% · 0.217 (v18)": "iex_v18", "±0.2% · 0.26": "e_iex_b0.4_v0.26",
    "±0.3% · no VPIN": "e_iex_b0.6_novpin", "±0.3% · 0.12": "e_iex_b0.6_v0.12", "±0.3% · 0.15": "e_iex_b0.6_v0.15",
    "±0.3% · 0.18": "e_iex_b0.6_v0.18", "±0.3% · 0.217": "e_iex_b0.6_v0.217", "±0.3% · 0.26": "e_iex_b0.6_v0.26",
}
LONG_OPTIONS = {  # label -> (tag, side filter)
    "longs off": None,
    "longs: filtered, all days": "rl_none",
    "longs: SPY < SMA50": "rl_below_sma50",
    "longs: SPY 20d ret < 0": "rl_below_ret20",
    "longs: SPY > SMA20": "rl_sma20",
}


def load(tag, side=None):
    out = []
    for f in glob.glob(f"data/{tag}_*_trades.csv"):
        for r in csv.DictReader(open(f)):
            if r["row_type"] != "trade":
                continue
            if side and r["direction"].lower() != side:
                continue
            out.append((date.fromisoformat(r["date"]), float(r["pnl"]), r["direction"].lower()))
    return sorted(out)


def metrics(trades):
    if not trades:
        return dict(pnl=0.0, n=0, pf=0.0, sharpe=0.0, win=0.0, dd=0.0)
    pnl = sum(p for _, p, _ in trades)
    wins = sum(p for _, p, _ in trades if p > 0); losses = -sum(p for _, p, _ in trades if p < 0)
    daily = defaultdict(float)
    for d, p, _ in trades:
        daily[d] += p
    vals = list(daily.values()); m = sum(vals) / len(vals)
    sd = math.sqrt(sum((v - m) ** 2 for v in vals) / len(vals)) if len(vals) > 1 else 0.0
    eq = peak = dd = 0.0
    for d in sorted(daily):
        eq += daily[d]; peak = max(peak, eq); dd = min(dd, eq - peak)
    return dict(pnl=pnl, n=len(trades), pf=(wins / losses if losses else 9.99), sharpe=(m / sd * math.sqrt(252) if sd else 0.0),
                win=sum(1 for _, p, _ in trades if p > 0) / len(trades), dd=dd)


def quarter_bounds(y, q):
    start = date(y, 3 * (q - 1) + 1, 1)
    end = date(y + (q == 4), (3 * q) % 12 + 1, 1)
    return start, end


def months_back(d, m):
    y, mo = d.year, d.month - m
    while mo <= 0:
        y -= 1; mo += 12
    return date(y, mo, 1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--objective", default="pf", choices=["pf", "pnl", "sharpe"])
    ap.add_argument("--min-trades", type=int, default=40)
    ap.add_argument("--lookback-months", type=int, default=12)
    a = ap.parse_args()

    shorts = {lbl: load(tag, "short") for lbl, tag in SHORT_CELLS.items()}
    longs = {lbl: (load(tag, "long") if tag else []) for lbl, tag in LONG_OPTIONS.items()}
    combos = {(s, l): sorted(shorts[s] + longs[l]) for s in shorts for l in longs}
    quarters = [(y, q) for y in range(2023, 2027) for q in range(1, 5) if (y, q) <= (2026, 3)]

    oos, picks = [], []
    for y, q in quarters:
        qs, qe = quarter_bounds(y, q)
        if y == 2026 and q == 3:
            qe = date(2026, 9, 11)
        fs = months_back(qs, a.lookback_months)
        best, best_key = None, None
        for key, tr in combos.items():
            fit = [t for t in tr if fs <= t[0] < qs]
            m = metrics(fit)
            if m["n"] < a.min_trades:
                continue
            score = m[a.objective]
            if best is None or score > best:
                best, best_key = score, key
        if best_key is None:
            best_key = ("±0.2% · 0.217 (v18)", "longs off")
        q_tr = [t for t in combos[best_key] if qs <= t[0] < qe]
        oos += q_tr
        picks.append((f"{y}Q{q}", best_key, best, metrics(q_tr)))

    def line(name, m):
        return f"{name:44s} {m['pnl']:+7.0f}  n={m['n']:4d}  PF={m['pf']:.2f}  win={m['win']*100:3.0f}%  dd={m['dd']:6.0f}  sharpe={m['sharpe']:.2f}"

    print(f"walk-forward: objective={a.objective}, lookback={a.lookback_months}m, min_trades={a.min_trades}, OOS 2023Q1..2026Q3\n")
    print(f"{'quarter':8s} {'chosen short cell':22s} {'long option':28s} {'fit ' + a.objective:>8s}   quarter result")
    for qn, (s, l), sc, m in picks:
        print(f"{qn:8s} {s:22s} {l:28s} {sc:8.2f}   {m['pnl']:+6.0f} n={m['n']:3d} PF={m['pf']:.2f}")
    static = [t for t in combos[("±0.2% · 0.217 (v18)", "longs off")] if date(2023, 1, 1) <= t[0] < date(2026, 9, 11)]
    print()
    print(line("walk-forward OOS 2023-2026Q3", metrics(oos)))
    print(line("static v18 same period", metrics(static)))
    for lbl in ["longs: SPY < SMA50"]:
        tr = [t for t in combos[("±0.2% · 0.217 (v18)", lbl)] if date(2023, 1, 1) <= t[0] < date(2026, 9, 11)]
        print(line(f"static v18 + {lbl} same period", metrics(tr)))
    print("\nper-year, walk-forward OOS vs static v18:")
    for y in range(2023, 2027):
        yo = metrics([t for t in oos if t[0].year == y]); ys = metrics([t for t in static if t[0].year == y])
        print(f"  {y}  WF {yo['pnl']:+6.0f} n={yo['n']:3d} PF={yo['pf']:.2f}   |  v18 {ys['pnl']:+6.0f} n={ys['n']:3d} PF={ys['pf']:.2f}")

    # exhibit: fit on 2026 only (Jan-Sep 2026), rank combos, and show what the same combo did 2022-2025
    print("\nexhibit — the five best combos fitted on 2026 alone, and their 2022-2025 record:")
    rows = []
    for key, tr in combos.items():
        m26 = metrics([t for t in tr if t[0].year == 2026]); mpre = metrics([t for t in tr if t[0].year < 2026])
        if m26["n"] >= a.min_trades:
            rows.append((m26[a.objective], key, m26, mpre))
    for sc, (s, l), m26, mpre in sorted(rows, reverse=True)[:5]:
        print(f"  {s:22s} {l:28s} 2026: {m26['pnl']:+6.0f} n={m26['n']:3d} PF={m26['pf']:.2f}   2022-25: {mpre['pnl']:+6.0f} n={mpre['n']:4d} PF={mpre['pf']:.2f}")


if __name__ == "__main__":
    main()
