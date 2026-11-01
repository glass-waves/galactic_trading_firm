# block Q: the afternoon high-vol breakout long on the mega-caps, long side alone (im short disabled)
def qcell(name, note, dm, tickers="AAPL,AMZN,NVDA,MSFT"):
    patch = {"actions": [im("long", dm)], "disable": ["window_im_short"]}
    (P / f"{name}.json").write_text(json.dumps(patch, indent=1))
    (S / f"{name}.json").write_text(json.dumps({"name": f"long-2026/{name}", "config_id": 23, "lookback": 22, "tickers": tickers,
        "patches": [f"research/lab/patches/long-2026/{name}.json"], "note": note}, indent=1))
exec((P / "blocks_fp.py").read_text())  # defines im(); regenerates block F/P files byte-identically
qcell("q1_dm15_long", "afternoon noise-area breakout LONG only, day_move_pct>=1.5, AAPL AMZN NVDA MSFT (row 23 exits)", 1.5)
qcell("q2_dm125_long", "q1 neighbour: day_move_pct>=1.25", 1.25)
qcell("q3_dm175_long", "q1 neighbour: day_move_pct>=1.75", 1.75)

# combined form: row 12 (v18) + the q1 long window. v18 windows re-created with entry_before 11:30 because the session must
# stay open to 15:30; exit_session (11:55 action) disabled -- the engine's force-exit safety net keeps v18 positions at the
# session's 11:55 and the im window's override carries its own 15:58. lookback 22 (noise_area needs it).
def v18_short(k):
    iid, nm, pr, conds = SHORT[k]
    return win(iid, nm, pr, "short", copy.deepcopy(conds) + BAND + VPIN, (None, "11:30"))
NOISE = {"instance_id": "noise", "indicator_type": "noise_area", "timescale": "OneMinute", "weight": 0.0, "enabled": True,
         "params": {"band_mult": 1.0, "gap_adjust": True, "lookback_sessions": 14, "decision_every_min": 30, "decision_tolerance_min": 2}}
def kcell(name, note, extra_actions, lookback=22):
    patch = {"indicators": [NOISE], "actions": [v18_short("thrust"), v18_short("core")] + extra_actions,
             "disable": ["exit_session"], "session": {"no_new_entries_after": "15:30"}}
    (P / f"{name}.json").write_text(json.dumps(patch, indent=1))
    (S / f"{name}.json").write_text(json.dumps({"name": f"long-2026/{name}", "config_id": 12, "lookback": lookback,
        "patches": [f"research/lab/patches/long-2026/{name}.json"], "note": note}, indent=1))
kcell("k0_comb_scaffold", "scaffold for the combined form: v18 re-created with entry_before 11:30, session open to 15:30, exit_session disabled, noise added, lookback 22, no new window; must equal base/v18", [])
kcell("k1_comb_dm15", "v18 + afternoon noise-area breakout long (day_move>=1.5) as an added window", [im("long", 1.5)])
kcell("k0b_comb_scaffold_lb8", "k0 at lookback 8: isolates the patch mechanics from the lookback-22 warm-up difference", [], lookback=8)

# block R: one factor at a time on q1 (the desk's regime list, applied to the afternoon long)
def rcell(name, note, extra, inds=()):
    w = im("long", 1.5); w["params"]["conditions"] += extra
    patch = {"indicators": [copy.deepcopy(i) for i in inds], "actions": [w], "disable": ["window_im_short"]}
    (P / f"{name}.json").write_text(json.dumps(patch, indent=1))
    (S / f"{name}.json").write_text(json.dumps({"name": f"long-2026/{name}", "config_id": 23, "lookback": 22, "tickers": "AAPL,AMZN,NVDA,MSFT",
        "patches": [f"research/lab/patches/long-2026/{name}.json"], "note": note}, indent=1))
rcell("r1_q1_spyup", "q1 + SPY above its open at entry", SPY_UP)
rcell("r2_q1_spypc", "q1 + SPY above prior close at entry", SPY_PC_UP)
rcell("r3_q1_vpin", "q1 + VPIN >= 0.217", VPIN)
rcell("r4_q1_bull1h", "q1 + 1h supertrend(standard) bullish (the directional 1h)", BULL, inds=IND[:1])

# block V: neighbourhood of r3 (q1 + VPIN floor), +-1 step on each knob; combined form of r3
def vcell(name, note, dm, vpin):
    rcell_conds = [c_min("vpin_1m.raw_vpin", vpin)]
    w = im("long", dm); w["params"]["conditions"] += rcell_conds
    patch = {"indicators": [], "actions": [w], "disable": ["window_im_short"]}
    (P / f"{name}.json").write_text(json.dumps(patch, indent=1))
    (S / f"{name}.json").write_text(json.dumps({"name": f"long-2026/{name}", "config_id": 23, "lookback": 22, "tickers": "AAPL,AMZN,NVDA,MSFT",
        "patches": [f"research/lab/patches/long-2026/{name}.json"], "note": note}, indent=1))
vcell("v1_dm15_vp18", "r3 neighbour: VPIN floor 0.18", 1.5, 0.18)
vcell("v2_dm15_vp26", "r3 neighbour: VPIN floor 0.26", 1.5, 0.26)
vcell("v3_dm125_vp217", "r3 neighbour: day_move>=1.25", 1.25, 0.217)
vcell("v4_dm175_vp217", "r3 neighbour: day_move>=1.75", 1.75, 0.217)
# (k2_comb_r3 dropped unrun: k1 already showed v18's exits gut the afternoon window in the added form)
# short twin of r3: the mirrored afternoon window (im short, day_move>=1.5) with the same VPIN floor, long disabled
_s = im("short", 1.5); _s["params"]["conditions"] += VPIN
_patch = {"actions": [_s], "disable": ["window_im_long"]}
(P / "t1_r3_short_twin.json").write_text(json.dumps(_patch, indent=1))
(S / "t1_r3_short_twin.json").write_text(json.dumps({"name": "long-2026/t1_r3_short_twin", "config_id": 23, "lookback": 22, "tickers": "AAPL,AMZN,NVDA,MSFT",
    "patches": ["research/lab/patches/long-2026/t1_r3_short_twin.json"], "note": "short twin of r3: im short (pos<=-1, below VWAP, day_move>=1.5) + VPIN>=0.217"}, indent=1))

# block W: r3 (VPIN floor) x r4 (directional 1h bullish) -- the two factors that each helped on train
rcell("w1_q1_vpin_bull1h", "q1 + VPIN>=0.217 + 1h supertrend(standard) bullish", VPIN + BULL, inds=IND[:1])
_w1 = im("long", 1.5); _w1["params"]["conditions"] += VPIN + BULL
_k = {"indicators": [NOISE, copy.deepcopy(IND[0])], "actions": [v18_short("thrust"), v18_short("core"), _w1],
      "disable": ["exit_session"], "session": {"no_new_entries_after": "15:30"}}
(P / "k3_comb_w1.json").write_text(json.dumps(_k, indent=1))
(S / "k3_comb_w1.json").write_text(json.dumps({"name": "long-2026/k3_comb_w1", "config_id": 12, "lookback": 22,
    "patches": ["research/lab/patches/long-2026/k3_comb_w1.json"], "note": "v18 + w1 (afternoon breakout long, day_move>=1.5, VPIN>=0.217, 1h st bullish) as an added window; lookback 22"}, indent=1))
