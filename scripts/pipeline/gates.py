"""backtest gates: pure functions over the metrics dicts produced by metrics.full_metrics().

every gate returns {"gate": name, "pass": bool, "checks": [{name, value, threshold, ok}]}; that json is
what pipeline_candidates.backtest_result stores. thresholds are stated at the research sizing
(--sizing-fraction 0.36 --max-position-pct 0.36), which is what every gate sweep runs at
(plan §4.3 as amended by §9 "B (pipeline)").

  default-ticker : 5y PF >= 1.3, >= 4 of 5 years positive, >= 100 trades, no year < -300, 2026 >= 0
  volume-config  : trades >= baseline, PF >= 1.3, >= 4 of 5 years positive, P&L >= baseline - 10 %, no year < -300
  quality-config : PF >= baseline + 0.05, >= 4 of 5 years positive, trades >= 0.6 x baseline, no year < -300
  stress-mode    : volume-config with trades >= 0.9 x baseline, plus on SPY < -1 % days: total P&L >= 3 x
                   baseline's, >= +40 per such day on average, no single day < -300; ordinary-day P&L
                   within +-10 % of baseline
  additive-config: for a candidate that leaves every baseline trade untouched and only adds a new window
                   (metrics['marginal'], from metrics.marginal_metrics()): the base subset must match the
                   baseline's P&L within +-2 % and trade count within +-1 % (base_unchanged); the added
                   subset must be net positive with PF >= 1.3, >= 15 trades over 5y, no year worse than
                   -100 and no single day worse than -300; and no year of the *combined* book may be more
                   than 50 worse than the baseline's same year (combined_years_not_worse) — this is the
                   quality-config years_positive requirement's replacement for a candidate whose baseline
                   itself has flat/negative years an additive edge can never repair
  sizing-config  : for a candidate that only changes position sizing (no new entries/exits): same trade
                   set as the baseline (metrics['marginal'], via metrics.marginal_metrics()/split_marginal() -
                   trade count within +-2 % of baseline's and the matched ('base') subset >= 95 % of the
                   baseline's trade count, i.e. almost every baseline trade still fires); P&L >= baseline
                   x 1.10; PF >= baseline PF - 0.02; max drawdown no more than 30 % deeper than the
                   baseline's (dd >= baseline dd x 1.3); every year's P&L >= that year's baseline P&L - 50;
                   and the largest single position observed (metrics['max_position_fraction'], size x
                   entry_price / 10,000 capital) <= the candidate's own effective sizing cap (its
                   _sweep_args' --max-position-pct if it has one, else the standard 0.36) x 1.02 - a 2 %
                   tolerance for whole-share rounding and fill-vs-sizing-price drift, not a fixed 0.45:
                   a candidate that raises its own cap via _sweep_args is measured against that cap, not
                   the standard sweep's
  additive-ticker: for a ticker candidate judged on its MARGINAL effect on the live book, not its own
                   stand-alone five-year record (default-ticker's bar: PF >= 1.3, >= 4 of 5 years, >= 100
                   trades — stricter for a newcomer than the incumbents ever had to clear). the candidate's
                   sweep (--tickers T alone) is concatenated with the baseline book's trades (baseline tag
                   'iex_v18', via metrics.marginal_metrics(): every candidate trade is 'added' since
                   trade_key()'s ticker never matches one of the baseline's four names, so 'base' is always
                   empty for this gate and is not checked). checks: added trades >= 40 (5y); added P&L > 0;
                   added PF >= 1.15; combined PF (baseline_rows + added_rows) >= baseline PF - 0.02 (not
                   dilutive); combined_years_not_worse: no year of the combined book more than 50 worse
                   than the baseline's same year; added worst year >= -150. bar-cache coverage of the
                   candidate's own ticker is enforced the same way as default-ticker, before the gate ever
                   runs (pipeline.py's kind-level coverage check, not a metrics check here). the live
                   3-position cap is reported as info (metrics.concurrency_exceedance() over the combined
                   set) and never gated — the replay runs each ticker set independently so a straight
                   concatenation is exact except for that cap
"""
from __future__ import annotations

