#!/usr/bin/env python3
"""emit the index intraday-momentum study's --patch-json files (research/index_momentum/<cell>.json).

every cell is a STANDALONE book: v18's two promoted short windows are disabled, as are the parts
of v18's exit/entry stack that would change a momentum trade (breakeven monitor, 5m ATR x7 trail,
the two 1m-noise reject gates); the 2.5 % hard stop and the session's daily-loss breaker stay as
catastrophe guards. the session is opened with the `session` key (entries to 15:30, flat 15:58),
and `tickers` is set. one weight-0 `noise_area` instance (`noise`) feeds the windows.

noise-area breakout (Zarattini, Aziz & Barbon 2024): at each decision bar (`noise.decision`, every
30 min from the 09:59 bar = 10:00 price), long if price >= upper boundary and above VWAP, short if
<= lower and below VWAP (the paper's trailing stop is max(UB, VWAP) / min(LB, VWAP), so an entry
on the wrong side of VWAP would be stopped at once). exit: `vwap_stop` (price back through session
VWAP, checked every bar) or 15:58. per-window exit_overrides: no score exit, 7 h max hold.

Gao last half hour (Gao, Han, Li & Zhou 2018): at the 15:29 bar, long if the 09:30-10:00 return
>= 0 else short; hold to 15:58.

usage: python3 research/index_momentum/make_patches.py [cell ...]          (no args: every cell)
       python3 research/index_momentum/make_patches.py --pipeline CELL OUT.json
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PROMOTED_SHORTS = ["window_5m_thrust_short", "window_strong_core_short"]
V18_STACK_OFF = ["reject_1m_noise", "reject_1m_noise_short", "breakeven", "trailing_stop_atr"]
SESSION = {"no_new_entries_after": "15:30", "force_exit_by": "15:58"}
MAX_HOLD = 25_200_000          # 7 h: longer than any session, so only the stops / clock end a trade
NO_SCORE_EXIT = -1e9           # long: composite <= -1e9 never; short: composite >= 1e9 never
EXIT_OV = {"force_exit_by": "15:58", "max_hold_ms": MAX_HOLD, "score_exit_threshold": NO_SCORE_EXIT}


def noise(lookback=14, band=1.0, every=30, tol=2, gap=True, min_sessions=None):
    p = {"lookback_sessions": lookback, "band_mult": band, "decision_every_min": every,
         "decision_tolerance_min": tol, "gap_adjust": gap}
    if min_sessions:
        p["min_sessions"] = min_sessions
    return {"indicator_type": "noise_area", "instance_id": "noise", "timescale": "OneMinute",
            "weight": 0.0, "enabled": True, "params": p}


def cmin(key, x):
    return {"type": "indicator_min", "instance_id": f"noise.{key}", "min_score": x}


def cmax(key, x):
    return {"type": "indicator_max", "instance_id": f"noise.{key}", "max_score": x}


def window(iid, name, direction, prio, conds, after, before):
    return {"instance_id": iid, "action_type": "entry_window", "phase": "Entry", "priority": prio,
            "enabled": True, "params": {"name": name, "direction": direction, "conditions": conds,
                                        "entry_after": after, "entry_before": before,
                                        "exit_overrides": copy.deepcopy(EXIT_OV)}}


def vwap_stop(buf=0.0):
    return {"instance_id": "vwap_stop_im", "action_type": "vwap_stop", "phase": "Exit", "priority": 2,
            "enabled": True, "params": {"buffer_pct": buf, "scope_min_max_hold_ms": MAX_HOLD}}


def na_windows(after="09:59", before="15:30", vwap_buf=0.0, extra=(), long_=True, short=True):
    """noise-area breakout pair. `extra`: conditions added to both (e.g. a vol filter)."""
    w = []
    if long_:
        w.append(window("window_im_long", "im long", "long", 30,
                        [cmin("decision", 0.5), cmin("pos", 1.0), cmin("vwap_pct", vwap_buf)] + list(extra),
                        after, before))
    if short:
        w.append(window("window_im_short", "im short", "short", 31,
                        [cmin("decision", 0.5), cmax("pos", -1.0), cmax("vwap_pct", -vwap_buf)] + list(extra),
                        after, before))
    return w


def gao_windows(agree=False):
    lc, sc = [cmin("ret_first30_pct", 0.0)], [cmax("ret_first30_pct", -1e-9)]
    if agree:   # 15:00-15:30 return must have the same sign
        lc.append(cmin("ret_hour_pct", 0.0))
        sc.append(cmax("ret_hour_pct", -1e-9))
    return [window("window_gao_long", "gao long", "long", 40, lc, "15:29", "15:30"),
            window("window_gao_short", "gao short", "short", 41, sc, "15:29", "15:30")]


def cell(tickers, actions, ind=None, sizing=None):
    acts = list(actions)
    disable = PROMOTED_SHORTS + V18_STACK_OFF
    if sizing:
        acts.append(sizing)
        disable = disable + ["sizing_fixed"]
    return {"disable": disable, "indicators": [ind or noise()], "actions": acts,
            "session": dict(SESSION), "tickers": list(tickers)}


def na(tickers, ind=None, vwap_buf=0.0, stop_buf=None, stop=True, **kw):
    """`vwap_buf`: entry must clear VWAP by this %; `stop_buf`: the stop's buffer (default = vwap_buf);
    stop=False: no VWAP stop (hold to 15:58 or the 2.5 % hard stop; the paper's base version)."""
    acts = na_windows(vwap_buf=vwap_buf, **kw)
    if stop:
        acts.append(vwap_stop(vwap_buf if stop_buf is None else stop_buf))
    return cell(tickers, acts, ind)


def vol_target(base=0.36, target_pct=None):
    """size = base x min(1, target / day_move): tiers on noise.day_move_pct (approximation)."""
    tiers = [{"min": 0.0, "mult": 1.0}]
    for dm in (0.9, 1.1, 1.4, 1.8):
        tiers.append({"min": dm, "mult": round(min(1.0, target_pct / dm), 3)})
    return {"instance_id": "sizing_voltarget", "action_type": "indicator_tiered", "phase": "Sizing",
            "priority": 0, "enabled": True,
            "params": {"instance_id": "noise.day_move_pct", "base_fraction": base, "tiers": tiers,
                       "fallback_mult": 1.0}}


SPY, QQQ = ["SPY"], ["QQQ"]
CELLS = {
    # base: 14 sessions, band x1.0, decisions every 30 min from 10:00, VWAP stop (no buffer)
    "na_spy": lambda: na(SPY),
    "na_qqq": lambda: na(QQQ),
    # decision cadence and stop buffer
    "na_spy_h60": lambda: na(SPY, ind=noise(every=60)),
    "na_spy_b10": lambda: na(SPY, vwap_buf=0.10),
    "na_spy_s25": lambda: na(SPY, stop_buf=0.25),     # entry on the right side of VWAP, stop 0.25 % through it
    "na_spy_nostop": lambda: na(SPY, stop=False),
    # best region of the sim scan (sim.py, 72 cells): decisions from 12:00, no VWAP stop (hold to 15:58)
    "na_qqq_pm_hold": lambda: na(QQQ, after="11:59", stop=False),
    "na_spy_pm_hold": lambda: na(SPY, after="11:59", stop=False),
    # day-level vol filter as a real window condition (checks analyze.py's derived dm filter)
    "na_qqq_dm07": lambda: na(QQQ, extra=[cmin("day_move_pct", 0.7)]),
    # the candidate: QQQ, decisions 12:00-15:30, hold to 15:58, days with a 14-session avg open-to-close
    # move >= 0.7 % (plateau 0.7-1.0, LOYO); lb4 = 4-session noise area that fits the 8-day warmup
    "na_qqq_pm_hold_dm07": lambda: na(QQQ, after="11:59", stop=False, extra=[cmin("day_move_pct", 0.7)]),
    "na_qqq_pm_hold_dm07_lb4": lambda: na(QQQ, ind=noise(lookback=4), after="11:59", stop=False,
                                          extra=[cmin("day_move_pct", 0.7)]),
    # Gao first half hour -> last half hour
    "gao_spy": lambda: cell(SPY, gao_windows()),
    # QQQ and "15:00-15:30 sign agrees" variants: sim only (report §4); `gao_windows(agree=True)` builds them
}


def pipeline_patch(cell_name, out_file):
    """the cell as a pipeline patch. a 14-session noise area needs ~20 trading days of warmup, so the
    gate sweep gets `--lookback-days 22` via `_sweep_args` (the last occurrence of a flag wins)."""
    p = CELLS[cell_name]()
    if p["indicators"][0]["params"]["lookback_sessions"] > 4:
        p["_sweep_args"] = "--lookback-days 22"
    with open(out_file, "w") as fh:
        json.dump(p, fh, indent=1)
        fh.write("\n")


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "--pipeline":
        pipeline_patch(sys.argv[2], sys.argv[3])
        print(sys.argv[3])
        return
    names = sys.argv[1:] or list(CELLS)
    for k in names:
        with open(os.path.join(HERE, f"{k}.json"), "w") as fh:
            json.dump(CELLS[k](), fh, indent=1)
            fh.write("\n")
    print(" ".join(names))


if __name__ == "__main__":
    main()
