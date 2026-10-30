"""backtest gates: pure functions over the metrics dicts produced by metrics.full_metrics().

every gate returns {"gate": name, "pass": bool, "checks": [{name, value, threshold, ok}]}; that json is
what pipeline_candidates.backtest_result stores. thresholds are stated at the research sizing
(--sizing-fraction 0.36 --max-position-pct 0.36), which is what every gate sweep runs at
(plan §4.3 as amended by §9 "B (pipeline)").

  (recalibrated 2026-10-29 after the statistics audit, docs/pipeline.md "what the numbers can and cannot say":
   PF floor 1.4 = the owner's bar; ex-best-year PF on every config gate; a PAIRED day-bootstrap against the
   baseline replaces point-estimate increments — an upgrade claim needs the lower one-sided 90 % bound of the
   P&L difference > 0 at the Šidák level for the cells the study scanned; a volume claim must not be
   demonstrably worse. P(diff <= 0) and the top ticker's P&L share are reported on every result.)

  default-ticker : 5y PF >= 1.4, >= 4 of 5 years positive, >= 100 trades, no year < -300, 2026 >= 0, ex-best-year PF >= 1.2
  volume-config  : trades >= baseline, PF >= max(1.4, baseline - 0.02), >= 4 of 5 years positive, P&L >= baseline - 10 %,
                   no year < -300, drawdown no more than 25 % deeper than baseline's, $/trade >= 85 % of baseline's,
                   ex-best-year PF, paired upper 90 % bound > 0 (not demonstrably worse)
  quality-config : paired lower bound > 0 (Šidák for cells_scanned), PF >= max(1.4, baseline), >= 4 of 5 years positive,
                   trades >= 0.6 x baseline, no year < -300, ex-best-year PF
  stress-mode    : volume-config with trades >= 0.9 x baseline, plus on SPY < -1 % days: total P&L >= 3 x
                   baseline's, >= +40 per such day on average, no single day < -300; ordinary-day P&L
                   within +-10 % of baseline
  additive-config: for a candidate that leaves every baseline trade untouched and only adds a new window
                   (metrics['marginal'], from metrics.marginal_metrics()): the base subset must match the
                   baseline's P&L within +-2 % and trade count within +-1 % (base_unchanged); the added
                   subset must be net positive with PF >= 1.3, >= 15 trades over 5y, no year worse than
                   -100 and no single day worse than -300; the paired lower bound > 0 (the added half IS the
                   paired difference); ex-best-year PF not below the baseline's - 0.02; and no year of the *combined* book may be more
                   than 50 worse than the baseline's same year (combined_years_not_worse) — this is the
                   quality-config years_positive requirement's replacement for a candidate whose baseline
                   itself has flat/negative years an additive edge can never repair
  sizing-config  : for a candidate that only changes position sizing (no new entries/exits): same trade
                   set as the baseline (metrics['marginal'], via metrics.marginal_metrics()/split_marginal() -
                   trade count within +-2 % of baseline's and the matched ('base') subset >= 95 % of the
                   baseline's trade count, i.e. almost every baseline trade still fires); P&L >= baseline
                   x 1.10; PF >= baseline PF - 0.02; drawdown PER DOLLAR MADE (|dd| / P&L) no more than 10 % above the
                   baseline's (leverage alone raises P&L and drawdown together and must not pass on P&L alone);
                   paired lower bound > 0; ex-best-year PF not below the baseline's - 0.02; every year's P&L >= that year's baseline P&L - 50;
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
                   empty for this gate and is not checked). checks: added trades >= 80 (5y, was 40); added P&L > 0; paired lower bound > 0;
                   added PF >= 1.15; combined PF (baseline_rows + added_rows) >= baseline PF - 0.02 (not
                   dilutive); combined_years_not_worse: no year of the combined book more than 50 worse
                   than the baseline's same year; added worst year >= -150. bar-cache coverage of the
                   candidate's own ticker is enforced the same way as default-ticker, before the gate ever
                   runs (pipeline.py's kind-level coverage check, not a metrics check here). the live
                   3-position cap is reported as info (metrics.concurrency_exceedance() over the combined
                   set) and never gated — the replay runs each ticker set independently so a straight
                   concatenation is exact except for that cap
  standalone-strategy: for a standalone second strategy (its own book, own tickers, a different edge —
                   e.g. candidate #47 qqq-noise-pm-vol, QQQ only, afternoons, the promoted short windows
                   disabled) judged two ways at once, never against a baseline swept on the candidate's
                   own tickers (that baseline would be a different, barely-traded strategy on those
                   names — meaningless; see additive-ticker's rationale, which this gate reuses for the
                   baseline choice: pipeline.py's baseline_for() forces 'iex_v18' the same way).
                   needs metrics['standalone'] (metrics.standalone_metrics() against baseline_tag=
                   'iex_v18'): a daily P&L series for the candidate and the baseline over the union of
                   SPY sessions (zero-filled), their Pearson correlation, the combined (baseline +
                   candidate, concatenated trades — exact, each book replays independently) book's PF
                   and drawdown computed on the combined daily series, active-day counts, and the
                   candidate's PF with its single best year excluded (ex_best_year — guards against a
                   one-year wonder). checks: standalone_pf >= 1.4; standalone_years_positive >= 4 of 5;
                   standalone_trades >= 150 (5y); standalone_worst_year >= -300 (36 % sizing);
                   daily_corr_with_baseline <= 0.30; combined_pf >= baseline_pf (not dilutive);
                   combined_max_dd >= baseline_max_dd x 1.25 (no more than 25 % deeper); combined_active_days
                   >= baseline_active_days x 1.25; ex_best_year_pf >= 1.2. the live 3-position cap is
                   reported as info (metrics.concurrency_exceedance() over the combined set) and never
                   gated, same as additive-ticker
"""
from __future__ import annotations