import re

YEAR_FLOOR = -300.0
MIN_PF = 1.3
MIN_YEARS_POSITIVE = 4
STANDARD_MAX_POSITION_PCT = 0.36
MAX_POSITION_FRACTION_TOLERANCE = 1.02
ADDED_TICKER_MIN_TRADES = 40
ADDED_TICKER_MIN_PF = 1.15
ADDED_TICKER_WORST_YEAR_FLOOR = -150.0
COMBINED_PF_TOLERANCE = 0.02
COMBINED_YEAR_TOLERANCE = 50.0


def check(name: str, value, threshold: str, ok: bool) -> dict:
    if isinstance(value, float):
        value = round(value, 3)
    return {"name": name, "value": value, "threshold": threshold, "ok": bool(ok)}


def _result(gate: str, checks: list[dict]) -> dict:
    return {"gate": gate, "pass": all(c["ok"] for c in checks), "checks": checks}


def _minus_pct(x: float, pct: float) -> float:
    """x reduced by pct of its magnitude (so a negative baseline still gets a lower bound below it)."""
    return x - pct * abs(x)


def _common(m: dict) -> list[dict]:
    return [
        check("years_positive", m["years_positive"], f">= {MIN_YEARS_POSITIVE} of 5", m["years_positive"] >= MIN_YEARS_POSITIVE),
        check("min_year_pnl", m["min_year_pnl"], f">= {YEAR_FLOOR:+.0f} (36 % sizing)", m["min_year_pnl"] >= YEAR_FLOOR),
    ]


def default_ticker(m: dict, baseline: dict | None = None) -> dict:
    checks = [
        check("pf_5y", m["pf"], f">= {MIN_PF}", m["pf"] >= MIN_PF),
        *_common(m),
        check("trades_5y", m["n"], ">= 100", m["n"] >= 100),
        check("pnl_2026", m["pnl_2026"], ">= 0", m["pnl_2026"] >= 0),
    ]
    return _result("default-ticker", checks)


def _need_baseline(gate: str, baseline: dict | None) -> dict | None:
    if not baseline:
        return _result(gate, [check("baseline", None, "baseline metrics required", False)])
    return None


def volume_config(m: dict, baseline: dict | None = None, gate: str = "volume-config", trade_factor: float = 1.0) -> dict:
    err = _need_baseline(gate, baseline)
    if err:
        return err
    b = baseline
    need_trades = trade_factor * b["n"]
    pnl_floor = _minus_pct(b["pnl"], 0.10)
    checks = [
        check("trades_5y", m["n"], f">= {need_trades:.0f} ({trade_factor:g} x baseline {b['n']})", m["n"] >= need_trades),
        check("pf_5y", m["pf"], f">= {MIN_PF}", m["pf"] >= MIN_PF),
        *_common(m),
        check("pnl_5y", m["pnl"], f">= {pnl_floor:+.0f} (baseline {b['pnl']:+.0f} - 10 %)", m["pnl"] >= pnl_floor),
    ]
    return _result(gate, checks)


def quality_config(m: dict, baseline: dict | None = None) -> dict:
    err = _need_baseline("quality-config", baseline)
    if err:
        return err
    b = baseline
    need_pf = b["pf"] + 0.05
    need_trades = 0.6 * b["n"]
    checks = [
        check("pf_5y", m["pf"], f">= {need_pf:.3f} (baseline {b['pf']:.3f} + 0.05)", m["pf"] >= need_pf),
        *_common(m),
        check("trades_5y", m["n"], f">= {need_trades:.0f} (0.6 x baseline {b['n']})", m["n"] >= need_trades),
    ]
    return _result("quality-config", checks)


