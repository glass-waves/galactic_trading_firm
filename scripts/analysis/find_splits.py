#!/usr/bin/env python3
"""scan a bar cache directory for candidate stock splits, to help maintain data/splits.csv.

alpaca's split-adjusted cache rebases a ticker's *entire* history to the current share count,
so a plain day-over-day close-ratio screen only catches a split the day it happens — every
later refetch of the same cache shows it as already adjusted. this script does two things:

  1. day-over-day screen: flags any session whose close/prev_close (or the inverse, for a
     reverse split) snaps within 3% of a common split ratio. this catches a split that is
     still raw in the cache (e.g. right after it happens, before the next full refetch).
  2. --raw-dir cross-check: given a matching *unadjusted* ("raw") sibling cache directory
     (e.g. data/bars_raw next to data/bars), computes the exact
     raw_close / adjusted_close ratio on each ticker's last session, which is the precise,
     date-independent split factor — this is how the current data/splits.csv rows were found
     (see docs/research/2026-10-14_split_adjustment.md). use this when you still have both an
     old (pre-refetch) raw cache and the current adjusted one.

prints candidate `ticker,date,ratio` rows (same layout as data/splits.csv) to stdout, and a
human-readable diagnostic per candidate to stderr. this only proposes rows — nothing here
writes to data/splits.csv; a human reviews and merges them in.

usage:
  scripts/analysis/find_splits.py data/bars
  scripts/analysis/find_splits.py data/bars --raw-dir data/bars_raw
  scripts/analysis/find_splits.py data/bars --tickers NVDA,AVGO --min-ratio 1.4 --max-ratio 25
"""
import argparse
import csv
import glob
import os
import sys
from datetime import datetime, timezone
from zoneinfo import ZoneInfo

ET = ZoneInfo("America/New_York")

# ratios (new shares per old share) worth flagging; includes a few X-for-Y fractional splits.
COMMON_RATIOS = [1.25, 1.5, 2, 2.5, 3, 4, 5, 6, 7, 8, 10, 15, 20, 25, 30, 40, 50]


def session_closes(path):
    """{eastern_date: last_close_of_session} from a `<TICKER>.csv` cache file
    (epoch-seconds timestamp, open, high, low, close, volume; header row optional)."""
    out = {}
    with open(path, newline="") as f:
        r = csv.reader(f)
        first = next(r, None)
        if first and not first[0].lstrip("-").isdigit():
            pass  # header row, already consumed
        elif first:
            _consume_row(out, first)
        for row in r:
            _consume_row(out, row)
    return out


def _consume_row(out, row):
    if len(row) < 5:
        return
    try:
        ts = int(row[0])
        close = float(row[4])
    except ValueError:
        return
    d = datetime.fromtimestamp(ts, tz=timezone.utc).astimezone(ET).date()
    out[d] = close  # rows are in ascending timestamp order; last write per day wins


def nearest_common_ratio(x):
    """closest value in COMMON_RATIOS or its reciprocal to `x`, with relative error."""
    best = None
    for r in COMMON_RATIOS:
        for cand in (r, 1.0 / r):
            err = abs(x - cand) / cand
            if best is None or err < best[1]:
                best = (cand, err)
    return best


def fmt_ratio(x):
    if abs(x - round(x)) < 1e-6:
        return str(int(round(x)))
    return f"{x:.2f}".rstrip("0").rstrip(".")


def scan_day_over_day(closes, min_ratio, max_ratio):
    days = sorted(closes)
    out = []
    for prev, cur in zip(days, days[1:]):
        p, c = closes[prev], closes[cur]
        if p <= 0 or c <= 0:
            continue
        ratio = p / c  # > 1 for a forward split (price drops going into `cur`)
        if ratio < 1 / max_ratio or ratio > max_ratio:
            continue
        snapped, err = nearest_common_ratio(ratio)
        if snapped < min_ratio or err > 0.03:
            continue
        out.append((cur, snapped, ratio, err))
    return out


def find_split_regime_changes(adj_closes, raw_closes, threshold=0.10):
    """walk the raw/adjusted close ratio day by day and flag every step change bigger than
    `threshold`. the adjusted cache rebases the *whole* history to the current share count,
    so this ratio is flat (± rounding noise, well under 1%) everywhere except right at a
    split's effective date, where it steps down by exactly that split's factor — this finds
    every split in the overlap of the two caches, not just the most recent one.

    returns a list of `(effective_date, factor)`, oldest first; `factor` is the exact
    raw/adjusted step (new shares per old share), not snapped to a common ratio."""
    common = sorted(d for d in adj_closes if d in raw_closes and adj_closes[d] > 0)
    out = []
    if not common:
        return out
    prev_ratio = raw_closes[common[0]] / adj_closes[common[0]]
    for d in common[1:]:
        if adj_closes[d] <= 0:
            continue
        cur_ratio = raw_closes[d] / adj_closes[d]
        if prev_ratio > 0 and abs(cur_ratio / prev_ratio - 1.0) > threshold:
            out.append((d, prev_ratio / cur_ratio))
        prev_ratio = cur_ratio
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cache_dir")
    ap.add_argument("--raw-dir", default=None, help="matching unadjusted cache, for an exact ratio cross-check")
    ap.add_argument("--tickers", default=None, help="comma list; default: every *.csv in cache_dir")
    ap.add_argument("--min-ratio", type=float, default=1.4, help="ignore day-over-day jumps smaller than this")
    ap.add_argument("--max-ratio", type=float, default=25.0, help="ignore day-over-day jumps larger than this (likely bad data, not a split)")
    args = ap.parse_args()

    if args.tickers:
        tickers = [t.strip() for t in args.tickers.split(",") if t.strip()]
    else:
        tickers = sorted(
            os.path.splitext(os.path.basename(p))[0]
            for p in glob.glob(os.path.join(args.cache_dir, "*.csv"))
        )

    found_any = False
    for ticker in tickers:
        path = os.path.join(args.cache_dir, f"{ticker}.csv")
        if not os.path.isfile(path):
            continue
        adj_closes = session_closes(path)

        if args.raw_dir:
            raw_path = os.path.join(args.raw_dir, f"{ticker}.csv")
            if os.path.isfile(raw_path):
                raw_closes = session_closes(raw_path)
                for d, exact in find_split_regime_changes(adj_closes, raw_closes):
                    snapped, err = nearest_common_ratio(exact)
                    print(f"{ticker},{d.isoformat()},{fmt_ratio(snapped)}")
                    print(
                        f"  {ticker}: raw/adjusted ratio steps from one regime to the next"
                        f" on {d}, factor {exact:.4f} (snaps to {fmt_ratio(snapped)},"
                        f" err {err * 100:.1f}%)",
                        file=sys.stderr,
                    )
                    found_any = True
            continue  # --raw-dir given: that's the precise method, skip the noisier screen

        for date, snapped, raw_ratio, err in scan_day_over_day(adj_closes, args.min_ratio, args.max_ratio):
            print(f"{ticker},{date.isoformat()},{fmt_ratio(snapped)}")
            print(
                f"  {ticker} {date}: day-over-day close ratio {raw_ratio:.4f}"
                f" snaps to {fmt_ratio(snapped)} (err {err * 100:.1f}%)",
                file=sys.stderr,
            )
            found_any = True

    if not found_any:
        print("# no candidates found", file=sys.stderr)


if __name__ == "__main__":
    main()
