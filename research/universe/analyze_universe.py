#!/usr/bin/env python3
"""universe-expansion analysis (2026-10-07).

Reads sweep CSVs for each candidate ticker under two tags:
  v18 tag  : cand_<id>_<year>_trades.csv   (default-ticker gate sweep the pipeline already ran)
  variant  : uq_<TICKER>_<year>_trades.csv (thrust-1h15 quality variant, run separately)

Applies the pre-registered selection rule (see RULE below), builds the combined book
(iex_v18's 4 names + qualifying names, using each name's *selected* variant), checks the
3-position concurrency cap, and runs a leave-one-year-out (LOYO) sanity check.

stdlib + scripts/pipeline/metrics.py only. Run from repo root:
    python3 research/universe/analyze_universe.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts" / "pipeline"))
import metrics  # noqa: E402

YEARS = metrics.YEARS

# pre-registered selection rule (stated BEFORE combined-book numbers are computed):
#   a name QUALIFIES if its own five-year record, under the variant picked for it below,
#   has PF >= 1.3, >= 4 of 5 years positive, >= 60 trades, no year below -300.
# variant-choice policy (also pre-registered): try v18 (cand_<id>) first; a name that fails
# v18 but clears the rule under thrust-1h15 (uq_<T>) qualifies under thrust-1h15 instead;
# a name clearing the rule under both is kept on v18 (no window change needed for it).
MIN_PF = 1.3
MIN_YEARS_POS = 4
MIN_TRADES = 60
YEAR_FLOOR = -300.0

BASE_TICKERS = ["AAPL", "AMZN", "MSFT", "NVDA"]
BASE_TAG = "iex_v18"


def passes_rule(m: dict) -> bool:
    return (
        m["pf"] >= MIN_PF
        and m["years_positive"] >= MIN_YEARS_POS
        and m["n"] >= MIN_TRADES
        and m["min_year_pnl"] >= YEAR_FLOOR
    )


def try_summarize(tag: str) -> dict | None:
    if not metrics.sweep_files_present(tag):
        return None
    m = metrics.summarize(tag)
    if m["n"] == 0:
        return None
    return m


def evaluate_candidate(ticker: str, cand_tag: str | None) -> dict:
    """returns {'ticker', 'v18': m|None, 'variant': m|None, 'selected_tag', 'selected_variant', 'qualifies'}"""
    v18 = try_summarize(cand_tag) if cand_tag else None
    variant = try_summarize(f"uq_{ticker}")
    chosen_tag, chosen_variant, chosen_m, qualifies = None, None, None, False
    if v18 and passes_rule(v18):
        chosen_tag, chosen_variant, chosen_m, qualifies = cand_tag, "v18", v18, True
    elif variant and passes_rule(variant):
        chosen_tag, chosen_variant, chosen_m, qualifies = f"uq_{ticker}", "thrust-1h15", variant, True
    else:
        # report under whichever has more trades for visibility, even though it fails
        cands = [(cand_tag, "v18", v18), (f"uq_{ticker}", "thrust-1h15", variant)]
        cands = [c for c in cands if c[2]]
        if cands:
            chosen_tag, chosen_variant, chosen_m = max(cands, key=lambda c: c[2]["n"])
    return {
        "ticker": ticker,
        "v18": v18,
        "variant": variant,
        "selected_tag": chosen_tag,
        "selected_variant": chosen_variant,
        "metrics": chosen_m,
        "qualifies": qualifies,
    }


def load_rows_for(ticker: str, tag: str) -> list[dict]:
    rows = metrics.load_trades(tag)
    # for per-ticker tags (cand_<id>, uq_<T>) all rows already belong to this ticker, but
    # guard anyway in case a tag ever carries more than one.
    return [r for r in rows if r.get("ticker") == ticker] if any(r.get("ticker") not in (None, ticker) for r in rows) else rows


def combined_rows(qualifying: list[dict]) -> list[dict]:
    base_rows = [r for r in metrics.load_trades(BASE_TAG) if r.get("ticker") in BASE_TICKERS]
    rows = list(base_rows)
    for c in qualifying:
        rows.extend(load_rows_for(c["ticker"], c["selected_tag"]))
    return rows


def concurrency_check(rows: list[dict], cap: int = 3) -> dict:
    """count distinct days on which more than `cap` positions would have been open at once,
    using entry_time/exit_time per row (same-day only; intraday-only system, no overnight)."""
    from collections import defaultdict
    by_day = defaultdict(list)
    for r in rows:
        et, xt = r.get("entry_time"), r.get("exit_time")
        if not et or not xt:
            continue
        by_day[r["date"]].append((et, xt))
    over_days = []
    max_seen = 0
    for day, spans in by_day.items():
        events = []
        for et, xt in spans:
            events.append((et, 1))
            events.append((xt, -1))
        events.sort(key=lambda e: (e[0], -e[1]))  # entries before exits at same timestamp
        cur = 0
        peak = 0
        for _, delta in events:
            cur += delta
            peak = max(peak, cur)
        max_seen = max(max_seen, peak)
        if peak > cap:
            over_days.append((day, peak))
    return {"days_checked": len(by_day), "days_over_cap": len(over_days), "max_concurrent": max_seen,
            "over_days_sample": over_days[:10]}


def loyo(qualifying_all_years_fn, qualifying: list[dict]):
    """leave-one-year-out: for each held-out year, re-run the *same* selection rule using
    only the other four years' trades for each candidate, then report the combined book's
    performance on the held-out year, for both the LOYO-selected set and the full set."""
    out = {}
    for held_out in YEARS:
        train_years = [y for y in YEARS if y != held_out]
        selected = []
        for c in qualifying_candidates_pool:
            tag = c["selected_tag"]
            if not tag:
                continue
            rows = load_rows_for(c["ticker"], tag)
            train_rows = [r for r in rows if r["year"] in train_years]
            m = metrics.stats(train_rows)
            m["years_positive"] = sum(1 for y in train_years if metrics.stats([r for r in train_rows if r["year"] == y])["pnl"] > 0)
            m["min_year_pnl"] = min((metrics.stats([r for r in train_rows if r["year"] == y])["pnl"] for y in train_years), default=0.0)
            if m["n"] > 0 and m["pf"] >= MIN_PF and m["years_positive"] >= 3 and m["n"] >= MIN_TRADES * 0.8 and m["min_year_pnl"] >= YEAR_FLOOR:
                selected.append(c)
        base_rows_ho = [r for r in metrics.load_trades(BASE_TAG) if r.get("ticker") in BASE_TICKERS and r["year"] == held_out]
        combined_ho = list(base_rows_ho)
        for c in selected:
            rows = load_rows_for(c["ticker"], c["selected_tag"])
            combined_ho.extend([r for r in rows if r["year"] == held_out])
        out[held_out] = {
            "selected": sorted(c["ticker"] for c in selected),
            "held_out_stats": metrics.stats(combined_ho),
        }
    return out


def main():
    cand_map_path = ROOT / "research" / "universe" / "candidates.json"
    cand_map = json.loads(cand_map_path.read_text())  # {ticker: cand_tag|null}

    results = []
    for ticker, cand_tag in cand_map.items():
        results.append(evaluate_candidate(ticker, cand_tag))

    global qualifying_candidates_pool
    qualifying_candidates_pool = [r for r in results if r["qualifies"]]

    print("=== per-name results ===")
    for r in results:
        m = r["metrics"]
        if m:
            print(f"{r['ticker']:6s} sel={r['selected_variant'] or '-':12s} "
                  f"PF={m['pf']:.2f} n={m['n']:4d} pnl={m['pnl']:+8.0f} "
                  f"years_pos={m['years_positive']} min_yr={m['min_year_pnl']:+7.0f} "
                  f"qualifies={r['qualifies']}")
        else:
            print(f"{r['ticker']:6s} NO DATA (sweep missing/empty)")

    print("\n=== qualifying names ===")
    print([r["ticker"] for r in qualifying_candidates_pool])

    rows = combined_rows(qualifying_candidates_pool)
    combined = metrics.stats(rows)
    combined["years"] = {y: metrics.stats([r for r in rows if r["year"] == y]) for y in YEARS}
    print("\n=== combined book (base 4 + qualifiers) ===")
    print(json.dumps({"pnl": combined["pnl"], "n": combined["n"], "pf": combined["pf"],
                       "dd": combined["dd"],
                       "years": {y: combined["years"][y]["pnl"] for y in YEARS}}, indent=2))

    base18 = metrics.summarize(BASE_TAG)
    print("\n=== v18 baseline (base 4 only) ===")
    print(json.dumps({"pnl": base18["pnl"], "n": base18["n"], "pf": base18["pf"]}, indent=2))

    conc = concurrency_check(rows)
    print("\n=== concurrency check ===")
    print(json.dumps(conc, indent=2))

    print("\n=== LOYO ===")
    loyo_out = loyo(None, qualifying_candidates_pool)
    print(json.dumps(loyo_out, indent=2))

    out = {
        "results": [
            {"ticker": r["ticker"], "selected_variant": r["selected_variant"],
             "qualifies": r["qualifies"], "metrics": r["metrics"]}
            for r in results
        ],
        "qualifying": [r["ticker"] for r in qualifying_candidates_pool],
        "combined": {"pnl": combined["pnl"], "n": combined["n"], "pf": combined["pf"], "dd": combined["dd"],
                     "years": {y: combined["years"][y]["pnl"] for y in YEARS}},
        "baseline_v18": {"pnl": base18["pnl"], "n": base18["n"], "pf": base18["pf"]},
        "concurrency": conc,
        "loyo": loyo_out,
    }
    (ROOT / "research" / "universe" / "results.json").write_text(json.dumps(out, indent=2))
    print("\nwrote research/universe/results.json")


if __name__ == "__main__":
    main()