def stress_mode(m: dict, baseline: dict | None = None) -> dict:
    base = volume_config(m, baseline, gate="stress-mode", trade_factor=0.9)
    if not baseline or not base["checks"] or base["checks"][0]["name"] == "baseline":
        return base
    ms, bs = m.get("stress"), baseline.get("stress")
    if not ms or not bs:
        base["checks"].append(check("stress_buckets", None, "SPY day returns required (data/bars_iex/SPY.csv)", False))
        base["pass"] = False
        return base
    need_total = 3.0 * bs["stress_pnl"]
    ord_lo, ord_hi = _minus_pct(bs["ordinary_pnl"], 0.10), bs["ordinary_pnl"] + 0.10 * abs(bs["ordinary_pnl"])
    extra = [
        check("stress_pnl", ms["stress_pnl"], f">= {need_total:+.0f} (3 x baseline {bs['stress_pnl']:+.0f}) on {ms['stress_days']} SPY < -1 % days", ms["stress_pnl"] >= need_total),
        check("stress_pnl_per_day", ms["stress_pnl_per_day"], ">= +40 per SPY < -1 % day", ms["stress_pnl_per_day"] >= 40.0),
        check("stress_worst_day", ms["stress_worst_day"], ">= -300", ms["stress_worst_day"] >= -300.0),
        check("ordinary_pnl", ms["ordinary_pnl"], f"within [{ord_lo:+.0f}, {ord_hi:+.0f}] (baseline {bs['ordinary_pnl']:+.0f} +-10 %)", ord_lo <= ms["ordinary_pnl"] <= ord_hi),
    ]
    checks = base["checks"] + extra
    return _result("stress-mode", checks)


def additive_config(m: dict, baseline: dict | None = None) -> dict:
    """for a candidate that only adds a new window on top of an unchanged baseline (plan: see module
    docstring). needs metrics['marginal'] (metrics.marginal_metrics()'s output: base vs added trades)."""
    err = _need_baseline("additive-config", baseline)
    if err:
        return err
    b = baseline
    mg = m.get("marginal")
    if not mg:
        return _result("additive-config", [check("marginal", None, "marginal split required (metrics.marginal_metrics against the baseline tag)", False)])
    base, added = mg["base"], mg["added"]
    base_pnl_lo, base_pnl_hi = _minus_pct(b["pnl"], 0.02), b["pnl"] + 0.02 * abs(b["pnl"])
    n_lo, n_hi = 0.99 * b["n"], 1.01 * b["n"]
    year_margins = {y: round(m["years"][y]["pnl"] - b["years"][y]["pnl"], 2) for y in b["years"]}
    worst_margin_year = min(year_margins, key=lambda y: year_margins[y])
    checks = [
        check("base_unchanged_pnl", base["pnl"], f"within [{base_pnl_lo:+.0f}, {base_pnl_hi:+.0f}] (baseline {b['pnl']:+.0f} +-2 %)", base_pnl_lo <= base["pnl"] <= base_pnl_hi),
        check("base_unchanged_trades", base["n"], f"within [{n_lo:.1f}, {n_hi:.1f}] (baseline {b['n']} +-1 %)", n_lo <= base["n"] <= n_hi),
        check("added_pnl", added["pnl"], "> 0", added["pnl"] > 0),
        check("added_pf", added["pf"], f">= {MIN_PF}", added["pf"] >= MIN_PF),
        check("added_trades", added["n"], ">= 15 (5y)", added["n"] >= 15),
        check("added_worst_year", added["worst_year_pnl"], ">= -100 (36 % sizing)", added["worst_year_pnl"] >= -100.0),
        check("added_worst_day", added["worst_day"], ">= -300", added["worst_day"] >= -300.0),
        check("combined_years_not_worse", year_margins[worst_margin_year],
              f">= -50 vs baseline per year (worst: {worst_margin_year})", year_margins[worst_margin_year] >= -50.0),
    ]
    return _result("additive-config", checks)


