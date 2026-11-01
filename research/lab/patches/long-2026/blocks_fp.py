# block F: best-of-screen long combination (1h st bullish + signed ADX >= 0.4 + VPIN floor), twin mirrored
cell("f1_adx40_vpin", "a3 (st bullish + signed ADX>=0.4) + VPIN>=0.217 | twin = st bearish + ADX<=-0.4, shorts without VPIN", BULL + adx_l(0.4) + VPIN, BEAR + adx_s(0.4), short_vpin=False)
# (unrun, removed) cell("f2_adx40_vpin_na", "f1 on NVDA,AAPL only (HISTORY E: ticker-specific long inclusion)", BULL + adx_l(0.4) + VPIN, BEAR + adx_s(0.4), short_vpin=False, tickers="NVDA,AAPL")
# (unrun, removed) cell("f3_long_only_a3", "a3 long side standalone, shorts disabled (interaction control for the mirrored books)", BULL + adx_l(0.4), shorts="none")

# block P: coordinator pivot (meta desk) -- row 23 (cand_48 QQQ noise-area afternoon, high-vol day gate) on the mega-caps
def im(direction, dm):
    up = direction == "long"
    conds = [c_min("noise.decision", 0.5), (c_min("noise.pos", 1.0) if up else c_max("noise.pos", -1.0)),
             (c_min("noise.vwap_pct", 0.0) if up else c_max("noise.vwap_pct", 0.0)), c_min("noise.day_move_pct", dm)]
    p = {"name": f"im {direction}", "direction": direction, "conditions": conds, "entry_after": "11:59", "entry_before": "15:30",
         "exit_overrides": {"max_hold_ms": 25200000, "force_exit_by": "15:58", "score_exit_threshold": -1000000000.0}}
    return {"instance_id": f"window_im_{direction}", "action_type": "entry_window", "phase": "Entry", "priority": 10 if up else 11, "enabled": True, "params": p}
def pcell(name, note, dm, tickers="AAPL,AMZN,NVDA,MSFT"):
    patch = {"actions": [im("long", dm), im("short", dm)]}
    (P / f"{name}.json").write_text(json.dumps(patch, indent=1))
    (S / f"{name}.json").write_text(json.dumps({"name": f"long-2026/{name}", "config_id": 23, "lookback": 22, "tickers": tickers,
        "patches": [f"research/lab/patches/long-2026/{name}.json"], "note": note}, indent=1))
pcell("p1_mc_dm07", "row 23 (cand_48 noise-area pm, day_move>=0.7) on AAPL AMZN NVDA MSFT; long side = target, im short = twin", 0.7)
pcell("p2_mc_dm15", "p1 with the high-vol gate at day_move_pct>=1.5 (mega-caps move ~2x QQQ)", 1.5)
pcell("p3_mc_dm20", "p1 with day_move_pct>=2.0", 2.0)
cell("f4_na_spypc", "f2 (NVDA,AAPL morning long, st bull + ADX>=0.4 + VPIN) + SPY above prior close (2022 guard) | twin mirrored", BULL + adx_l(0.4) + VPIN + SPY_PC_UP, BEAR + adx_s(0.4) + SPY_PC_DN, short_vpin=False, tickers="NVDA,AAPL")