import re

YEAR_FLOOR = -300.0
MIN_PF = 1.4                      # the owner's bar ("keep PF > 1.4"); was 1.3 until 2026-10-29
EX_BEST_YEAR_MIN_PF = 1.2         # PF with the best year removed — the record is 1.40 with 2022 and 1.19 without
EX_BEST_YEAR_TOLERANCE = 0.02     # relative gates: ex-best-year PF may not fall more than this below the baseline's
PF_TOLERANCE = 0.02
ADDED_CONFIG_MIN_PF = 1.3         # additive-config's added half: a sanity floor; the paired bootstrap is the real guard
DD_TOLERANCE = 1.25               # volume-config: drawdown no more than 25 % deeper than the baseline's
PER_TRADE_FACTOR = 0.85           # volume-config: $/trade at least 85 % of the baseline's (no edge erosion for volume)
PAIRED_ALPHA = 0.10               # one-sided 90 % bound on the paired day-bootstrap, Šidák-adjusted for cells_scanned
MIN_YEARS_POSITIVE = 4
STANDARD_MAX_POSITION_PCT = 0.36
MAX_POSITION_FRACTION_TOLERANCE = 1.02
ADDED_TICKER_MIN_TRADES = 80   # was 40: 'added PF >= 1.15 over 40 trades' admits 37 % of zero-edge names
ADDED_TICKER_MIN_PF = 1.15
ADDED_TICKER_WORST_YEAR_FLOOR = -150.0
COMBINED_PF_TOLERANCE = 0.02
COMBINED_YEAR_TOLERANCE = 50.0
STANDALONE_MIN_PF = 1.4
STANDALONE_MIN_TRADES = 150
STANDALONE_MAX_CORR = 0.30
STANDALONE_DD_TOLERANCE = 1.25
STANDALONE_ACTIVE_DAYS_FACTOR = 1.25
STANDALONE_EX_BEST_YEAR_MIN_PF = 1.2


def check(name: str, value, threshold: str, ok: bool) -> dict:
    if isinstance(value, float):
        value = round(value, 3)
    return {"name": name, "value": value, "threshold": threshold, "ok": bool(ok)}


def _result(gate: str, checks: list[dict]) -> dict:
    return {"gate": gate, "pass": all(c["ok"] for c in checks), "checks": checks}