def additive_ticker(m: dict, baseline: dict | None = None) -> dict:
    """for a ticker candidate judged on its marginal effect on the live book (plan: see module
    docstring), not its own stand-alone five-year record (default_ticker()). needs metrics['marginal']
    (metrics.marginal_metrics() against baseline_tag='iex_v18') for both the 'added' subset — which
    is the candidate's whole trade set, since trade_key()'s ticker never matches the baseline's four
    names — and 'combined' (baseline_rows + added_rows, the book as if the candidate traded alongside
    the baseline). 'base' is not checked: there is nothing for this gate to leave unchanged."""
    err = _need_baseline("additive-ticker", baseline)
    if err:
        return err
    b = baseline
    mg = m.get("marginal")
    if not mg or "combined" not in mg:
        return _result("additive-ticker", [check("marginal", None, "marginal split required (metrics.marginal_metrics against baseline_tag='iex_v18')", False)])
    added, combined = mg["added"], mg["combined"]
    need_pf = b["pf"] - COMBINED_PF_TOLERANCE
    year_margins = {y: round(combined["years"][y]["pnl"] - b["years"][y]["pnl"], 2) for y in b["years"]}
    worst_margin_year = min(year_margins, key=lambda y: year_margins[y])
    conc = mg.get("concurrency") or {}
    checks = [
        check("added_trades", added["n"], f">= {ADDED_TICKER_MIN_TRADES} (5y)", added["n"] >= ADDED_TICKER_MIN_TRADES),
        check("added_pnl", added["pnl"], "> 0", added["pnl"] > 0),
        check("added_pf", added["pf"], f">= {ADDED_TICKER_MIN_PF}", added["pf"] >= ADDED_TICKER_MIN_PF),
        check("combined_pf", combined["pf"], f">= {need_pf:.3f} (baseline {b['pf']:.3f} - {COMBINED_PF_TOLERANCE:g}, not dilutive)", combined["pf"] >= need_pf),
        check("combined_years_not_worse", year_margins[worst_margin_year],
              f">= -{COMBINED_YEAR_TOLERANCE:.0f} vs baseline per year (worst: {worst_margin_year})", year_margins[worst_margin_year] >= -COMBINED_YEAR_TOLERANCE),
        check("added_worst_year", added["worst_year_pnl"], f">= {ADDED_TICKER_WORST_YEAR_FLOOR:+.0f} (36 % sizing)", added["worst_year_pnl"] >= ADDED_TICKER_WORST_YEAR_FLOOR),
        check("concurrency_info", conc.get("n_exceedance_days", 0),
              f"info only, not gated (cap {conc.get('cap', 3)}; max concurrent observed {conc.get('max_concurrent_observed', 0)})", True),
    ]
    return _result("additive-ticker", checks)


def effective_max_position_pct(sweep_args_override: str | None, default: float = STANDARD_MAX_POSITION_PCT) -> float:
    """the candidate's own sizing cap: the value its _sweep_args passed to --max-position-pct (the
    flag the gate sweep actually ran with — see docs/pipeline.md's _sweep_args section and
    crates/backtest/src/main.rs get_arg(), which resolves a repeated flag to its last occurrence),
    else the standard research sizing cap. a malformed or missing value falls back to `default`."""
    if sweep_args_override:
        m = re.search(r"--max-position-pct[ =](\S+)", sweep_args_override)
        if m:
            try:
                return float(m.group(1))
            except ValueError:
                pass
    return default


