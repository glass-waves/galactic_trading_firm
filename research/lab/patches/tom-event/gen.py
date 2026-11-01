#!/usr/bin/env python3
"""tom-event desk: generate calendar-event patches + specs (research/lab/desks/tom-event.md).

the calendar is the k-th NYSE session of each month (k<0: counted from the month end, -1 = last session),
taken from data/bars_iex/SPY.csv sessions (the lab's own session list) with a weekday-minus-holidays
fallback after the cache ends (through 2026-12). dates are calendar facts, not returns.
    python3 research/lab/patches/tom-event/gen.py      # (re)writes every cell below
"""
import datetime as dt, json, zoneinfo
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
PD = ROOT / "research/lab/patches/tom-event"
SD = ROOT / "research/lab/specs/tom-event"
ET = zoneinfo.ZoneInfo("America/New_York")
END = dt.date(2026, 12, 31)
FUTURE_HOLIDAYS = {dt.date(2026, 11, 26), dt.date(2026, 12, 25)}

IDX3 = ["SPY", "QQQ", "IWM"]
ETF4 = ["XLK", "XLF", "XLE", "SMH"]
MEGA4 = ["AAPL", "AMZN", "MSFT", "NVDA"]
POOL34 = ("AAPL ABBV ADBE AMAT AMD AMZN AVGO CAT COIN COST CRM CVX GS HD JPM LLY MA META MRK MSFT MU NFLX NVDA "
          "ORCL PG PLTR QCOM SHOP TSLA TXN UBER UNH V XOM").split()
ALL41 = IDX3 + ETF4 + POOL34


def sessions():
    with open(ROOT / "data/bars_iex/SPY.csv") as f:
        next(f)
        days = sorted({dt.datetime.fromtimestamp(int(l.split(",", 1)[0]), ET).date() for l in f})
    d = days[-1] + dt.timedelta(days=1)
    while d <= END:
        if d.weekday() < 5 and d not in FUTURE_HOLIDAYS:
            days.append(d)
        d += dt.timedelta(days=1)
    return days


def kth(k):
    by = defaultdict(list)
    for d in sessions():
        by[(d.year, d.month)].append(d)
    out = []
    for _, v in sorted(by.items()):
        if (k > 0 and len(v) >= k) or (k < 0 and len(v) >= -k):
            out.append(v[k - 1] if k > 0 else v[k])
    return [d.isoformat() for d in out]


def patch(k, entry=("09:30", "09:35"), exit_="15:55", stop=0.025, trail=None, gap=None, n_conc=3, cap_pct=0.95):
    disable = ["window_5m_thrust_short", "window_strong_core_short", "reject_1m_noise", "reject_1m_noise_short",
               "breakeven"]
    if trail is None:
        disable.append("trailing_stop_atr")
    inds = [{"instance_id": "tom_cal", "indicator_type": "event_calendar", "timescale": "OneMinute", "weight": 0.0,
             "enabled": True, "params": {"dates": kth(k)}}]
    conds = [{"type": "indicator_max", "instance_id": "tom_cal", "max_score": 0.0}]
    if gap is not None:
        inds.append({"instance_id": "gap_1m", "indicator_type": "gap", "timescale": "OneMinute", "weight": 0.0,
                     "enabled": True, "params": {"scale_pct": 1.0}})
        op, v = gap
        conds.append({"type": "indicator_max" if op == "le" else "indicator_min", "instance_id": "gap_1m.gap_pct",
                      ("max_score" if op == "le" else "min_score"): v})
    acts = [
        {"instance_id": "window_tom_long", "action_type": "entry_window", "phase": "Entry", "priority": 5,
         "enabled": True,
         "params": {"name": "tom long", "direction": "long", "conditions": conds, "entry_after": entry[0],
                    "entry_before": entry[1],
                    "exit_overrides": {"force_exit_by": exit_, "max_hold_ms": 23400000, "score_exit_threshold": -2.0}}},
        {"instance_id": "hard_stop", "action_type": "fixed_pct_stop", "phase": "Exit", "priority": 1, "enabled": True,
         "params": {"stop_loss_pct": stop}},
        {"instance_id": "exit_session", "action_type": "session_close", "phase": "Exit", "priority": 0,
         "enabled": True, "params": {"force_exit_by": exit_}},
    ]
    if trail is not None:
        acts.append({"instance_id": "trailing_stop_atr", "action_type": "atr_trailing_stop", "phase": "Exit",
                     "priority": 2, "enabled": True,
                     "params": {"timescale": "FiveMinute", "atr_period": 14, "multiplier": trail}})
    return {"disable": disable, "indicators": inds, "actions": acts,
            "session": {"no_new_entries_after": entry[1], "force_exit_by": exit_,
                        "max_concurrent_positions": n_conc, "max_capital_deployed_pct": cap_pct}}


def write(name, p, tickers, note, capital=10000, frac=None):
    n = len(tickers)
    if frac is None:
        frac = round(min(0.45, 0.90 / n), 4)
    (PD / f"{name}.json").write_text(json.dumps(p, indent=1) + "\n")
    spec = {"name": f"tom-event/{name}", "config_id": 12, "patches": [f"research/lab/patches/tom-event/{name}.json"],
            "args": [], "tickers": ",".join(tickers), "capital": capital,
            "sizing": ["--sizing-fraction", str(frac), "--max-position-pct", str(frac), "--cross-index", "SPY"],
            "note": note}
    (SD / f"{name}.json").write_text(json.dumps(spec, indent=1) + "\n")


# ---- screen: every timing / calendar variant on all 41 names at once (capital 100k, 2 % per name, so every name
#      gets whole shares; subsets are read from the trades file in bps and counted as cells in the report).
SCREEN = {
    "s_k2_open_close":   dict(k=2, note="k=2 enter 09:30 exit 15:55 stop 2.5 % (the pretest rule)"),
    "s_k2_open_close_nostop": dict(k=2, stop=0.20, note="k=2 open->close, no hard stop (20 %)"),
    "s_k2_gapdown":      dict(k=2, gap=("le", 0.0), note="k=2 open->close, only when today's gap <= 0"),
    "s_k1_open_close":   dict(k=1, note="k=1 (first session)"),
    "s_k3_open_close":   dict(k=3, note="k=3"),
    "s_klast_open_close": dict(k=-1, note="last session of the month"),
    "s_k5_ctrl":         dict(k=5, note="control: k=5 shifted calendar, identical window"),
    "s_k10_ctrl":        dict(k=10, note="control: k=10 shifted calendar, identical window"),
}

# ---- standalone book specs (10k book, 0.90 of capital split over the names, max_concurrent = number of names,
#      so the live 3-position cap never binds and the lab's independent-ticker replay matches what the book would do).
BOOKS = {
    "b_idx3_k2": (IDX3, dict(k=2), "standalone: SPY/QQQ/IWM long open->15:55 on session 2, 0.30 each, stop 2.5 %"),
    "b_pool34_k2": (POOL34, dict(k=2), "standalone: 34-name pool long open->15:55 on session 2, 100k book 0.0265 each (scale /10 for 10k)"),
}

if __name__ == "__main__":
    for name, (tk, c, note) in BOOKS.items():
        big = len(tk) > 10
        write(name, patch(n_conc=len(tk), **c), tk, note, capital=100000 if big else 10000)
    for name, c in SCREEN.items():
        c = dict(c)
        note = c.pop("note")
        write(name, patch(n_conc=41, **c), ALL41, "screen, 41 names, 2 %/name on 100k: " + note,
              capital=100000, frac=0.02)
    print(f"{len(SCREEN)} screen cells; k=2 dates: {len(kth(2))}")