def _minus_pct(x: float, pct: float) -> float:
    """x reduced by pct of its magnitude (so a negative baseline still gets a lower bound below it)."""
    return x - pct * abs(x)


def _exb(m: dict | None) -> tuple[float | None, str | None]:
    """(PF with the best year excluded, that year) from metrics['stats']['ex_best_year']."""
    e = ((m or {}).get("stats") or {}).get("ex_best_year") or {}
    pf = e.get("pf")
    return (None if pf is None else float(pf)), e.get("excluded_year")


def _ex_best_year_check(m: dict, baseline: dict | None = None) -> dict:
    """absolute bar (EX_BEST_YEAR_MIN_PF) when there is no baseline; relative (baseline's ex-best-year
    PF − tolerance) when the candidate is judged against one whose own ex-best-year PF is below the bar
    — an additive or sizing change cannot be asked to repair the baseline's 2023–2025."""
    pf, yr = _exb(m)
    if pf is None:
        return check("ex_best_year_pf", None, "stats.ex_best_year required (metrics.candidate_stats)", False)
    bpf, _ = _exb(baseline)
    if bpf is not None and bpf < EX_BEST_YEAR_MIN_PF:
        need = bpf - EX_BEST_YEAR_TOLERANCE
        return check("ex_best_year_pf", pf, f">= {need:.3f} (baseline {bpf:.3f} - {EX_BEST_YEAR_TOLERANCE:g}, excluding {yr})", pf >= need)
    return check("ex_best_year_pf", pf, f">= {EX_BEST_YEAR_MIN_PF} (excluding {yr})", pf >= EX_BEST_YEAR_MIN_PF)


def sidak_alpha(alpha: float, cells_scanned: int | None) -> float:
    """the per-test level that keeps a family-wise one-sided `alpha` over `cells_scanned` tries."""
    n = max(1, int(cells_scanned or 1))
    if n == 1:
        return alpha   # exact: 1 - (1 - a) ** 1 would return 0.0999... and miss the stored 0.10 level
    return 1.0 - (1.0 - alpha) ** (1.0 / n)


def _paired_quantile(pr: dict, p: float) -> tuple[float, float]:
    """(quantile level actually used, value): the largest stored level <= p (conservative)."""
    qs = sorted(pr.get("pnl_quantiles") or [])
    if not qs:
        return (p, float("nan"))
    chosen = qs[0]
    for lvl, val in qs:
        if lvl <= p:
            chosen = (lvl, val)
    return (float(chosen[0]), float(chosen[1]))


def _paired_checks(m: dict, cells_scanned: int | None, require_improvement: bool) -> list[dict]:
    """the paired day-bootstrap against the baseline (metrics['paired']). `require_improvement`: the
    lower one-sided bound of the P&L difference must be > 0 at the Šidák level for `cells_scanned`
    (an upgrade claim). otherwise the candidate must merely not be demonstrably worse: the upper 90 %
    bound of the difference must be > 0 (a volume claim trades PF headroom for trades, it does not
    promise more money). P(diff ≤ 0) is reported either way, never gated on its own."""
    pr = m.get("paired")
    if not pr or not pr.get("pnl_quantiles"):
        return [check("paired_bootstrap", None, "paired day-bootstrap vs the baseline required (metrics.paired_day_bootstrap)", False)]
    n_cells = max(1, int(cells_scanned or 1))
    out = [check("paired_p_le_0", pr.get("p_le_0"), f"info: P(candidate - baseline <= 0) over {pr.get('n_days')} days, paired; observed diff {pr.get('pnl_diff'):+.0f}", True)]
    if require_improvement:
        a = sidak_alpha(PAIRED_ALPHA, n_cells)
        lvl, lo = _paired_quantile(pr, a)
        out.append(check("paired_pnl_lower_bound", lo, f"> 0 at the {lvl:.2%} quantile (one-sided 90 %, Šidák for {n_cells} cell{'s' if n_cells > 1 else ''})", lo > 0))
    else:
        lvl, hi = _paired_quantile(pr, 1.0 - PAIRED_ALPHA)
        out.append(check("paired_pnl_not_worse", hi, f"> 0 at the {lvl:.0%} quantile (not demonstrably worse than the baseline)", hi > 0))
    return out


