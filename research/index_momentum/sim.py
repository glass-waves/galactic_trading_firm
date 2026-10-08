#!/usr/bin/env python3
"""reference simulation of the paper's noise-area rules on the bar cache (no engine), to separate
"is the edge there on 2022-26 IEX bars" from "what the engine's building blocks approximate".

exact per-minute sigma (14 sessions), gap-adjusted bands, decisions at HH:00/HH:30 from 10:00 to
15:30 on the bar that closes then (HH:29 / HH:59), fill next bar open, exit at the 15:59 bar open.
stop modes: `decision` = the paper (price vs max(UB, VWAP) / min(LB, VWAP) checked only at decision
bars), `minute` = the engine's vwap_stop (price through VWAP on any bar, +buffer), `none` = hold.
costs per leg: 3 bps + $0.005/share (x `--cost-mult`). 36 % of $10k per trade, whole shares.

usage: sim.py TICKER [--stop decision|minute|none] [--buf PCT] [--cost-mult K] [--every 30] [--band 1.0]
              [--start 30]   (first decision, minutes since 09:30; 150 = 12:00)
"""
import csv
import datetime as dt
import sys
import zoneinfo
from collections import defaultdict

ET = zoneinfo.ZoneInfo("America/New_York")


def arg(name, default):
    a = sys.argv
    return type(default)(a[a.index(name) + 1]) if name in a else default


def main():
    tk = sys.argv[1]
    mode, buf, cm, every = arg("--stop", "decision"), arg("--buf", 0.0) / 100, arg("--cost-mult", 1.0), arg("--every", 30)
    band, start = arg("--band", 1.0), arg("--start", 30)
    days = defaultdict(dict)
    for r in csv.DictReader(open(f"data/bars_iex/{tk}.csv")):
        t = dt.datetime.fromtimestamp(int(r["timestamp"]), ET)
        m = t.hour * 60 + t.minute - 570 + 1          # minutes since 09:30 incl. this bar
        if 1 <= m <= 390:
            days[t.date()][m] = (float(r["open"]), float(r["high"]), float(r["low"]), float(r["close"]), float(r["volume"]))
    dates = sorted(days)
    moves = []                                        # per past session: {m: |c/o-1|}
    prev_close = None
    trades = []
    for d in dates:
        bars = days[d]
        o = bars[min(bars)][0]
        full = min(bars) == 1 and max(bars) >= 389
        if len(moves) >= 14 and prev_close and full:
            sig = {}
            for m in range(1, 391):
                v = [s[m] for s in moves[-14:] if m in s]
                if v:
                    sig[m] = band * sum(v) / len(v)
            hi, lo = max(o, prev_close), min(o, prev_close)
            pos, entry, cum_pv, cum_v = 0, None, 0.0, 0.0
            for m in sorted(bars):
                op, h, l, c, v = bars[m]
                cum_pv += (h + l + c) / 3 * v
                cum_v += v
                vwap = cum_pv / cum_v if cum_v else c
                nxt = bars.get(m + 1)
                if m >= 389 or nxt is None:
                    if pos and (m >= 389 or nxt is None) and m >= 389:
                        trades.append((d, pos, entry, (nxt or bars[m])[0] if nxt else c))
                        pos = 0
                    continue
                ub, lb = hi * (1 + sig.get(m, 0)), lo * (1 - sig.get(m, 0))
                dec = m % every == 0 and start <= m <= 360
                if pos:
                    if mode == "minute" or (mode == "decision" and dec):
                        if mode == "minute":
                            out = (pos > 0 and c < vwap * (1 - buf)) or (pos < 0 and c > vwap * (1 + buf))
                        else:
                            out = (pos > 0 and c < max(ub, vwap)) or (pos < 0 and c > min(lb, vwap))
                        if out:
                            trades.append((d, pos, entry, nxt[0]))
                            pos = 0
                if not pos and dec and m < 360 + 1:
                    if c > ub and c > vwap * (1 + buf):
                        pos, entry = 1, nxt[0]
                    elif c < lb and c < vwap * (1 - buf):
                        pos, entry = -1, nxt[0]
        if bars:
            prev_close = bars[max(bars)][3]
            if full:
                moves.append({m: abs(b[3] / o - 1) for m, b in bars.items()})
    # pnl
    yr = defaultdict(list)
    for d, p, e, x in trades:
        sh = int(3600 / e)
        cost = cm * (0.0003 * (e + x) + 0.01) * sh
        yr[d.year].append(p * (x - e) * sh - cost)
    allp = [v for y in yr for v in yr[y]]
    def pf(v):
        w, l = sum(x for x in v if x > 0), -sum(x for x in v if x < 0)
        return w / l if l else float("inf")
    print(f"{tk} stop={mode} buf={buf*100:.2f}% cost x{cm} every {every} band {band} start {start}: {len(allp)} tr, P&L {sum(allp):+.0f}, PF {pf(allp):.2f}, "
          f"win {100*sum(1 for x in allp if x>0)/len(allp):.0f}%  | " + " ".join(f"{y}: {sum(v):+.0f}/{pf(v):.2f}" for y, v in sorted(yr.items())))


if __name__ == "__main__":
    main()
