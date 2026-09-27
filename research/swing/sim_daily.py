#!/usr/bin/env python3
"""daily-bar simulations for the multi-day long research (2022-01-01..2026-09-25).
all numbers in the report's section 2/4 come from here. costs: 10 bps round trip per trade.
sizing: equal dollar, $10,000 notional per trade, at most one open position per name.
outputs research/swing/results_daily.json and a markdown dump research/swing/results_daily.md"""
import os, csv, json, math, datetime as dt
import numpy as np
ROOT = os.path.dirname(os.path.abspath(__file__))
from fetch_daily import UNIVERSE
FOUR = ["AAPL", "AMZN", "MSFT", "NVDA"]
NAMES = [s for s in UNIVERSE if s != "SPY"]
COST = 0.0010            # 10 bps round trip
NOTIONAL = 10_000.0
T0, T1 = "2022-01-01", "2026-09-25"

def load(sym):
    rows = list(csv.DictReader(open(os.path.join(ROOT, "daily", sym + ".csv"))))
    d = [r["date"] for r in rows]
    a = {k: np.array([float(r[k]) for r in rows]) for k in ("open", "high", "low", "close", "volume")}
    return d, a
DATA = {s: load(s) for s in UNIVERSE}
DATES = DATA["SPY"][0]
for s in UNIVERSE: assert DATA[s][0] == DATES, s
N = len(DATES)
IDX = {d: i for i, d in enumerate(DATES)}
YEARS = sorted(set(d[:4] for d in DATES if d >= T0))

def sma(x, n):
    out = np.full_like(x, np.nan); c = np.cumsum(np.insert(x, 0, 0.0))
    out[n - 1:] = (c[n:] - c[:-n]) / n; return out
def rolling_max(x, n):   # max over the previous n bars, excluding today
    out = np.full_like(x, np.nan)
    for i in range(n, len(x)): out[i] = x[i - n:i].max()
    return out
SPY = DATA["SPY"][1]
SPY200 = SPY["close"] > sma(SPY["close"], 200)
SPY50 = SPY["close"] > sma(SPY["close"], 50)

