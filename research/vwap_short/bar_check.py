#!/usr/bin/env python3
"""raw-bar check of the VWAP-retest idea, independent of the engine (data/bars_iex, 1m RTH bars,
running typical-price VWAP from 09:30 like the replay).

usage: bar_check.py frontier            # retest-short outcome by SPY-at-entry vs SPY-at-close
       bar_check.py days D1,D2,...      # per-name crash-day walk: depth vs VWAP, first retest, SPY then
"retest" = close within [-0.15 %, +0.10 %] of VWAP after the name was >= 0.8 % below it, 10:00-15:15;
the short is entered at the next bar's open and held to the 15:55 close (no stop, no costs).
"""
import collections, csv, datetime, statistics, sys, zoneinfo

ET = zoneinfo.ZoneInfo("US/Eastern")
NAMES = ("AMZN", "AAPL", "NVDA", "MSFT")


def bars(t, only=None):
    d = collections.defaultdict(dict)
    for r in csv.DictReader(open(f"data/bars_iex/{t}.csv")):
        ts = datetime.datetime.fromtimestamp(int(r["timestamp"]), ET)
        ds = ts.date().isoformat()
        if (only and ds not in only) or ds > "2026-09-10":
            continue
        m = ts.hour * 60 + ts.minute
        if 570 <= m < 960:
            d[ds][m] = tuple(float(r[k]) for k in ("open", "high", "low", "close", "volume"))
    return d


def spy_ret(spy, d, m):
    b = spy[d]; o = b[min(b)][0]; ks = [k for k in b if k <= m]
    return (b[max(ks)][3] / o - 1) * 100


def walk(b, depth=0.8):
    """yield (minute, dist %, min dist so far %) per bar."""
    pv = vol = 0.0; mn = 0.0
    for m in sorted(b):
        o, h, l, c, v = b[m]; pv += (h + l + c) / 3 * v; vol += v
        if vol:
            dist = (c / (pv / vol) - 1) * 100; mn = min(mn, dist)
            yield m, dist, mn


def short_to_close(b, m):
    nxt = [k for k in sorted(b) if k > m]
    if not nxt:
        return None
    ex = [b[k][3] for k in sorted(b) if k <= 955][-1]
    return (b[nxt[0]][0] / ex - 1) * 100


def frontier():
    spy = bars("SPY"); close = {d: spy_ret(spy, d, 999) for d in spy}
    data = {t: bars(t) for t in NAMES}
    print("| SPY at entry | name-days | days | short->15:55 mean | win | of which SPY closed < -1 %: mean |")
    print("|---|---|---|---|---|---|")
    for thr in (None, 0.0, -0.5, -1.0, -1.5):
        hits = []
        for t, dd in data.items():
            for d, b in dd.items():
                if d not in spy:
                    continue
                for m, dist, mn in walk(b):
                    if 600 <= m < 915 and mn <= -0.8 and -0.15 <= dist <= 0.10 and (thr is None or spy_ret(spy, d, m) <= thr):
                        r = short_to_close(b, m)
                        if r is not None:
                            hits.append((d, r, close[d]))
                        break
        r = [h[1] for h in hits]; rt = [h[1] for h in hits if h[2] < -1]
        print(f"| {'any' if thr is None else f'<= {thr:+.1f} %'} | {len(hits)} | {len({h[0] for h in hits})} | "
              f"{statistics.mean(r):+.2f} % | {sum(x > 0 for x in r) / len(r) * 100:.0f} % | "
              f"{len(rt)}: {statistics.mean(rt) if rt else 0:+.2f} % |")


def days(ds):
    spy = bars("SPY", set(ds)); data = {t: bars(t, set(ds)) for t in NAMES}
    hm = lambda m: f"{m // 60:02d}:{m % 60:02d}"
    for d in ds:
        b = spy[d]; o = b[min(b)][0]
        cr = {x: next((hm(m) for m in sorted(b) if (b[m][3] / o - 1) * 100 <= x), "never") for x in (-1, -1.5, -2)}
        print(f"{d}: SPY {spy_ret(spy, d, 999):+.1f} %; first <= -1 % {cr[-1]}, <= -1.5 % {cr[-1.5]}, <= -2 % {cr[-2]}")
        for t in NAMES:
            bb = data[t].get(d)
            if not bb:
                continue
            first = None; mn = 0; tmn = None; below = n = 0
            pv = vol = 0.0
            for m, dist, mnn in walk(bb):
                if dist < mn:
                    mn, tmn = dist, m
                if m >= 600:
                    n += 1; below += dist < 0
                if first is None and 600 <= m < 915 and mnn <= -0.8 and -0.15 <= dist <= 0.10:
                    first = (m, spy_ret(spy, d, m), short_to_close(bb, m))
            o1 = bb[min(bb)][0]; ex = [bb[k][3] for k in sorted(bb) if k <= 955][-1]
            s = (f"  {t}: open->15:55 {(ex / o1 - 1) * 100:+.1f} %, deepest {mn:+.1f} % vs VWAP at {hm(tmn) if tmn else '-'}, "
                 f"below VWAP {below / max(n, 1) * 100:.0f} % of bars after 10:00; ")
            s += (f"first retest {hm(first[0])} with SPY {first[1]:+.1f} % -> short to 15:55 {first[2]:+.1f} %"
                  if first else "no retest 10:00-15:15")
            print(s)


if __name__ == "__main__":
    if sys.argv[1] == "frontier":
        frontier()
    else:
        days(sys.argv[2].split(","))
