#!/usr/bin/env python3
"""what would today's (or any day's) near-miss bars have done as shorts?

usage: near_miss_replay.py [YYYY-MM-DD]   (default: today, America/New_York)
reads entry_block_events (kind='near_miss') for the day, the bar cache (data/bars, refresh it
first with `backtest --fetch-bars` for that day), and simulates a short entered at the next bar's
open with the v18 exit stack: hard stop 2.5 %, breakeven 0.5 % (close-based), losing limit 40 min /
winning 90 min, flat at 11:55 ET, 3 bps + $0.005 charged per leg. prints one line per near-miss.
"""
import csv, subprocess, sys
from datetime import datetime, timezone, timedelta
from zoneinfo import ZoneInfo

ET = ZoneInfo("America/New_York")
day = sys.argv[1] if len(sys.argv) > 1 else datetime.now(ET).strftime("%Y-%m-%d")

sql = f"""SELECT ticker, ts, reason FROM entry_block_events
WHERE kind='near_miss' AND (ts AT TIME ZONE 'America/New_York')::date = '{day}' ORDER BY ts"""
out = subprocess.run(["./scripts/psql.sh", "-At", "-F", "|", "-c", sql], capture_output=True, text=True).stdout
events = [l.split("|", 2) for l in out.strip().split("\n") if l]

def bars_for(ticker):
    rows = []
    for r in csv.reader(open(f"data/bars/{ticker}.csv")):
        if r[0] == "timestamp":
            continue
        ts = datetime.fromtimestamp(int(r[0]), timezone.utc)
        if ts.astimezone(ET).strftime("%Y-%m-%d") == day:
            rows.append((ts, float(r[1]), float(r[2]), float(r[3]), float(r[4])))
    return rows

def simulate(bars, i):
    """short at bars[i+1].open; returns (pnl_pct, reason, exit_hm)."""
    if i + 1 >= len(bars):
        return None
    entry = bars[i + 1][1] * (1 - 0.0003) - 0.005
    t0 = bars[i + 1][0]
    mfe = 0.0
    for j in range(i + 1, len(bars)):
        ts, o, h, l, c = bars[j]
        hm = ts.astimezone(ET).strftime("%H:%M")
        held = (ts - t0).total_seconds() / 60
        upl = (entry - c) / entry
        mfe = max(mfe, upl)
        reason = None
        if upl <= -0.025:
            reason = "hard_stop"
        elif mfe >= 0.005 and c >= entry:
            reason = "breakeven"
        elif (upl < 0 and held >= 40) or (upl > 0 and held >= 90) or (upl == 0 and held >= 90):
            reason = "max_hold"
        elif hm >= "11:55":
            reason = "session_close"
        if reason:
            fill = bars[j + 1][1] if j + 1 < len(bars) else c
            exitp = fill * (1 + 0.0003) + 0.005
            return ((entry - exitp) / entry * 100, reason, hm)
    return ((entry - bars[-1][4]) / entry * 100, "eod", "close")

cache = {}
print(f"near-miss bars on {day}: {len(events)}")
tot = 0.0
for ticker, ts, reason in events:
    if ticker not in cache:
        cache[ticker] = bars_for(ticker)
    bars = cache[ticker]
    t = datetime.fromisoformat(ts.replace(" ", "T")).astimezone(timezone.utc) if "+" in ts else datetime.strptime(ts[:19], "%Y-%m-%d %H:%M:%S").replace(tzinfo=timezone.utc)
    idx = next((k for k, b in enumerate(bars) if b[0] >= t), None)
    if idx is None:
        print(f"  {ticker} {ts[:16]}  no bars cached for this day")
        continue
    res = simulate(bars, idx)
    if not res:
        continue
    pnl, why, hm = res
    tot += pnl
    short_reason = reason.split("|")[0].strip()[:90]
    print(f"  {ticker:5s} {t.astimezone(ET).strftime('%H:%M')} ET  short would have made {pnl:+.2f}% ({why} at {hm})   filtered by: {short_reason}")
print(f"sum of hypothetical short returns: {tot:+.2f}%  (positive = the filters cost us; negative = they saved us)")
