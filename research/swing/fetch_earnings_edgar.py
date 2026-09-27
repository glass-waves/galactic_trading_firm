#!/usr/bin/env python3
"""earnings announcement dates for the universe = EDGAR 8-K filings with item 2.02 (results of operations),
same method that produced research/entries/data/earnings_<T>.txt (verified identical for AAPL/NVDA).
writes research/swing/earnings/<SYMBOL>.txt (one filing date per line, newest first)."""
import os, json, time, requests
ROOT = os.path.dirname(os.path.abspath(__file__)); OUT = os.path.join(ROOT, "earnings"); os.makedirs(OUT, exist_ok=True)
from fetch_daily import UNIVERSE
HDR = {"User-Agent": "galactic_trading_firm research dylan.whitej@gmail.com", "Accept-Encoding": "gzip, deflate"}
syms = [s for s in UNIVERSE if s != "SPY" and not os.path.exists(os.path.join(OUT, s + ".txt"))]
if not syms: print("all cached"); raise SystemExit
ct = requests.get("https://www.sec.gov/files/company_tickers.json", headers=HDR, timeout=60).json()
cik = {v["ticker"]: int(v["cik_str"]) for v in ct.values()}
for s in syms:
    if s not in cik: print("no cik for", s); continue
    time.sleep(0.15)
    d = requests.get(f"https://data.sec.gov/submissions/CIK{cik[s]:010d}.json", headers=HDR, timeout=60).json()
    r = d["filings"]["recent"]
    dates = [r["filingDate"][i] for i in range(len(r["form"])) if r["form"][i] == "8-K" and "2.02" in (r["items"][i] or "")]
    # older filings live in paged files; pull if the recent block does not reach 2021
    for f in d["filings"].get("files", []):
        if min(dates, default="9999") <= "2021-01-01": break
        time.sleep(0.15)
        rr = requests.get("https://data.sec.gov/submissions/" + f["name"], headers=HDR, timeout=60).json()
        dates += [rr["filingDate"][i] for i in range(len(rr["form"])) if rr["form"][i] == "8-K" and "2.02" in (rr["items"][i] or "")]
    dates = sorted(set(x for x in dates if x >= "2021-01-01"), reverse=True)
    open(os.path.join(OUT, s + ".txt"), "w").write("\n".join(dates) + "\n")
    print(s, len(dates), dates[:3], flush=True)
