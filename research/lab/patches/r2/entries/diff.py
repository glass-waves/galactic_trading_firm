#!/usr/bin/env python3
"""diff two lab trade files: diff.py CELL BASE WINDOW  (names as in research/lab/runs/<name>.<window>.trades.csv,
'/' -> '__'). Matches trades on (date, ticker, entry_time); prints removed / added / changed by ticker, window, year."""
import csv, sys, collections, pathlib
R = pathlib.Path(__file__).resolve().parents[5] / "research/lab/runs"
def load(n, w):
    f = R / f"{n.replace('/', '__')}.{w}.trades.csv"
    return {(r["date"], r["ticker"], r["entry_time"]): r for r in csv.DictReader(open(f)) if r["row_type"] == "trade"}
def summ(rows):
    p = [float(r["pnl"]) for r in rows]
    g = sum(x for x in p if x > 0); l = -sum(x for x in p if x < 0)
    return f"{sum(p):+7.0f} / {len(p):3d} / PF {g / l if l else float('inf'):5.2f}"
def by(rows, key):
    d = collections.defaultdict(list)
    for r in rows: d[key(r)].append(r)
    return "  ".join(f"{k}: {summ(v)}" for k, v in sorted(d.items()))
a, b, w = sys.argv[1], sys.argv[2], sys.argv[3]
A, B = load(a, w), load(b, w)
rem = [B[k] for k in B if k not in A]; add = [A[k] for k in A if k not in B]
chg = [(A[k], B[k]) for k in A if k in B and abs(float(A[k]["pnl"]) - float(B[k]["pnl"])) > 0.01]
print(f"{a} vs {b} [{w}]  kept {len(A) - len(add)}  removed {summ(rem)}  added {summ(add)}  changed {len(chg)} (Δ {sum(float(x['pnl']) - float(y['pnl']) for x, y in chg):+.0f})")
for lab, rows in (("removed", rem), ("added", add)):
    if not rows: continue
    print(f"  {lab} by ticker : {by(rows, lambda r: r['ticker'])}")
    print(f"  {lab} by window : {by(rows, lambda r: r['entry_reason'].replace('window:', ''))}")
    print(f"  {lab} by year   : {by(rows, lambda r: r['date'][:4])}")
    print(f"  {lab} by exit   : {by(rows, lambda r: r['exit_reason'])}")
