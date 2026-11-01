import sys, csv, random
from collections import defaultdict
sys.path.insert(0, "/home/dylmet/Projects/galactic_trading_firm/scripts/pipeline")
import metrics as pm
IDX3 = ["SPY", "QQQ", "IWM"]; ETF4 = ["XLK", "XLF", "XLE", "SMH"]; MEGA4 = ["AAPL", "AMZN", "MSFT", "NVDA"]
POOL34 = ("AAPL ABBV ADBE AMAT AMD AMZN AVGO CAT COIN COST CRM CVX GS HD JPM LLY MA META MRK MSFT MU NFLX NVDA "
          "ORCL PG PLTR QCOM SHOP TSLA TXN UBER UNH V XOM").split()
SUBS = {"SPY": ["SPY"], "QQQ": ["QQQ"], "IDX3": IDX3, "ETF4": ETF4, "MEGA4": MEGA4, "POOL34": POOL34, "ALL41": IDX3 + ETF4 + POOL34}
def load(path):
    rows = list(csv.DictReader(open(path)))
    for r in rows:
        r["pnl"] = float(r["pnl"]); r["notional"] = float(r["size"]) * float(r["entry_price"])
        r["bps"] = 1e4 * r["pnl"] / r["notional"]
    return rows
def pf(x):
    g = sum(v for v in x if v > 0); l = -sum(v for v in x if v < 0); return g / l if l else 99
def report(path, subs=SUBS, show=True):
    rows = load(path); out = {}
    for name, tk in subs.items():
        rr = [r for r in rows if r["ticker"] in tk]
        if not rr: continue
        b = [r["bps"] for r in rr]
        # daily equal-weight basket return in bps (mean of names that traded that day)
        day = defaultdict(list)
        for r in rr: day[r["date"]].append(r["bps"])
        dmean = {d: sum(v) / len(v) for d, v in day.items()}
        p = pm.sign_flip_p(dmean, n=5000, seed=0)
        yr = defaultdict(list)
        for r in rr: yr[r["date"][:4]].append(r["bps"])
        byname = defaultdict(float)
        for r in rr: byname[r["ticker"]] += r["bps"]
        tot = sum(b); top = max(byname, key=byname.get)
        share = byname[top] / tot if tot > 0 else float("nan")
        exb = [r["bps"] for r in rr if r["ticker"] != top]
        dpf = pf(list(dmean.values()))
        loyo = {y: sum(v for r2 in [0] for v in []) for y in yr}
        out[name] = dict(n=len(rr), days=len(dmean), bps=sum(b) / len(b), pf=pf(b), daypf=dpf, p=p,
                         yrs={y: (round(sum(v) / len(v), 1), len(v)) for y, v in sorted(yr.items())},
                         top=top, share=share, ex_top_pf=pf(exb) if len(tk) > 1 else None,
                         worst_day=min(dmean.values()), best_day=max(dmean.values()))
        if show:
            o = out[name]
            ys = " ".join(f"{y[2:]}:{v[0]:+.1f}" for y, v in o["yrs"].items())
            ext = f" top {top} {share:.0%} exTopPF {o['ex_top_pf']:.2f}" if o["ex_top_pf"] else ""
            print(f"  {name:<7} n {o['n']:>5} days {o['days']:>3} {o['bps']:+6.1f} bps PF {o['pf']:.2f} dayPF {dpf:.2f} p {p:.3f} | {ys} | worst {o['worst_day']:+.0f} best {o['best_day']:+.0f}{ext}")
    return out
if __name__ == "__main__":
    for path in sys.argv[1:]:
        print(path.split("/")[-1]); report(path)
