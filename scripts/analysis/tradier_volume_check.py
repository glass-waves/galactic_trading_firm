#!/usr/bin/env python3
"""compare Tradier 1-minute bar volume against our SIP and IEX caches for one session.

usage: tradier_volume_check.py YYYY-MM-DD [SYM ...]   (default: AAPL AMZN MSFT NVDA SPY)
env:   TRADIER_TOKEN (required); TRADIER_BASE (default https://api.tradier.com; sandbox: https://sandbox.tradier.com)
reads data/bars/<SYM>.csv (SIP) and data/bars_iex/<SYM>.csv (IEX); prints per-symbol RTH totals and the
minute-level correlation / ratio. a consolidated feed sums to ≈ 1.0 × SIP; IEX is ≈ 0.03 × SIP.
"""
import csv, json, os, sys, urllib.parse, urllib.request
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

ET = ZoneInfo("America/New_York")
day = sys.argv[1]
syms = sys.argv[2:] or ["AAPL", "AMZN", "MSFT", "NVDA", "SPY"]
tok = os.environ.get("TRADIER_TOKEN")
if not tok:
    sys.exit("TRADIER_TOKEN not set (put it in .env and `set -a; source .env; set +a`)")
base = os.environ.get("TRADIER_BASE", "https://api.tradier.com")


def tradier_minutes(sym):
    q = urllib.parse.urlencode({"symbol": sym, "interval": "1min", "start": f"{day} 09:30", "end": f"{day} 16:00", "session_filter": "open"})
    req = urllib.request.Request(f"{base}/v1/markets/timesales?{q}", headers={"Authorization": f"Bearer {tok}", "Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        js = json.load(r)
    data = (js.get("series") or {}).get("data") or []
    out = {}
    for b in data if isinstance(data, list) else [data]:
        # tradier 'time' is ET wall clock, e.g. "2026-10-02T09:30:00"
        hm = b["time"][11:16]
        out[hm] = float(b.get("volume") or 0)
    return out


def cache_minutes(path):
    out = {}
    try:
        f = open(path)
    except FileNotFoundError:
        return out
    for r in csv.reader(f):
        if r[0] == "timestamp":
            continue
        ts = datetime.fromtimestamp(int(r[0]), timezone.utc).astimezone(ET)
        if ts.strftime("%Y-%m-%d") == day:
            out[ts.strftime("%H:%M")] = float(r[5])
    return out


def corr(a, b):
    ks = sorted(set(a) & set(b))
    if len(ks) < 10:
        return float("nan"), len(ks)
    xa = [a[k] for k in ks]; xb = [b[k] for k in ks]
    ma = sum(xa) / len(xa); mb = sum(xb) / len(xb)
    cov = sum((x - ma) * (y - mb) for x, y in zip(xa, xb))
    va = sum((x - ma) ** 2 for x in xa) ** 0.5; vb = sum((y - mb) ** 2 for y in xb) ** 0.5
    return (cov / (va * vb) if va and vb else float("nan")), len(ks)


print(f"{'sym':5s} {'tradier':>12s} {'SIP':>12s} {'IEX':>12s} {'trd/SIP':>8s} {'trd/IEX':>8s} {'corr(trd,SIP)':>14s} {'bars':>5s}")
for s in syms:
    t = tradier_minutes(s); sip = cache_minutes(f"data/bars/{s}.csv"); iex = cache_minutes(f"data/bars_iex/{s}.csv")
    T, S, I = sum(t.values()), sum(sip.values()), sum(iex.values())
    c, n = corr(t, sip)
    print(f"{s:5s} {T:12,.0f} {S:12,.0f} {I:12,.0f} {T/S if S else float('nan'):8.3f} {T/I if I else float('nan'):8.1f} {c:14.3f} {len(t):5d}")
    first = sorted(t)[:3]
    print("      first minutes tradier/SIP/IEX:", [(k, int(t[k]), int(sip.get(k, 0)), int(iex.get(k, 0))) for k in first])
