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
"""
from __future__ import annotations

YEAR_FLOOR = -300.0
MIN_PF = 1.3
MIN_YEARS_POSITIVE = 4


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


GATES = {
    "default-ticker": default_ticker,
    "volume-config": volume_config,
    "quality-config": quality_config,
    "stress-mode": stress_mode,
}
ALIASES = {"default-config": "volume-config", "default": None}
NEEDS_BASELINE = {"volume-config", "quality-config", "stress-mode"}


def resolve(name: str, kind: str) -> str:
    """map a requested gate name (or 'default') to a stable gate name for the candidate kind."""
    if name in GATES:
        return name
    if name == "default" or not name:
        return "default-ticker" if kind == "ticker" else "volume-config"
    if name in ALIASES and ALIASES[name]:
        return ALIASES[name]
    raise KeyError(f"unknown gate '{name}' (known: {', '.join(GATES)})")


def run_gate(name: str, metrics: dict, baseline: dict | None = None) -> dict:
    return GATES[name](metrics, baseline)


def failed_names(result: dict) -> list[str]:
    return [c["name"] for c in result.get("checks", []) if not c["ok"]]