def _name_share_info(m: dict) -> dict:
    ns = m.get("name_share") or {}
    return check("name_share_info", ns.get("share"), f"info only: {ns.get('ticker')} carries this share of the P&L", True)


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
        _ex_best_year_check(m),
    ]
    return _result("default-ticker", checks)


def _need_baseline(gate: str, baseline: dict | None) -> dict | None:
    if not baseline:
        return _result(gate, [check("baseline", None, "baseline metrics required", False)])
    return None


def volume_config(m: dict, baseline: dict | None = None, gate: str = "volume-config", trade_factor: float = 1.0,
                  cells_scanned: int | None = None) -> dict:
    """more trades at the owner's PF: trades >= baseline; PF >= max(1.4, baseline − 0.02); drawdown no
    more than 25 % deeper; $/trade >= 85 % of the baseline's (volume must not come from edge erosion);
    ex-best-year PF; and the paired day-bootstrap must not show the candidate demonstrably worse."""
    err = _need_baseline(gate, baseline)
    if err:
        return err
    b = baseline
    need_trades = trade_factor * b["n"]
    need_pf = max(MIN_PF, b["pf"] - PF_TOLERANCE)
    pnl_floor = _minus_pct(b["pnl"], 0.10)
    dd_floor = b["dd"] * DD_TOLERANCE
    b_per_trade = (b["pnl"] / b["n"]) if b["n"] else 0.0
    per_trade = (m["pnl"] / m["n"]) if m["n"] else 0.0
    need_per_trade = _minus_pct(b_per_trade, 1.0 - PER_TRADE_FACTOR)   # 85 % of a positive baseline; below a negative one
    checks = [
        check("trades_5y", m["n"], f">= {need_trades:.0f} ({trade_factor:g} x baseline {b['n']})", m["n"] >= need_trades),
        check("pf_5y", m["pf"], f">= {need_pf:.3f} (max of {MIN_PF}, baseline {b['pf']:.3f} - {PF_TOLERANCE:g})", m["pf"] >= need_pf),
        *_common(m),
        check("pnl_5y", m["pnl"], f">= {pnl_floor:+.0f} (baseline {b['pnl']:+.0f} - 10 %)", m["pnl"] >= pnl_floor),
        check("max_drawdown", m["dd"], f">= {dd_floor:+.0f} (baseline {b['dd']:+.0f} x {DD_TOLERANCE:g}, no more than 25 % deeper)", m["dd"] >= dd_floor),
        check("pnl_per_trade", per_trade, f">= {need_per_trade:+.2f} (baseline {b_per_trade:+.2f} - {100 * (1 - PER_TRADE_FACTOR):.0f} %)", per_trade >= need_per_trade),
        _ex_best_year_check(m, b),
        *_paired_checks(m, cells_scanned, require_improvement=False),
        _name_share_info(m),
    ]
    return _result(gate, checks)


def quality_config(m: dict, baseline: dict | None = None, cells_scanned: int | None = None) -> dict:
    """an upgrade claim: the paired day-bootstrap lower bound of the P&L difference vs the baseline must
    be > 0 at the Šidák level for the cells the study scanned (the old "+0.05 PF" was 0.27 standard
    deviations of a 450-trade cell — noise); PF >= max(1.4, baseline); trades >= 0.6 x baseline;
    ex-best-year PF >= 1.2."""
    err = _need_baseline("quality-config", baseline)
    if err:
        return err
    b = baseline
    need_pf = max(MIN_PF, b["pf"])
    need_trades = 0.6 * b["n"]
    checks = [
        check("pf_5y", m["pf"], f">= {need_pf:.3f} (max of {MIN_PF}, baseline {b['pf']:.3f})", m["pf"] >= need_pf),
        *_common(m),
        check("trades_5y", m["n"], f">= {need_trades:.0f} (0.6 x baseline {b['n']})", m["n"] >= need_trades),
        _ex_best_year_check(m, b),
        *_paired_checks(m, cells_scanned, require_improvement=True),
        _name_share_info(m),
    ]
    return _result("quality-config", checks)


