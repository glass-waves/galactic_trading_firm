#!/usr/bin/env python3
"""fetch daily bars 2021-06-01..2026-09-25 (split-adjusted, SIP) for the swing universe + SPY.
caches to research/swing/daily/<SYMBOL>.csv; skips symbols already cached.
rate limit: <=50 symbols per request, 0.5 s sleep between requests."""
import os, csv, time, sys, requests
ROOT = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(ROOT, "daily")
ENV = os.path.join(ROOT, "..", "..", ".env")
UNIVERSE = ("AAPL MSFT AMZN NVDA GOOGL META TSLA AVGO LLY JPM V MA UNH COST HD PG KO PEP XOM CVX MRK ABBV "
            "ADBE CRM NFLX AMD ORCL QCOM TXN INTC CSCO AMAT MU BAC WFC GS MS CAT DE BA GE HON UNP UPS LOW "
            "NKE SBUX MCD DIS CMCSA T VZ PFE TMO ABT DHR LIN NEE PLD SPY").split()
import datetime as _dt
START = "2021-06-01"
END = (_dt.date.today() - _dt.timedelta(days=1)).isoformat()  # extend to yesterday on every run

def load_env():
    kv = {}
    for line in open(ENV):
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line: continue
        k, v = line.split("=", 1); kv[k] = v.strip().strip('"').strip("'")
    return kv

def main():
    env = load_env()
    hdr = {"APCA-API-KEY-ID": env["APCA_API_KEY_ID"], "APCA-API-SECRET-KEY": env["APCA_API_SECRET_KEY"]}
    # incremental: a cached symbol is re-fetched from the day after its last row; a missing one from START
    def last_date(sym):
        fp = os.path.join(OUT, sym + ".csv")
        if not os.path.exists(fp): return None
        last = None
        for row in csv.reader(open(fp)):
            if row and row[0] != "date": last = row[0]
        return last
    starts = {}
    for sym in UNIVERSE:
        ld = last_date(sym)
        if ld is None: starts[sym] = START
        elif ld < END: starts[sym] = (_dt.date.fromisoformat(ld) + _dt.timedelta(days=1)).isoformat()
    todo = sorted(starts)
    print("to fetch:", len(todo), "up to date:", len(UNIVERSE) - len(todo), "end:", END)
    for i in range(0, len(todo), 50):
        chunk = todo[i:i + 50]
        bars = {s: [] for s in chunk}
        params = {"symbols": ",".join(chunk), "timeframe": "1Day", "start": min(starts[s] for s in chunk) + "T00:00:00Z",
                  "end": END + "T23:59:59Z", "adjustment": "split", "feed": "sip", "limit": 10000}
        token = None; pages = 0
        while True:
            if token: params["page_token"] = token
            for attempt in range(5):
                r = requests.get("https://data.alpaca.markets/v2/stocks/bars", headers=hdr, params=params, timeout=60)
                if r.status_code == 429: time.sleep(10); continue
                r.raise_for_status(); break
            j = r.json(); pages += 1
            for s, rows in (j.get("bars") or {}).items():
                bars[s].extend(rows)
            token = j.get("next_page_token")
            print(f"  page {pages}: {sum(len(v) for v in bars.values())} bars so far, token={'y' if token else 'n'}", flush=True)
            time.sleep(0.5)
            if not token: break
        for s, rows in bars.items():
            fp = os.path.join(OUT, s + ".csv")
            existing = {}
            if os.path.exists(fp):
                for row in csv.reader(open(fp)):
                    if row and row[0] != "date": existing[row[0]] = row
            for b in rows:
                d = b["t"][:10]
                if d >= starts[s]:
                    existing[d] = [d, b["o"], b["h"], b["l"], b["c"], b["v"], b.get("vw", ""), b.get("n", "")]
            with open(fp, "w", newline="") as f:
                w = csv.writer(f); w.writerow(["date", "open", "high", "low", "close", "volume", "vwap", "n"])
                for d in sorted(existing): w.writerow(existing[d])
            print(f"  {s}: +{len(rows)} bars -> {len(existing)} rows", flush=True)
if __name__ == "__main__": main()
