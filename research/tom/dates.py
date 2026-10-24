#!/usr/bin/env python3
"""turn-of-month session 2 (RQ-11): the second NYSE trading day of every calendar month.

trading days come from the bar cache's actual sessions (data/bars/SPY.csv, SIP; falls back to
data/bars_iex/SPY.csv). months after the cache ends (or not fully in it) use weekdays minus the
NYSE holidays listed below -- extend FUTURE_HOLIDAYS before using this past 2026.

  python3 research/tom/dates.py                 # print the dates, one per line
  python3 research/tom/dates.py --write-patch   # rewrite the dates in research/tom/tom_long.patch.json
"""
import datetime as dt, json, sys, zoneinfo
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
START, END = dt.date(2022, 1, 1), dt.date(2026, 12, 31)
ET = zoneinfo.ZoneInfo("America/New_York")
FUTURE_HOLIDAYS = {dt.date(2026, 11, 26), dt.date(2026, 12, 25)}  # only months past the cache need these
PATCH = ROOT / "research/tom/tom_long.patch.json"
CAL_ID = "tom_day2"


def cache_sessions():
    for p in (ROOT / "data/bars/SPY.csv", ROOT / "data/bars_iex/SPY.csv"):
        if p.exists():
            with open(p) as f:
                next(f)
                return sorted({dt.datetime.fromtimestamp(int(l.split(",", 1)[0]), ET).date() for l in f}), p
    sys.exit("no SPY bar cache (data/bars or data/bars_iex)")


def second_sessions():
    days, src = cache_sessions()
    last = days[-1]
    d = last + dt.timedelta(days=1)
    while d <= END:                                   # calendar fallback past the cache
        if d.weekday() < 5 and d not in FUTURE_HOLIDAYS:
            days.append(d)
        d += dt.timedelta(days=1)
    by_month = defaultdict(list)
    for d in days:
        if START <= d <= END:
            by_month[(d.year, d.month)].append(d)
    out = [v[1] for _, v in sorted(by_month.items()) if len(v) >= 2]
    return out, src, last


if __name__ == "__main__":
    dates, src, last = second_sessions()
    if "--write-patch" in sys.argv:
        patch = json.loads(PATCH.read_text())
        ind = next(i for i in patch["indicators"] if i["instance_id"] == CAL_ID)
        ind["params"]["dates"] = [d.isoformat() for d in dates]
        PATCH.write_text(json.dumps(patch, indent=1) + "\n")
        print(f"{len(dates)} dates -> {PATCH.relative_to(ROOT)} (cache {src.relative_to(ROOT)} to {last}; calendar after)")
    else:
        print("\n".join(d.isoformat() for d in dates))
