#!/usr/bin/env python3
"""section 3: intraday event angle on data/bars_iex/<T>.csv (1-minute RTH, unix seconds, 2022-01-03..2026-09-24).
reaction day = next trading session after the EDGAR 8-K 2.02 date (all four report after the close).
FOMC decision days from crates/indicators/src/custom/event_calendar.rs (decision at 14:00 ET, so the morning is pre-decision).
measures per name: 09:30->11:30 long return distribution by day type, and continuation after a +0.5% first 15 minutes."""
import csv, os, json, datetime as dt, numpy as np
from zoneinfo import ZoneInfo
ROOT = os.path.dirname(os.path.abspath(__file__)); REPO = os.path.join(ROOT, "..", "..")
ET = ZoneInfo("US/Eastern"); FOUR = ["AAPL", "AMZN", "MSFT", "NVDA"]
FOMC = set("""2022-01-26 2022-03-16 2022-05-04 2022-06-15 2022-07-27 2022-09-21 2022-11-02 2022-12-14
2023-02-01 2023-03-22 2023-05-03 2023-06-14 2023-07-26 2023-09-20 2023-11-01 2023-12-13
2024-01-31 2024-03-20 2024-05-01 2024-06-12 2024-07-31 2024-09-18 2024-11-07 2024-12-18
2025-01-29 2025-03-19 2025-05-07 2025-06-18 2025-07-30 2025-09-17 2025-10-29 2025-12-10
2026-01-28 2026-03-18 2026-04-29 2026-06-17 2026-07-29 2026-09-16""".split())
def load(sym):
    days = {}
    for r in csv.reader(open(os.path.join(REPO, "data", "bars_iex", sym + ".csv"))):
        if r[0] == "timestamp": continue
        t = dt.datetime.fromtimestamp(int(r[0]), ET); m = t.hour * 60 + t.minute
        days.setdefault(t.strftime("%Y-%m-%d"), {})[m] = (float(r[1]), float(r[4]))
    return days
def px(bars, minute, which):  # nearest bar at/after minute (open) or at/before (close)
    if which == "open":
        for m in range(minute, minute + 6):
            if m in bars: return bars[m][0]
    else:
        for m in range(minute - 1, minute - 7, -1):
            if m in bars: return bars[m][1]
    return None
def spy_days(): return load("SPY")
def main():
    out = {}; spy = spy_days()
    for sym in FOUR:
        days = load(sym)
        filed = [x.strip() for x in open(os.path.join(REPO, "research", "entries", "data", f"earnings_{sym}.txt")) if x.strip()]
        alld = sorted(days); react = set()
        for f in filed:
            nxt = [d for d in alld if d > f]
            if nxt: react.add(nxt[0])
        rows = []
        for d in alld:
            b = days[d]; o = px(b, 570, "open"); c15 = px(b, 585, "close"); c1130 = px(b, 690, "close")
            if not (o and c15 and c1130) or len(b) < 100: continue
            kind = "earnings" if d in react else ("fomc" if d in FOMC else "ordinary")
            rows.append((d, kind, c1130 / o - 1, c15 / o - 1, c1130 / c15 - 1))
        res = {}
        for kind in ("earnings", "fomc", "ordinary"):
            rr = [r for r in rows if r[1] == kind]; R = np.array([r[2] for r in rr]); f15 = np.array([r[3] for r in rr]); cont = np.array([r[4] for r in rr])
            up = f15 >= 0.005; dn = f15 <= -0.005
            res[kind] = {"days": len(rr), "ret_0930_1130_mean_bps": round(1e4 * R.mean(), 1), "median_bps": round(1e4 * np.median(R), 1), "std_bps": round(1e4 * R.std()),
                         "win%": round(100 * (R > 0).mean(), 1), "p(|r|>1%)": round(100 * (np.abs(R) > 0.01).mean(), 1), "p(r>+1%)": round(100 * (R > 0.01).mean(), 1),
                         "first15_up>=0.5%_days": int(up.sum()), "cont_after_up_mean_bps": round(1e4 * cont[up].mean(), 1) if up.any() else None,
                         "cont_after_up_win%": round(100 * (cont[up] > 0).mean(), 1) if up.any() else None,
                         "first15_dn<=-0.5%_days": int(dn.sum()), "cont_after_dn_mean_bps": round(1e4 * cont[dn].mean(), 1) if dn.any() else None,
                         "cont_after_dn_win%": round(100 * (cont[dn] < 0).mean(), 1) if dn.any() else None}
        # what share of the strong-up mornings are event days?
        strong = [r for r in rows if r[2] > 0.01]; res["share_of_>+1%_mornings_that_are_event_days"] = round(100 * sum(1 for r in strong if r[1] != "ordinary") / len(strong), 1)
        res["event_day_share_of_all_days"] = round(100 * sum(1 for r in rows if r[1] != "ordinary") / len(rows), 1)
        # earnings reaction split by gap direction: gap up (open vs prior close) -> morning continuation?
        prevc = {}; 
        for i in range(1, len(alld)):
            pb = days[alld[i - 1]]; pc = px(pb, 960, "close"); prevc[alld[i]] = pc
        gu = [r for r in rows if r[1] == "earnings" and prevc.get(r[0]) and px(days[r[0]], 570, "open") / prevc[r[0]] - 1 > 0.02]
        gd = [r for r in rows if r[1] == "earnings" and prevc.get(r[0]) and px(days[r[0]], 570, "open") / prevc[r[0]] - 1 < -0.02]
        res["earnings_gap_up>2%"] = {"days": len(gu), "ret_0930_1130_mean_bps": round(1e4 * np.mean([r[2] for r in gu]), 1) if gu else None, "win%": round(100 * np.mean([r[2] > 0 for r in gu]), 1) if gu else None}
        res["earnings_gap_dn<-2%"] = {"days": len(gd), "ret_0930_1130_mean_bps": round(1e4 * np.mean([r[2] for r in gd]), 1) if gd else None, "win%": round(100 * np.mean([r[2] > 0 for r in gd]), 1) if gd else None}
        out[sym] = res
        print(sym, json.dumps(res, indent=None)[:1500])
    # pooled
    pooled = {}
    for kind in ("earnings", "fomc", "ordinary"):
        n = sum(out[s][kind]["days"] for s in FOUR); pooled[kind] = {"days": n,
            "ret_mean_bps": round(sum(out[s][kind]["ret_0930_1130_mean_bps"] * out[s][kind]["days"] for s in FOUR) / n, 1),
            "win%": round(sum(out[s][kind]["win%"] * out[s][kind]["days"] for s in FOUR) / n, 1),
            "up_days": sum(out[s][kind]["first15_up>=0.5%_days"] for s in FOUR),
            "cont_after_up_win%": round(sum((out[s][kind]["cont_after_up_win%"] or 0) * out[s][kind]["first15_up>=0.5%_days"] for s in FOUR) / max(1, sum(out[s][kind]["first15_up>=0.5%_days"] for s in FOUR)), 1),
            "cont_after_up_mean_bps": round(sum((out[s][kind]["cont_after_up_mean_bps"] or 0) * out[s][kind]["first15_up>=0.5%_days"] for s in FOUR) / max(1, sum(out[s][kind]["first15_up>=0.5%_days"] for s in FOUR)), 1)}
    out["pooled"] = pooled; print("POOLED", json.dumps(pooled))
    json.dump(out, open(os.path.join(ROOT, "results_intraday_events.json"), "w"), indent=1)
main()