def stress_mode(m: dict, baseline: dict | None = None, cells_scanned: int | None = None) -> dict:
    base = volume_config(m, baseline, gate="stress-mode", trade_factor=0.9, cells_scanned=cells_scanned)
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


def additive_config(m: dict, baseline: dict | None = None, cells_scanned: int | None = None) -> dict:
    """for a candidate that only adds a new window on top of an unchanged baseline (plan: see module
    docstring). needs metrics['marginal'] (metrics.marginal_metrics()'s output: base vs added trades).
    since 2026-10-29 the added half must also clear the paired day-bootstrap (lower bound > 0 at the
    Šidák level): "added PF >= 1.3 over 15 trades" alone is met by a third of zero-edge windows."""
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
        check("added_pf", added["pf"], f">= {ADDED_CONFIG_MIN_PF}", added["pf"] >= ADDED_CONFIG_MIN_PF),
        check("added_trades", added["n"], ">= 15 (5y)", added["n"] >= 15),
        check("added_worst_year", added["worst_year_pnl"], ">= -100 (36 % sizing)", added["worst_year_pnl"] >= -100.0),
        check("added_worst_day", added["worst_day"], ">= -300", added["worst_day"] >= -300.0),
        check("combined_years_not_worse", year_margins[worst_margin_year],
              f">= -50 vs baseline per year (worst: {worst_margin_year})", year_margins[worst_margin_year] >= -50.0),
        _ex_best_year_check(m, b),
        *_paired_checks(m, cells_scanned, require_improvement=True),
        _name_share_info(m),
    ]
    return _result("additive-config", checks)


def additive_ticker(m: dict, baseline: dict | None = None, cells_scanned: int | None = None) -> dict:
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
        *_paired_checks(m, cells_scanned, require_improvement=True),
    ]
    return _result("additive-ticker", checks)


def standalone_strategy(m: dict, baseline: dict | None = None, cells_scanned: int | None = None) -> dict:
    """for a standalone second-strategy candidate (its own book, own tickers, often a different
    edge entirely) judged on its own 5y record AND on what it does to the live book, never against
    a baseline swept on the candidate's own tickers (module docstring; the baseline is always
    'iex_v18', forced by pipeline.py's baseline_for() the same way additive_ticker()'s is). needs
    metrics['standalone'] (metrics.standalone_metrics() against baseline_tag='iex_v18')."""
    err = _need_baseline("standalone-strategy", baseline)
    if err:
        return err
    b = baseline
    sa = m.get("standalone")
    if not sa:
        return _result("standalone-strategy", [check(
            "standalone", None,
            "standalone metrics required (metrics.standalone_metrics against baseline_tag='iex_v18')", False)])
    combined = sa["combined"]
    dd_floor = b["dd"] * STANDALONE_DD_TOLERANCE
    need_active = STANDALONE_ACTIVE_DAYS_FACTOR * sa["baseline_active_days"]
    exb = sa["ex_best_year"]
    conc = sa.get("concurrency") or {}
    checks = [
        check("standalone_pf", m["pf"], f">= {STANDALONE_MIN_PF}", m["pf"] >= STANDALONE_MIN_PF),
        check("standalone_years_positive", m["years_positive"], f">= {MIN_YEARS_POSITIVE} of 5", m["years_positive"] >= MIN_YEARS_POSITIVE),
        check("standalone_trades", m["n"], f">= {STANDALONE_MIN_TRADES} (5y)", m["n"] >= STANDALONE_MIN_TRADES),
        check("standalone_worst_year", m["min_year_pnl"], f">= {YEAR_FLOOR:+.0f} (36 % sizing)", m["min_year_pnl"] >= YEAR_FLOOR),
        check("daily_corr_with_baseline", sa["daily_corr"], f"<= {STANDALONE_MAX_CORR}", sa["daily_corr"] <= STANDALONE_MAX_CORR),
        check("combined_pf", combined["pf"], f">= {b['pf']:.3f} (baseline {sa['baseline_tag']}, not dilutive)", combined["pf"] >= b["pf"]),
        check("combined_max_dd", combined["dd"], f">= {dd_floor:+.0f} (baseline {b['dd']:+.0f} x {STANDALONE_DD_TOLERANCE:g}, no more than 25 % deeper)", combined["dd"] >= dd_floor),
        check("combined_active_days", sa["combined_active_days"], f">= {need_active:.0f} ({STANDALONE_ACTIVE_DAYS_FACTOR:g} x baseline {sa['baseline_active_days']})", sa["combined_active_days"] >= need_active),
        check("ex_best_year_pf", exb["pf"], f">= {STANDALONE_EX_BEST_YEAR_MIN_PF} (excluding {exb.get('excluded_year')}, {exb.get('excluded_year_pnl'):+.0f})", exb["pf"] >= STANDALONE_EX_BEST_YEAR_MIN_PF),
        check("concurrency_info", conc.get("n_exceedance_days", 0),
              f"info only, not gated (cap {conc.get('cap', 3)}; max concurrent observed {conc.get('max_concurrent_observed', 0)})", True),
        _name_share_info(m),
    ]
    return _result("standalone-strategy", checks)


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