def sizing_config(m: dict, baseline: dict | None = None, sweep_args_override: str | None = None) -> dict:
    """for a candidate that only changes position sizing (same entries/exits as the baseline, a
    bigger or smaller fraction on some of them). needs metrics['marginal'] (metrics.marginal_metrics()'s
    base/added split against the baseline tag, built from metrics.split_marginal()/trade_key()) to
    confirm the trade set itself did not change, and metrics['max_position_fraction'] (from
    metrics.max_position_fraction(), already folded into metrics.summarize()) to cap position size.
    `sweep_args_override` is the candidate's stored patch `_sweep_args` (pipeline.py's
    backtest_result.sweep_args_override) - it raises the cap check's own threshold exactly as it
    raised the sweep's clamp, so a candidate that asked to run at a higher cap is judged against
    that cap, not the standard sweep's 0.36."""
    err = _need_baseline("sizing-config", baseline)
    if err:
        return err
    b = baseline
    mg = m.get("marginal")
    if not mg:
        return _result("sizing-config", [check("marginal", None, "marginal split required (metrics.marginal_metrics against the baseline tag)", False)])
    base = mg["base"]
    n_lo, n_hi = 0.98 * b["n"], 1.02 * b["n"]
    common_frac = (base["n"] / b["n"]) if b["n"] else 0.0
    need_pnl = b["pnl"] * 1.10
    need_pf = b["pf"] - 0.02
    dd_floor = b["dd"] * 1.3
    year_margins = {y: round(m["years"][y]["pnl"] - b["years"][y]["pnl"], 2) for y in b["years"]}
    worst_margin_year = min(year_margins, key=lambda y: year_margins[y])
    max_frac = m.get("max_position_fraction", 0.0)
    cap = effective_max_position_pct(sweep_args_override)
    cap_threshold = cap * MAX_POSITION_FRACTION_TOLERANCE
    checks = [
        check("same_trade_count", m["n"], f"within [{n_lo:.1f}, {n_hi:.1f}] (baseline {b['n']} +-2 %)", n_lo <= m["n"] <= n_hi),
        check("same_trade_set", common_frac, f">= 0.95 of baseline's {b['n']} trade keys in common (date, ticker, entry_time, direction)", common_frac >= 0.95),
        check("pnl_5y", m["pnl"], f">= {need_pnl:+.0f} (baseline {b['pnl']:+.0f} x 1.10)", m["pnl"] >= need_pnl),
        check("pf_5y", m["pf"], f">= {need_pf:.3f} (baseline {b['pf']:.3f} - 0.02)", m["pf"] >= need_pf),
        check("max_drawdown", m["dd"], f">= {dd_floor:+.0f} (baseline {b['dd']:+.0f} x 1.3, no more than 30 % deeper)", m["dd"] >= dd_floor),
        check("years_not_worse", year_margins[worst_margin_year],
              f">= -50 vs baseline per year (worst: {worst_margin_year})", year_margins[worst_margin_year] >= -50.0),
        check("max_position_fraction", max_frac,
              f"<= {cap_threshold:.4f} (cap {cap:g}{' from _sweep_args' if sweep_args_override else ' standard'} x 1.02; size x entry_price / 10,000 capital)",
              max_frac <= cap_threshold),
    ]
    return _result("sizing-config", checks)


GATES = {
    "default-ticker": default_ticker,
    "volume-config": volume_config,
    "quality-config": quality_config,
    "stress-mode": stress_mode,
    "additive-config": additive_config,
    "sizing-config": sizing_config,
    "additive-ticker": additive_ticker,
}
ALIASES = {"default-config": "volume-config", "default": None}
NEEDS_BASELINE = {"volume-config", "quality-config", "stress-mode", "additive-config", "sizing-config", "additive-ticker"}
NEEDS_MARGINAL = {"additive-config", "sizing-config", "additive-ticker"}


def resolve(name: str, kind: str) -> str:
    """map a requested gate name (or 'default') to a stable gate name for the candidate kind."""
    if name in GATES:
        return name
    if name == "default" or not name:
        return "default-ticker" if kind == "ticker" else "volume-config"
    if name in ALIASES and ALIASES[name]:
        return ALIASES[name]
    raise KeyError(f"unknown gate '{name}' (known: {', '.join(GATES)})")


def run_gate(name: str, metrics: dict, baseline: dict | None = None, sweep_args_override: str | None = None) -> dict:
    """sweep_args_override is only used by sizing-config (its cap check); every other gate ignores it."""
    if name == "sizing-config":
        return sizing_config(metrics, baseline, sweep_args_override=sweep_args_override)
    return GATES[name](metrics, baseline)


def failed_names(result: dict) -> list[str]:
    return [c["name"] for c in result.get("checks", []) if not c["ok"]]
