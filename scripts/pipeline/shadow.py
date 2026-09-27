"""shadow-trial clock, parity and edge rules as pure functions (plan §4.2.3 as amended by §9).

sessions come from `book_sessions` (days the trader actually hosted the book); a book that is never
hosted never times out — it is reported as "not hosted" instead.
"""
from __future__ import annotations

import datetime as dt

MAX_SESSIONS = 60           # evaluate regardless of trade count after this many hosted sessions
NOT_HOSTED_GRACE_DAYS = 3   # trading days after shadow_started_at with no book_sessions row -> flag
PARITY_COUNT_TOL = 0.30     # trade count within +-30 %
PARITY_BPS_TOL = 10.0       # mean P&L per trade within 10 bps of price
EDGE_MIN_PF = 1.0
EDGE_BPS_TOL = 10.0         # shadow P&L per trade not below the replay's by more than 10 bps
EDGE_PRIMARY_PCT = 0.20     # config kind: not below the primary's P&L by more than 20 %


def due(n_sessions: int, n_trades: int, min_sessions: int, min_trades: int, max_sessions: int = MAX_SESSIONS) -> tuple[bool, str]:
    """(is the evaluation due, why)."""
    if n_sessions >= max_sessions:
        return True, f"{n_sessions} sessions >= {max_sessions} (time limit)"
    if n_sessions >= min_sessions and n_trades >= min_trades:
        return True, f"{n_sessions} sessions >= {min_sessions} and {n_trades} trades >= {min_trades}"
    need = []
    if n_sessions < min_sessions:
        need.append(f"{min_sessions - n_sessions} more sessions")
    if n_trades < min_trades:
        need.append(f"{min_trades - n_trades} more trades")
    return False, "needs " + " and ".join(need) + f" (or {max_sessions - n_sessions} sessions to the time limit)"


def trading_days_between(start: dt.date, end: dt.date, calendar: list[dt.date] | None = None) -> int:
    """trading days strictly after `start` up to and including `end`. `calendar` = the primary's
    hosted session dates when known (exact); weekdays otherwise (approximate: holidays count)."""
    if calendar:
        return sum(1 for d in calendar if start < d <= end)
    n, d = 0, start + dt.timedelta(days=1)
    while d <= end:
        if d.weekday() < 5:
            n += 1
        d += dt.timedelta(days=1)
    return n


def not_hosted(started: dt.date, sessions: list[dt.date], today: dt.date, calendar: list[dt.date] | None = None,
               grace: int = NOT_HOSTED_GRACE_DAYS) -> tuple[bool, int]:
    """(flag, trading days elapsed). flagged when the book has no hosted session after `grace`
    trading days — reported as plumbing, never failed."""
    elapsed = trading_days_between(started, today, calendar)
    return (not sessions and elapsed >= grace), elapsed


def bps(pnl: float, entry_price: float, size: float) -> float | None:
    notional = abs(entry_price * size)
    if notional <= 0:
        return None
    return pnl / notional * 1e4


def mean_bps(trades: list[dict]) -> float | None:
    vals = [b for b in (bps(t["pnl"], t["entry_price"], t["size"]) for t in trades) if b is not None]
    return sum(vals) / len(vals) if vals else None


def parity(shadow_trades: list[dict], replay_trades: list[dict]) -> dict:
    """trades are dicts with pnl, entry_price, size. count within +-30 % of the replay's and mean
    bps within 10 bps. with no trades on either side the comparison is vacuous and passes."""
    ns, nr = len(shadow_trades), len(replay_trades)
    if nr == 0 and ns == 0:
        count_ok, count_detail = True, "no trades on either side"
    elif nr == 0:
        count_ok, count_detail = ns <= 1, f"replay 0 trades, shadow {ns}"
    else:
        rel = abs(ns - nr) / nr
        count_ok, count_detail = rel <= PARITY_COUNT_TOL, f"shadow {ns} vs replay {nr} ({rel * 100:.0f} % apart)"
    sb, rb = mean_bps(shadow_trades), mean_bps(replay_trades)
    if sb is None or rb is None:
        bps_ok, bps_detail, diff = True, "n/a (no trades on one side)", None
    else:
        diff = sb - rb
        bps_ok, bps_detail = abs(diff) <= PARITY_BPS_TOL, f"shadow {sb:+.1f} bps vs replay {rb:+.1f} bps"
    return {
        "ok": count_ok and bps_ok,
        "shadow_trades": ns, "replay_trades": nr, "count_ok": count_ok, "count_detail": count_detail,
        "shadow_bps": None if sb is None else round(sb, 2), "replay_bps": None if rb is None else round(rb, 2),
        "bps_diff": None if diff is None else round(diff, 2), "bps_ok": bps_ok, "bps_detail": bps_detail,
    }


def profit_factor(trades: list[dict]) -> float:
    gw = sum(t["pnl"] for t in trades if t["pnl"] > 0)
    gl = -sum(t["pnl"] for t in trades if t["pnl"] < 0)
    if gl <= 0:
        return 99.99 if gw > 0 else 0.0
    return gw / gl


def edge(kind: str, shadow_trades: list[dict], replay_trades: list[dict], primary_pnl: float | None = None) -> dict:
    """the gate-named default edge rule. returns {pass, checks:[...]}."""
    checks = []
    pf = profit_factor(shadow_trades)
    checks.append({"name": "shadow_pf", "value": round(pf, 3), "threshold": f">= {EDGE_MIN_PF}", "ok": pf >= EDGE_MIN_PF})
    sb, rb = mean_bps(shadow_trades), mean_bps(replay_trades)
    if sb is not None and rb is not None:
        checks.append({"name": "shadow_bps_vs_replay", "value": round(sb - rb, 2), "threshold": f">= -{EDGE_BPS_TOL:.0f} bps", "ok": sb - rb >= -EDGE_BPS_TOL})
    else:
        checks.append({"name": "shadow_bps_vs_replay", "value": None, "threshold": "n/a (no replay trades)", "ok": True})
    if kind == "config" and primary_pnl is not None:
        spnl = sum(t["pnl"] for t in shadow_trades)
        floor = primary_pnl - EDGE_PRIMARY_PCT * abs(primary_pnl)
        checks.append({"name": "shadow_pnl_vs_primary", "value": round(spnl, 2), "threshold": f">= {floor:+.0f} (primary {primary_pnl:+.0f} - 20 %)", "ok": spnl >= floor})
    return {"pass": all(c["ok"] for c in checks), "checks": checks}
