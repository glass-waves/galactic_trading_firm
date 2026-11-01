#!/usr/bin/env python3
"""signals-new desk: generate every patch + spec of the desk (re-run to reproduce).
new entry windows added alongside v18 (config row 12). see research/lab/desks/signals-new.md."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
P = ROOT / "research/lab/patches/signals-new"
S = ROOT / "research/lab/specs/signals-new"
FOUR = "AMZN,AAPL,NVDA,MSFT"
POOL34 = ("AAPL,ABBV,ADBE,AMAT,AMD,AMZN,AVGO,CAT,COIN,COST,CRM,CVX,GS,HD,JPM,LLY,MA,META,MRK,MSFT,MU,NFLX,"
          "NVDA,ORCL,PG,PLTR,QCOM,SHOP,TSLA,TXN,UBER,UNH,V,XOM")

BAND = [{"type": "indicator_min", "min_score": -0.4, "instance_id": "cross_1m"},
        {"type": "indicator_max", "max_score": 0.4, "instance_id": "cross_1m"}]
VPIN = [{"type": "indicator_min", "min_score": 0.217, "instance_id": "vpin_1m.raw_vpin"}]
THRUST = [{"type": "composite_max", "max_score": -0.35},
          {"type": "timescale_lag", "lag_by": 0.1, "timescale": "FiveMinute"},
          {"type": "timescale_max", "max_score": -0.5, "timescale": "FiveMinute"},
          {"type": "timescale_max", "max_score": 0.0, "timescale": "OneHour"}]
CORE = [{"type": "composite_max", "max_score": -0.35},
        {"type": "timescale_max", "max_score": -0.4, "timescale": "FiveMinute"},
        {"type": "timescale_max", "max_score": -0.4, "timescale": "OneHour"}]


def c_min(iid, v): return {"type": "indicator_min", "min_score": v, "instance_id": iid}
def c_max(iid, v): return {"type": "indicator_max", "max_score": v, "instance_id": iid}
def ts_max(ts, v): return {"type": "timescale_max", "max_score": v, "timescale": ts}
def comp_max(v): return {"type": "composite_max", "max_score": v}


def window(iid, name, direction, conds, prio=30, after=None, before="11:30", exit_overrides=None):
    p = {"name": name, "direction": direction, "conditions": conds}
    if after: p["entry_after"] = after
    if before: p["entry_before"] = before
    if exit_overrides: p["exit_overrides"] = exit_overrides
    return {"instance_id": iid, "action_type": "entry_window", "phase": "Entry", "priority": prio,
            "enabled": True, "params": p}


def ind(iid, typ, ts, **params):
    return {"indicator_type": typ, "instance_id": iid, "timescale": ts, "weight": 0.0, "enabled": True,
            "params": params}


# scaffold: v18's two windows with their own clock so the session can be opened for an afternoon window
AM = [window("window_5m_thrust_short_am", "5m thrust short", "short", THRUST + BAND + VPIN, prio=11,
             before="11:30", exit_overrides={"force_exit_by": "11:55"}),
      window("window_strong_core_short_am", "strong core short", "short", CORE + BAND + VPIN, prio=21,
             before="11:30", exit_overrides={"force_exit_by": "11:55"})]
OPEN_SESSION = {"no_new_entries_after": "15:30", "force_exit_by": "15:55"}
DISABLE = ["window_5m_thrust_short", "window_strong_core_short"]

cells = {}


def cell(name, patch, note, tickers=FOUR):
    cells[name] = (patch, note, tickers)


cell("scaffold", {"disable": DISABLE, "actions": AM, "session": OPEN_SESSION},
     "v18 windows re-clocked (entry_before 11:30, force 11:55) with the session opened to 15:30/15:55; must equal base/v18")


# ---------------- F1 afternoon VWAP-stretch fade (pretest c14): enter on the 12:30 bar, fixed hold
def pm_windows(x, hold=60, prio=-5):
    end_min = 12 * 60 + 30 + hold
    ov = {"force_exit_by": f"{end_min // 60}:{end_min % 60:02d}", "score_exit_threshold": -10.0,
          "max_hold_ms": hold * 60000}
    return [window("pm_fade_short", "pm vwap fade short", "short", [c_min("vwap_dist_1hr.dist_pct", x)],
                   prio=prio, after="12:30", before="12:31", exit_overrides=ov),
            window("pm_fade_long", "pm vwap fade long", "long", [c_max("vwap_dist_1hr.dist_pct", -x)],
                   prio=prio, after="12:30", before="12:31", exit_overrides=ov)]


for x in (1.0, 1.5):
    cell(f"pm_x{int(x*10):02d}_four", {"disable": DISABLE, "actions": AM + pm_windows(x), "session": OPEN_SESSION},
         f"scaffold + afternoon VWAP fade, |dist| >= {x}% at 12:30, hold 60, both sides, 4 home names (combined book)")
for x in (1.0, 1.5, 2.0):
    cell(f"pm_x{int(x*10):02d}_pool", {"disable": DISABLE, "actions": pm_windows(x), "session": OPEN_SESSION},
         f"fade-only (v18 windows off): |dist| >= {x}% at 12:30, hold 60, both sides, 34-name pool (added population)",
         tickers=POOL34)
cell("pm_x15_h45_pool", {"disable": DISABLE, "actions": pm_windows(1.5, hold=45), "session": OPEN_SESSION},
     "fade-only, |dist| >= 1.5%, hold 45 (pretest best hold), 34-name pool", tickers=POOL34)
cell("pm_x15_gated_pool", {"disable": DISABLE, "actions": pm_windows(1.5, prio=30), "session": OPEN_SESSION},
     "fade-only, |dist| >= 1.5%, hold 60, priority 30 (behind v18's 1m-noise reject gates), 34-name pool",
     tickers=POOL34)

# ---------------- F2 gap-fade short at the open (name gapped up, SPY flat/down, thrust shape)
GAP_IND = [ind("gap_1m", "gap", "OneMinute", scale_pct=1.0)]
LOOSE = [ts_max("FiveMinute", -0.3), comp_max(-0.25)]
TIGHT = [comp_max(-0.35), {"type": "timescale_lag", "lag_by": 0.1, "timescale": "FiveMinute"}, ts_max("FiveMinute", -0.5)]


def gap_patch(g, shape, extra=(), before="10:30"):
    conds = [c_min("gap_1m.gap_pct", g), c_max("gap_1m.above_open", 0.0), c_max("cross_1m", 0.4),
             c_max("cross_1m.index_ret_prior_close", 0.3)] + shape + list(extra)
    return {"indicators": GAP_IND, "actions": [window("gap_fade_short", "gap fade short", "short", conds, before=before)]}


cell("gap_g05_loose", gap_patch(0.5, LOOSE), "v18 + gap-fade short: gap >= 0.5%, below open, SPY <= +0.2% (session) and <= +0.3% vs prior close, 5m <= -0.3, comp <= -0.25, to 10:30")
cell("gap_g10_loose", gap_patch(1.0, LOOSE), "as gap_g05_loose with gap >= 1.0%")
cell("gap_g05_tight", gap_patch(0.5, TIGHT), "gap >= 0.5% with the thrust shape (comp <= -0.35, 5m lags 0.1, 5m <= -0.5), no 1h condition")
cell("gap_g10_tight", gap_patch(1.0, TIGHT), "gap >= 1.0% with the thrust shape")
cell("gap_g05_loose_vpin", gap_patch(0.5, LOOSE, VPIN), "gap_g05_loose + VPIN floor 0.217")
cell("gap_g05_loose_1130", gap_patch(0.5, LOOSE, before="11:30"), "gap_g05_loose with entries to 11:30")

# ---------------- F3 relative-volume / OFI-led thrust as a second short window
VOL_IND = [ind("rvol_1m", "relative_volume", "OneMinute", lookback_period=20),
           ind("ofi_1m", "ofi", "OneMinute", avg_period=20)]


def vol_patch(conds):
    return {"indicators": VOL_IND, "actions": [window("vol_thrust_short", "volume thrust short", "short", conds + BAND)]}


cell("vol_r20_o05", vol_patch([c_min("rvol_1m.rvol", 2.0), c_max("ofi_1m", -0.5)] + LOOSE), "v18 + rvol >= 2, OFI <= -0.5, 5m <= -0.3, comp <= -0.25, SPY band")
cell("vol_r30_o05", vol_patch([c_min("rvol_1m.rvol", 3.0), c_max("ofi_1m", -0.5)] + LOOSE), "as vol_r20_o05 with rvol >= 3")
cell("vol_r20_o05_vpin", vol_patch([c_min("rvol_1m.rvol", 2.0), c_max("ofi_1m", -0.5)] + LOOSE + VPIN), "vol_r20_o05 + VPIN floor")
cell("vol_r20_o08_1h", vol_patch([c_min("rvol_1m.rvol", 2.0), c_max("ofi_1m", -0.8)] + LOOSE + [ts_max("OneHour", 0.0)]), "rvol >= 2, OFI <= -0.8, loose shape, 1h <= 0")
cell("vol_ofi08_vpin", vol_patch([c_max("ofi_1m", -0.8)] + LOOSE + VPIN), "OFI-led without rvol: OFI <= -0.8, loose shape, VPIN floor")
cell("vol_r20_o05_flow", vol_patch([c_min("rvol_1m.rvol", 2.0), c_max("ofi_1m", -0.5), ts_max("OneHour", -0.2)]), "pure flow: rvol >= 2, OFI <= -0.5, 1h <= -0.2, no composite / 5m condition")

# ---------------- F4 prior-day-low break short
PDL_IND = [ind("pdl_5m", "prior_day_levels", "FiveMinute", scale_pct=0.005)]


def pdl_patch(lo, conds):
    c = [c_min("pdl_5m.dist_low_pct", -lo), c_max("pdl_5m.dist_low_pct", 0.0)] + conds + BAND
    return {"indicators": PDL_IND, "actions": [window("pdl_break_short", "prior low break short", "short", c)]}


cell("pdl_l05", pdl_patch(0.5, [ts_max("FiveMinute", -0.3)]), "v18 + short within 0.5% below the prior-day low, 5m <= -0.3, SPY band")
cell("pdl_l10", pdl_patch(1.0, [ts_max("FiveMinute", -0.3)]), "as pdl_l05 within 1.0% below")
cell("pdl_l05_vpin", pdl_patch(0.5, [ts_max("FiveMinute", -0.3)] + VPIN), "pdl_l05 + VPIN floor")
cell("pdl_l05_strict", pdl_patch(0.5, [ts_max("FiveMinute", -0.3), comp_max(-0.35)] + VPIN), "pdl_l05 + comp <= -0.35 + VPIN")
cell("pdl_l05_pure", pdl_patch(0.5, VPIN), "pure break: within 0.5% below prior low, SPY band, VPIN; no score condition")
cell("pdl_l05_1h", pdl_patch(0.5, [ts_max("FiveMinute", -0.3), ts_max("OneHour", 0.0)]), "pdl_l05 + 1h <= 0")

# ---------------- F5 5m bearish candle sequence inside the filters
def eng_patch(pattern, conds):
    return {"indicators": [ind("seq_5m", "candle_sequence", "FiveMinute", pattern=pattern, direction="bearish")],
            "actions": [window("seq_short", f"{pattern} 5m short", "short", [c_max("seq_5m", -0.5)] + BAND + VPIN + conds)]}


cell("eng_band_vpin", eng_patch("engulfing", []), "v18 + 5m bearish engulfing inside SPY band + VPIN floor")
cell("eng_band_vpin_5m", eng_patch("engulfing", [ts_max("FiveMinute", -0.2)]), "eng_band_vpin + 5m <= -0.2")
cell("eng_band_vpin_1h", eng_patch("engulfing", [ts_max("OneHour", 0.0)]), "eng_band_vpin + 1h <= 0")
cell("crows_band_vpin", eng_patch("three_crows", []), "v18 + 5m three black crows inside SPY band + VPIN floor")

# ---------------- F1b short-only afternoon fade on the 4 home names (screen: the long side lost on every cell)
def pm_short(x, hold=60, t="12:30", prio=-5):
    h, m = map(int, t.split(":")); end = h * 60 + m + hold; nxt = h * 60 + m + 1
    ov = {"force_exit_by": f"{end // 60}:{end % 60:02d}", "score_exit_threshold": -10.0, "max_hold_ms": hold * 60000}
    return [window("pm_fade_short", "pm vwap fade short", "short", [c_min("vwap_dist_1hr.dist_pct", x)],
                   prio=prio, after=t, before=f"{nxt // 60}:{nxt % 60:02d}", exit_overrides=ov)]


for x, tag in ((0.75, "x075"), (1.0, "x10"), (1.25, "x125"), (1.5, "x15"), (2.0, "x20")):
    cell(f"pms_{tag}", {"disable": DISABLE, "actions": AM + pm_short(x), "session": OPEN_SESSION},
         f"scaffold + short-only afternoon VWAP fade: dist >= {x}% on the 12:30 bar, hold 60, 4 names")
for hold in (45, 90):
    cell(f"pms_x10_h{hold}", {"disable": DISABLE, "actions": AM + pm_short(1.0, hold=hold), "session": OPEN_SESSION},
         f"pms_x10 with hold {hold}")
for t in ("12:00", "13:00"):
    cell(f"pms_x10_t{t.replace(':', '')}", {"disable": DISABLE, "actions": AM + pm_short(1.0, t=t), "session": OPEN_SESSION},
         f"pms_x10 entered on the {t} bar")

# ---------------- F3b disjoint volume thrust: 1h > 0, where neither v18 window can fire (both need 1h <= 0),
# so the new window cannot pre-empt a baseline entry (screen: every F3 cell displaced 28-62 baseline trades)
def ts_min(ts, v): return {"type": "timescale_min", "min_score": v, "timescale": ts}


cell("vol_r30_o05_1hpos", vol_patch([c_min("rvol_1m.rvol", 3.0), c_max("ofi_1m", -0.5)] + LOOSE + [ts_min("OneHour", 0.01)]),
     "rvol >= 3, OFI <= -0.5, loose shape, SPY band, 1h > 0 (disjoint from v18's windows)")
cell("vol_r20_o05_1hpos", vol_patch([c_min("rvol_1m.rvol", 2.0), c_max("ofi_1m", -0.5)] + LOOSE + [ts_min("OneHour", 0.01)]),
     "rvol >= 2, OFI <= -0.5, loose shape, SPY band, 1h > 0 (disjoint)")

if __name__ == "__main__":
    for name, (patch, note, tickers) in cells.items():
        (P / f"{name}.json").write_text(json.dumps(patch, indent=1) + "\n")
        spec = {"name": f"signals-new/{name}", "config_id": 12,
                "patches": [f"research/lab/patches/signals-new/{name}.json"], "tickers": tickers, "note": note}
        (S / f"{name}.json").write_text(json.dumps(spec, indent=1) + "\n")
    print(len(cells), "cells written")