# ---------- earnings reaction days ----------
def earnings_reaction_days(sym):
    """set of indices of the earnings *reaction* session. source: EDGAR 8-K item 2.02 filing dates
    (research/swing/earnings/<sym>.txt); reaction day = filing day or next session, whichever gaps more.
    one date per calendar quarter (largest |gap|); quarters with no filing use the largest-|gap| day proxy."""
    d, a = DATA[sym]; gap = np.abs(a["open"][1:] / a["close"][:-1] - 1); gap = np.insert(gap, 0, 0)
    p = os.path.join(ROOT, "earnings", sym + ".txt")
    filed = [x.strip() for x in open(p)] if os.path.exists(p) else []
    byq = {}
    for f in filed:
        if f < DATES[0] or f > DATES[-1]: continue
        i = next((k for k in range(N) if DATES[k] >= f), None)
        if i is None: continue
        cand = [i] + ([i + 1] if i + 1 < N else [])
        r = max(cand, key=lambda k: gap[k])
        q = (DATES[r][:4], (int(DATES[r][5:7]) - 1) // 3)
        if q not in byq or gap[r] > gap[byq[q]]: byq[q] = r
    # fill quarters with no filing (XOM) with the largest-gap day of that quarter
    for i in range(1, N):
        q = (DATES[i][:4], (int(DATES[i][5:7]) - 1) // 3)
        if q not in byq: byq[q] = i
        elif not filed and gap[i] > gap[byq[q]]: byq[q] = i
    if not filed:
        byq = {}
        for i in range(1, N):
            q = (DATES[i][:4], (int(DATES[i][5:7]) - 1) // 3)
            if q not in byq or gap[i] > gap[byq[q]]: byq[q] = i
    return set(byq.values()), bool(filed)
EARN = {s: earnings_reaction_days(s) for s in NAMES}

# ---------- trade bookkeeping ----------
def trade(sym, ei, xi, entry_px, exit_px, tag=""):
    r = exit_px / entry_px - 1 - COST
    return {"sym": sym, "entry": DATES[ei], "exit": DATES[xi], "ret": r, "pnl": r * NOTIONAL,
            "year": DATES[ei][:4], "spy200": bool(SPY200[ei]), "hold": xi - ei, "tag": tag}

def stats(trades):
    if not trades: return {"trades": 0}
    r = np.array([t["ret"] for t in trades]); pnl = r * NOTIONAL
    g, l = pnl[pnl > 0].sum(), -pnl[pnl < 0].sum()
    order = sorted(trades, key=lambda t: (t["exit"], t["entry"]))
    eq = np.cumsum([t["pnl"] for t in order]); peak = np.maximum.accumulate(np.insert(eq, 0, 0))[1:]
    mdd = float((eq - peak).min()) if len(eq) else 0.0
    return {"trades": int(len(r)), "pnl": round(float(pnl.sum())), "win%": round(100 * float((r > 0).mean()), 1),
            "pf": round(float(g / l), 2) if l > 0 else float("inf"), "avg_bps": round(1e4 * float(r.mean()), 1),
            "maxdd": round(mdd), "worst": round(float(pnl.min())), "avg_hold": round(float(np.mean([t["hold"] for t in trades])), 1)}

def table(trades, syms_label):
    out = {"all": stats(trades)}
    for y in YEARS: out[y] = stats([t for t in trades if t["year"] == y])
    out["spy>200"] = stats([t for t in trades if t["spy200"]]); out["spy<200"] = stats([t for t in trades if not t["spy200"]])
    nyears = (dt.date(2026, 9, 25) - dt.date(2022, 1, 1)).days / 365.25
    out["trades_per_year"] = round(len(trades) / nyears, 1); out["universe"] = syms_label
    return out

def run(strategy, syms, **kw):
    trades = []
    for s in syms:
        d, a = DATA[s]; open_until = -1
        for t in strategy(s, a, **kw):
            if t["entry"] < T0 or t["entry"] > T1: continue
            ei = IDX[t["entry"]]
            if ei <= open_until: continue   # one position per name
            open_until = IDX[t["exit"]]; trades.append(t)
    return trades

# ---------- strategies (each yields trade dicts in date order) ----------
def s_pead(sym, a, hold=5, thresh=0.03):
    o, c = a["open"], a["close"]; days, _ = EARN[sym]
    for r in sorted(days):
        if r < 1 or r + hold >= N: continue
        if c[r] / c[r - 1] - 1 > thresh:
            e = r + 1; x = e + hold - 1
            yield trade(sym, e, x, o[e], c[x], "pead")

def s_reversal(sym, a, hold=5, mode="fixed", regime=True):
    c, h = a["close"], a["high"]; s50 = sma(c, 50); hi5 = rolling_max(h, 5)
    for t in range(60, N - 1):
        if regime and not SPY200[t]: continue
        if not (c[t] > s50[t]): continue
        three = c[t] < c[t - 1] < c[t - 2] < c[t - 3]; r5 = c[t] / c[t - 5] - 1 < -0.04
        if not (three or r5): continue
        if mode == "fixed":
            x = min(t + hold, N - 1)
        else:  # exit first close above the prior 5-day high, max 10 sessions
            x = min(t + 10, N - 1)
            for k in range(t + 1, min(t + 11, N)):
                if c[k] > hi5[k]: x = k; break
        yield trade(sym, t, x, c[t], c[x], "rev")

def s_breakout(sym, a, hold=10, kind="52w", stop=0.05, regime=True):
    o, h, l, c, v = a["open"], a["high"], a["low"], a["close"], a["volume"]
    hi252, hi20, v20 = rolling_max(h, 252), rolling_max(h, 20), sma(v, 20)
    for t in range(253, N - 1):
        if regime and not SPY200[t]: continue
        sig = c[t] > hi252[t] if kind == "52w" else (c[t] > hi20[t] and v[t] > 1.5 * v20[t - 1])
        if not sig: continue
        px = c[t]; stop_px = px * (1 - stop); x = min(t + hold, N - 1); exit_px = c[x]
        for k in range(t + 1, x + 1):
            if o[k] <= stop_px: x, exit_px = k, o[k]; break
            if l[k] <= stop_px: x, exit_px = k, stop_px; break
        yield trade(sym, t, x, px, exit_px, "bo")

def s_overnight(sym, a, regime=None):
    o, c = a["open"], a["close"]
    for t in range(200, N - 1):
        if regime == "spy200" and not SPY200[t]: continue
        if regime == "spy50" and not SPY50[t]: continue
        yield trade(sym, t, t + 1, c[t], o[t + 1], "on")

def s_bearbounce(sym, a, hold=2, drop=0.02):
    c = a["close"]
    for t in range(60, N - 1):
        if SPY50[t]: continue
        if c[t] / c[t - 1] - 1 < -drop:
            x = min(t + hold, N - 1); yield trade(sym, t, x, c[t], c[x], "bb")

def intraday_drift():
    """section 4: open-to-close return per name, 2022-2026, annualised Sharpe, per year, regime split."""
    out = {}
    i0 = IDX[next(d for d in DATES if d >= T0)]
    for s in NAMES:
        d, a = DATA[s]; r = a["close"][i0:] / a["open"][i0:] - 1; yrs = np.array([x[:4] for x in DATES[i0:]])
        reg = SPY200[i0 - 1:N - 1]  # regime known at the prior close
        sh = lambda x: round(float(x.mean() / x.std() * math.sqrt(252)), 2) if len(x) > 20 and x.std() > 0 else None
        out[s] = {"sharpe": sh(r), "mean_bps": round(1e4 * float(r.mean()), 1), "by_year": {y: sh(r[yrs == y]) for y in YEARS},
                  "spy>200": sh(r[reg]), "spy<200": sh(r[~reg]), "pos_years": int(sum(1 for y in YEARS if (sh(r[yrs == y]) or 0) > 0))}
    return out

def main():
    R = {}
    for label, syms in (("four", FOUR), ("universe", NAMES)):
        for hold in (3, 5, 10):
            R[f"a_pead_h{hold}_{label}"] = table(run(s_pead, syms, hold=hold), label)
        R[f"a_pead_h5_gap5_{label}"] = table(run(s_pead, syms, hold=5, thresh=0.05), label)
        for hold in (3, 5):
            R[f"b_rev_h{hold}_{label}"] = table(run(s_reversal, syms, hold=hold), label)
            R[f"b_rev_h{hold}_noregime_{label}"] = table(run(s_reversal, syms, hold=hold, regime=False), label)
        R[f"b_rev_hi5_{label}"] = table(run(s_reversal, syms, mode="hi5"), label)
        R[f"b_rev_hi5_noregime_{label}"] = table(run(s_reversal, syms, mode="hi5", regime=False), label)
        for kind in ("52w", "20d"):
            for hold in (5, 10):
                R[f"c_bo_{kind}_h{hold}_{label}"] = table(run(s_breakout, syms, hold=hold, kind=kind), label)
                R[f"c_bo_{kind}_h{hold}_noregime_{label}"] = table(run(s_breakout, syms, hold=hold, kind=kind, regime=False), label)
        for reg in (None, "spy200", "spy50"):
            R[f"d_overnight_{reg or 'uncond'}_{label}"] = table(run(s_overnight, syms, regime=reg), label)
        for hold in (1, 2, 3):
            R[f"e_bearbounce_h{hold}_{label}"] = table(run(s_bearbounce, syms, hold=hold), label)
    R["drift"] = intraday_drift()
    R["earnings_source"] = {s: ("edgar" if EARN[s][1] else "gap-proxy") for s in NAMES}
    R["earnings_days_per_name"] = {s: len([i for i in EARN[s][0] if DATES[i] >= T0]) for s in NAMES}
    json.dump(R, open(os.path.join(ROOT, "results_daily.json"), "w"), indent=1)
    # markdown dump
    L = ["# daily simulation results (sim_daily.py)", "", "costs 10 bps round trip, $10k/trade, one position per name, 2022-01-01..2026-09-25", ""]
    cols = ["trades", "pnl", "win%", "pf", "avg_bps", "maxdd", "worst", "avg_hold"]
    for k, v in R.items():
        if k in ("drift", "earnings_source", "earnings_days_per_name"): continue
        L += [f"## {k}  (trades/yr {v['trades_per_year']})", "", "| slice | " + " | ".join(cols) + " |", "|" + "---|" * (len(cols) + 1)]
        for sl in ["all"] + YEARS + ["spy>200", "spy<200"]:
            st = v[sl]; L.append(f"| {sl} | " + " | ".join(str(st.get(c, "")) for c in cols) + " |")
        L.append("")
    L += ["## intraday open-to-close drift 2022-2026 (annualised Sharpe of daily o->c)", "", "| sym | sharpe | mean bps | " + " | ".join(YEARS) + " | spy>200 | spy<200 | +years |", "|" + "---|" * (7 + len(YEARS))]
    for s, v in sorted(R["drift"].items(), key=lambda kv: -(kv[1]["sharpe"] or -9)):
        L.append(f"| {s} | {v['sharpe']} | {v['mean_bps']} | " + " | ".join(str(v['by_year'][y]) for y in YEARS) + f" | {v['spy>200']} | {v['spy<200']} | {v['pos_years']} |")
    open(os.path.join(ROOT, "results_daily.md"), "w").write("\n".join(L) + "\n")
    # console summary
    print(f"{'strategy':38s} {'n':>5s} {'n/yr':>6s} {'pnl':>8s} {'win%':>5s} {'pf':>5s} {'bps':>6s} {'mdd':>7s} | >200 pf/n  <200 pf/n")
    for k, v in R.items():
        if k in ("drift", "earnings_source", "earnings_days_per_name"): continue
        a, u, dn = v["all"], v["spy>200"], v["spy<200"]
        print(f"{k:38s} {a['trades']:5d} {v['trades_per_year']:6.1f} {a.get('pnl',0):8d} {a.get('win%',0):5.1f} {a.get('pf',0):5} {a.get('avg_bps',0):6} {a.get('maxdd',0):7d} | {u.get('pf','-')}/{u['trades']}  {dn.get('pf','-')}/{dn['trades']}")
if __name__ == "__main__": main()