def sizing_config(m: dict, baseline: dict | None = None, sweep_args_override: str | None = None,
                  cells_scanned: int | None = None) -> dict:
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
    # leverage alone raises P&L and drawdown together: judge the drawdown PER DOLLAR MADE, which a
    # pure size-up leaves unchanged and a smarter allocation improves (was: dd no deeper than x1.3)
    b_eff = (abs(b["dd"]) / b["pnl"]) if b["pnl"] > 0 else float("inf")
    eff = (abs(m["dd"]) / m["pnl"]) if m["pnl"] > 0 else float("inf")
    eff_ceiling = b_eff * 1.10
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
        check("drawdown_per_dollar", eff, f"<= {eff_ceiling:.3f} (baseline |dd|/P&L {b_eff:.3f} x 1.10: size-up may not cost more drawdown per dollar made)", eff <= eff_ceiling),
        check("years_not_worse", year_margins[worst_margin_year],
              f">= -50 vs baseline per year (worst: {worst_margin_year})", year_margins[worst_margin_year] >= -50.0),
        check("max_position_fraction", max_frac,
              f"<= {cap_threshold:.4f} (cap {cap:g}{' from _sweep_args' if sweep_args_override else ' standard'} x 1.02; size x entry_price / 10,000 capital)",
              max_frac <= cap_threshold),
        _ex_best_year_check(m, b),
        *_paired_checks(m, cells_scanned, require_improvement=True),
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
    "standalone-strategy": standalone_strategy,
}
ALIASES = {"default-config": "volume-config"}  # "default" itself is resolved by kind in resolve()
NEEDS_BASELINE = {"volume-config", "quality-config", "stress-mode", "additive-config", "sizing-config", "additive-ticker", "standalone-strategy"}
NEEDS_MARGINAL = {"additive-config", "sizing-config", "additive-ticker"}
NEEDS_STANDALONE = {"standalone-strategy"}


def resolve(name: str, kind: str) -> str:
    """map a requested gate name (or 'default') to a stable gate name for the candidate kind."""
    if name in GATES:
        return name
    if name == "default" or not name:
        return "default-ticker" if kind == "ticker" else "volume-config"
    if name in ALIASES and ALIASES[name]:
        return ALIASES[name]
    raise KeyError(f"unknown gate '{name}' (known: {', '.join(GATES)})")


def run_gate(name: str, metrics: dict, baseline: dict | None = None, sweep_args_override: str | None = None,
             cells_scanned: int | None = None) -> dict:
    """sweep_args_override is only used by sizing-config (its cap check). `cells_scanned` (the
    candidate's provenance) sets the Šidák level of the paired-bootstrap checks; default-ticker has none."""
    if name == "sizing-config":
        return sizing_config(metrics, baseline, sweep_args_override=sweep_args_override, cells_scanned=cells_scanned)
    if name == "default-ticker":
        return default_ticker(metrics, baseline)
    return GATES[name](metrics, baseline, cells_scanned=cells_scanned)


def failed_names(result: dict) -> list[str]:
    return [c["name"] for c in result.get("checks", []) if not c["ok"]]
