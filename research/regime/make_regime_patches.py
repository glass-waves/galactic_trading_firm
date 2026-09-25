#!/usr/bin/env python3
"""up-regime date lists from the SPY bar cache -> --patch-json files (research 2026-09-24).

each regime is decided at the open of day d from daily closes through d-1 (no look-ahead):
  sma50      SPY close > 50-day SMA
  sma20      SPY close > 20-day SMA
  ret20      SPY 20-day return > 0
  lowvol     20-day realized vol (daily, annualized) below its trailing-250-day median
  sma50_lowvol  both sma50 and lowvol
writes research/regime/regime_<name>.json: an indicators patch adding an `event_calendar`
instance `cal_<name>` whose dates are the regime days (score -1 on those days), plus
research/regime/regime_days.csv for the report. usage: make_regime_patches.py [bars_dir]
"""
import csv, json, statistics, sys
from collections import OrderedDict
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

ET = ZoneInfo("America/New_York")
bars = sys.argv[1] if len(sys.argv) > 1 else "data/bars_iex"
closes = OrderedDict()
for r in csv.DictReader(open(f"{bars}/SPY.csv")):
    d = datetime.fromtimestamp(int(r["timestamp"]), timezone.utc).astimezone(ET).strftime("%Y-%m-%d")
    closes[d] = float(r["close"])  # last RTH bar of the day wins
days = list(closes); px = [closes[d] for d in days]
import math
rets = [math.log(px[i] / px[i - 1]) for i in range(1, len(px))]
regimes = {k: [] for k in ("sma50", "sma20", "ret20", "lowvol", "sma50_lowvol")}
rows = []
for i in range(1, len(days)):
    d = days[i]; j = i - 1  # decide with data through days[j]
    row = {"date": d}
    if j >= 50:
        sma50 = sum(px[j - 49:j + 1]) / 50; row["sma50"] = px[j] > sma50
    if j >= 20:
        sma20 = sum(px[j - 19:j + 1]) / 20; row["sma20"] = px[j] > sma20
        row["ret20"] = px[j] > px[j - 20]
        rv = statistics.pstdev(rets[j - 20:j]) * math.sqrt(252); row["rv20"] = rv
    if j >= 271:
        hist = [statistics.pstdev(rets[k - 20:k]) * math.sqrt(252) for k in range(j - 250, j + 1)]
        row["lowvol"] = row["rv20"] < statistics.median(hist)
    if "sma50" in row and "lowvol" in row:
        row["sma50_lowvol"] = row["sma50"] and row["lowvol"]
    for k in regimes:
        if row.get(k):
            regimes[k].append(d)
    rows.append(row)
with open("research/regime/regime_days.csv", "w") as f:
    w = csv.writer(f); w.writerow(["date", "sma50", "sma20", "ret20", "lowvol", "sma50_lowvol", "rv20"])
    for r in rows:
        w.writerow([r["date"]] + [int(r[k]) if k in r else "" for k in ("sma50", "sma20", "ret20", "lowvol", "sma50_lowvol")] + [f"{r['rv20']:.3f}" if "rv20" in r else ""])
for k, ds in regimes.items():
    patch = {"indicators": [{"indicator_type": "event_calendar", "instance_id": f"cal_{k}", "timescale": "OneMinute",
                             "weight": 0.0, "enabled": True, "params": {"dates": ds, "before_days": 0, "after_days": 0}}], "actions": []}
    json.dump(patch, open(f"research/regime/regime_{k}.json", "w"))
    by_year = {}
    for d in ds: by_year[d[:4]] = by_year.get(d[:4], 0) + 1
    print(f"{k:13s} {len(ds):4d} days  " + "  ".join(f"{y}:{n}" for y, n in sorted(by_year.items())))
print(f"sessions per year: " + "  ".join(f"{y}:{sum(1 for d in days if d[:4]==y)}" for y in sorted({d[:4] for d in days})))
