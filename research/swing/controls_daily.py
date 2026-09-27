#!/usr/bin/env python3
"""benchmarks for the PEAD / hold-N results: unconditional N-day long holds (every non-overlapping window, buy open
sell close N sessions later) on the universe and the four names, per year and per regime, plus 'any earnings
reaction day regardless of sign' and 'random-day' baselines. same costs/sizing as sim_daily."""
import json, os, numpy as np, sim_daily as S
ROOT = os.path.dirname(os.path.abspath(__file__))
def s_uncond(sym, a, hold=10, offset=0):
    o, c = a["open"], a["close"]; i0 = S.IDX[next(d for d in S.DATES if d >= S.T0)]
    for e in range(i0 + offset, S.N - hold, hold):
        yield S.trade(sym, e, e + hold - 1, o[e], c[e + hold - 1], "uncond")
def s_pead_any(sym, a, hold=10):
    o, c = a["open"], a["close"]; days, _ = S.EARN[sym]
    for r in sorted(days):
        if r < 1 or r + hold >= S.N: continue
        e = r + 1; x = e + hold - 1; yield S.trade(sym, e, x, o[e], c[x], "pead_any")
def s_pead_gap(sym, a, hold=10, thresh=0.03):  # trigger on the open gap of the reaction day instead of c2c
    o, c = a["open"], a["close"]; days, _ = S.EARN[sym]
    for r in sorted(days):
        if r < 1 or r + hold >= S.N: continue
        if o[r] / c[r - 1] - 1 > thresh:
            e = r + 1; x = e + hold - 1; yield S.trade(sym, e, x, o[e], c[x], "pead_gap")
def s_pead_h10_stop(sym, a, hold=10, thresh=0.03, stop=0.05):  # gap-aware 5% stop on the PEAD hold
    o, h, l, c = a["open"], a["high"], a["low"], a["close"]; days, _ = S.EARN[sym]
    for r in sorted(days):
        if r < 1 or r + hold >= S.N: continue
        if c[r] / c[r - 1] - 1 > thresh:
            e = r + 1; x = e + hold - 1; px = o[e]; sp = px * (1 - stop); xp = c[x]
            for k in range(e, x + 1):
                if k > e and o[k] <= sp: x, xp = k, o[k]; break
                if l[k] <= sp: x, xp = k, sp; break
            yield S.trade(sym, e, x, px, xp, "pead_stop")
OUT = {}
for label, syms in (("four", S.FOUR), ("universe", S.NAMES)):
    for hold in (5, 10):
        tr = S.run(s_uncond, syms, hold=hold); OUT[f"uncond_h{hold}_{label}"] = S.table(tr, label)
        st = OUT[f"uncond_h{hold}_{label}"]
        print(f"uncond h{hold} {label}: all pf={st['all']['pf']} bps={st['all']['avg_bps']} win={st['all']['win%']} | >200 pf={st['spy>200']['pf']} bps={st['spy>200']['avg_bps']} | <200 pf={st['spy<200']['pf']} bps={st['spy<200']['avg_bps']} | years " + " ".join(f"{y}:{st[y]['pf']}/{st[y]['avg_bps']}" for y in S.YEARS))
    OUT[f"pead_any_h10_{label}"] = S.table(S.run(s_pead_any, syms), label); st = OUT[f"pead_any_h10_{label}"]["all"]; print(f"pead ANY sign h10 {label}:", st)
    OUT[f"pead_gap3_h10_{label}"] = S.table(S.run(s_pead_gap, syms), label); st = OUT[f"pead_gap3_h10_{label}"]["all"]; print(f"pead open-gap>3% h10 {label}:", st, "n/yr", OUT[f"pead_gap3_h10_{label}"]["trades_per_year"])
    OUT[f"pead_h10_stop5_{label}"] = S.table(S.run(s_pead_h10_stop, syms), label); st = OUT[f"pead_h10_stop5_{label}"]["all"]; print(f"pead h10 with 5% stop {label}:", st)
# excess of PEAD h10 over unconditional h10 per year (bps/trade)
p = S.table(S.run(S.s_pead, S.NAMES, hold=10), "universe"); u = OUT["uncond_h10_universe"]
OUT["pead_h10_excess_bps_by_year"] = {y: round(p[y]["avg_bps"] - u[y]["avg_bps"], 1) for y in S.YEARS}
OUT["pead_h10_excess_bps_all"] = round(p["all"]["avg_bps"] - u["all"]["avg_bps"], 1)
print("PEAD h10 excess over unconditional h10, bps/trade:", OUT["pead_h10_excess_bps_all"], OUT["pead_h10_excess_bps_by_year"])
# same for breakout 20d h10 and reversal h5 (their >200 slices vs unconditional >200)
b = S.table(S.run(S.s_breakout, S.NAMES, hold=10, kind="20d"), "universe"); r = S.table(S.run(S.s_reversal, S.NAMES, hold=5), "universe")
OUT["bo20_h10_excess_bps_vs_uncond_spy>200"] = round(b["spy>200"]["avg_bps"] - u["spy>200"]["avg_bps"], 1)
OUT["rev_h5_excess_bps_vs_uncond_spy>200"] = round(r["spy>200"]["avg_bps"] - OUT["uncond_h5_universe"]["spy>200"]["avg_bps"], 1)
print("bo20 h10 excess vs uncond (spy>200):", OUT["bo20_h10_excess_bps_vs_uncond_spy>200"], " rev h5 excess vs uncond h5 (spy>200):", OUT["rev_h5_excess_bps_vs_uncond_spy>200"])
json.dump(OUT, open(os.path.join(ROOT, "results_controls.json"), "w"), indent=1, default=str)
