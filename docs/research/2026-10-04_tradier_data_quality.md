# Tradier market data quality for VPIN-grade 1-minute bars (2026-10-04)

Follow-up to `2026-10-03_robinhood_vs_alpaca_data.md`, which flagged Tradier Pro ($10/mo) as
attractive but left its data consolidation **unverified**. Web research only, as of 2026-10-04.
Unverified items are marked as such.

## 1. Data source and consolidation

Tradier's own market-data page states: **"Equities & Options: Consolidated feed from all
exchanges"** ([docs.tradier.com/docs/market-data](https://docs.tradier.com/docs/market-data#/)).
Greeks/IV come from ORATS; indices (NDX/RUT/COMP) are derived, not raw-exchange. No vendor name
(Xignite/dxFeed/Nasdaq Basic/direct SIP) is disclosed, and no separate CTA/UTP/OPRA
professional/non-professional subscriber form was found (unlike Alpaca/IBKR) — Tradier appears
to fold this into brokerage account terms.

**Conflicting secondary claim (low confidence):** one AI-search aggregation asserted Tradier's
real-time tier is "IEX-only, not SIP," tracing only to a generic blog listicle
([bullalert.ai](https://bullalert.ai/blog/best-stock-market-data-apis-2026/)), not any primary
source — contradicting Tradier's own "consolidated feed" language. Unresolved from docs alone —
**this is exactly what §5's empirical test should settle.**

**Account gating (new finding):** real-time data requires an **active Tradier Brokerage
account**, not sold as a standalone data product: *"If you are not a Tradier Brokerage account
holder, we are unable to provide you with any real-time data solution... available only to
active account holders and cannot be redistributed."* (same page). The brokerage account has a
**$0 minimum** ([tradier.com/individuals/pricing](https://tradier.com/individuals/pricing)), so
Pro ($10/mo) is an add-on to a free account — fine for cost, but it means brokerage onboarding
(KYC), not just an API signup.

## 2. Endpoints and cadence

**Streaming:** `wss://ws.tradier.com/v1/markets/events` (HTTP-streaming variant at
`stream.tradier.com/v1/markets/events`) ([docs.tradier.com/reference/websocket-market-data-streaming](https://docs.tradier.com/reference/websocket-market-data-streaming#/)).
Create a session first (`sessionid`, 5 min to connect); **only one session open at a time** per
token; symbol limits unpublished ("ask for what you need, don't abuse the API"). Event types:
`trade`, `quote`, `summary`, `timesale`, `tradex`.

`timesale` is **per-trade/tick**, not a 1-min aggregate (`exch, bid, ask, last, size, date, seq,
flag, cancel, correction, session`; `size` = single trade's shares). `trade`/`tradex` carry
`price, size, cvol` (`cvol` = cumulative session volume). No native 1-minute bar event exists on
the websocket — a consumer must bucket ticks by minute itself.

**REST `markets/timesales`:** `GET /v1/markets/timesales?symbol=&interval=tick|1min|5min|15min&start=&end=&session_filter=open|all`
([docs.tradier.com/reference/brokerage-api-markets-get-timesales](https://docs.tradier.com/reference/brokerage-api-markets-get-timesales);
fields at [docs.tradier.com/docs/timesale](https://docs.tradier.com/docs/timesale#/)). Fields:
`time, timestamp, price, open, high, low, close, volume, vwap`. **No documented start-vs-end
bar-stamping convention and no stated timezone** — sample rows begin at 09:30:00 ET, implying
Eastern wall-clock, start-of-bar, but that's inference not a guarantee (**unverified**, confirm
empirically). History depth: 1min → **20 days** (`session_filter=open`) / **10 days** (`=all`);
5min/15min → 40/18 days. Tick interval: 5 days, open-market only, **not available in sandbox**.

**Daily `markets/history`:** `date, open, high, low, close, volume`
([docs.tradier.com/docs/historical](https://docs.tradier.com/docs/historical#/)), split-adjusted;
intraday timesales split/dividend adjustment is undocumented. **Verify against a known split
before trusting multi-day intraday pulls across it.**

**Rate limits** (`docs.tradier.com/docs/rate-limiting`): Market Data 120/min prod, 60/min
sandbox; Trading (orders) 60/min both; Standard resources (`/accounts`, `/watchlists`) 120
prod/60 sandbox. Per-token, rolling 1-min windows, `X-Ratelimit-*` headers.

**Sandbox vs production:** sandbox data is "constructed from the same feed," delayed the
"industry standard 15 minutes," with full trading API + paper money
([docs.tradier.com/docs/trading](https://docs.tradier.com/docs/trading); corroborated by
[VentureBeat's sandbox-launch coverage](https://venturebeat.com/business/tradier-announces-launch-of-new-free-developer-sandbox-api)).
`timesales`/`history` structure should match production, just delayed; **no streaming in
sandbox**. So §5's volume check needs only a free sandbox token via REST — no funded account or
Pro needed; only real-time streaming/sub-15-min latency needs funded Pro.

## 3. Plans and terms

**Lite** $0/mo ($0.35/trade), **Pro** $10/mo ($0 commissions, desktop app, real-time data via the
brokerage account per §1; 2-months-free promo on
[tradier.com/individuals/tradier-pro](https://tradier.com/individuals/tradier-pro)), **Pro Plus**
$35/mo (lower index-option/futures commissions, same data tier as Pro). Brokerage minimum **$0**;
futures $500. API access requires a brokerage account holder, Partner, or Advisor — **no pure
data-only subscription exists**, unlike Alpaca/Polygon/Databento. No non-professional
attestation page distinct from account opening was found; redistribution is explicitly
disallowed. No automated-trading restriction found beyond standard ToS; a search snippet claims
"APIs are entitled for personal use only unless you are a Tradier Partner" — **unverified**, but
consistent with the redistribution language.

## 4. Reliability

`status.tradier.com` shows real 2026 incidents: **Market Data Latency and Returns** on
2026-07-28 (**5h10m**); elevated API errors on 2026-09-22, 2026-09-28, plus several in August
(08-18, 08-24, 08-31); a paper-trading issue mid-August. All "operational" at search time
([status.tradier.com](https://status.tradier.com/), aggregated at
[statusgator.com/services/tradier](https://statusgator.com/services/tradier)) — rougher than
anything found for Alpaca previously, worth weighing against the $10/mo price. Bar-stamping,
extended-hours handling, and intraday split handling are all undocumented or weakly implied (§2)
and need empirical confirmation before Tradier could feed VPIN in production.

## 5. Empirical plan (sandbox token, no funding needed)

Compare Tradier per-minute `volume` against local caches (`data/bars/<SYM>.csv` = SIP-consolidated,
`data/bars_iex/<SYM>.csv` = IEX single-venue) for the same RTH session, NVDA/AAPL/AMZN/MSFT/SPY.
Get a free sandbox token (no funding needed); base URL `https://sandbox.tradier.com/v1`.

```bash
for SYM in NVDA AAPL AMZN MSFT SPY; do
  curl -sG "https://sandbox.tradier.com/v1/markets/timesales" \
    --data-urlencode "symbol=$SYM" --data-urlencode "interval=1min" \
    --data-urlencode "start=2026-10-02 09:30" --data-urlencode "end=2026-10-02 16:00" \
    --data-urlencode "session_filter=open" \
    -H "Authorization: Bearer $TRADIER_SANDBOX_TOKEN" -H "Accept: application/json" \
    > "/tmp/tradier_${SYM}.json"
done
```

```python
import json, csv, statistics as st  # compare_volume.py: ratio of Tradier volume to SIP/IEX, per minute/symbol
def load_tradier(sym):
    bars = json.load(open(f"/tmp/tradier_{sym}.json"))["series"]["data"]  # inspect raw JSON; key may differ
    return {b["time"][11:16]: b["volume"] for b in bars}
def load_local(path):
    return {r["timestamp"][11:16]: float(r["volume"]) for r in csv.DictReader(open(path))}
for sym in ["NVDA", "AAPL", "AMZN", "MSFT", "SPY"]:
    tr, sip, iex = load_tradier(sym), load_local(f"data/bars/{sym}.csv"), load_local(f"data/bars_iex/{sym}.csv")
    rs = [tr[t] / sip[t] for t in tr if sip.get(t)]
    ri = [tr[t] / iex[t] for t in tr if iex.get(t)]
    print(sym, "vs SIP:", round(st.median(rs), 3) if rs else None, "vs IEX:", round(st.median(ri), 3) if ri else None)
```

**Interpretation:** ratio **≈ 1.0 vs SIP** → consolidated (confirms docs, refutes "IEX-only").
Ratio **≈ IEX/SIP's small known fraction vs SIP but ≈ 1.0 vs IEX** → "consolidated" is marketing
only; do not use for VPIN. Check minute-alignment first (a one-minute shift corrupts any ratio)
and diff `session_filter=all` vs `open` against the RTH-filtered caches to confirm extended
hours are excluded cleanly.

## Verdict

**Tentatively promising, not yet trustworthy.** If Tradier's "consolidated feed from all
exchanges" claim holds, it's the cheapest consolidated source found yet ($10/mo, no funded
minimum, undercutting Alpaca Algo Trader Plus's $99/mo 10x), with a free sandbox for zero-cost
testing. But: (a) a credible-sounding "IEX-only" contradiction is only resolved by §5; (b) 2026
status history shows a 5-hour market-data latency incident and several elevated-error weeks —
worse reliability optics than Alpaca; (c) bar-stamping, timezone, and intraday split-adjustment
are undocumented, each a real correctness risk; (d) data is bundled to a brokerage account, not
a pure data API — a second KYC relationship, not just an API key. **If §5 confirms ≈1.0 vs SIP:**
worth a real trial — run a `TradierFeed` alongside `AlpacaFeed` as a side-by-side shadow before
trusting it live.

**Integration shape**, mirroring `crates/data_feed/src/alpaca_feed.rs`'s `AlpacaFeed` (wss
client, 1-min bar subscription, reconnect-with-backoff, `fetch_historical_bars` REST backfill): a
new `crates/data_feed/src/tradier_feed.rs` `TradierFeed` (token + symbols) emitting the same
`BarEvent`s into the `mpsc` channel `CandleAggregator` consumes. Tradier's websocket gives
ticks, not native bars, so this needs its **own minute-bucketing layer** — materially bigger
than a drop-in swap. `fetch_historical_bars` would map to `timesales?interval=1min`, capped at
20/10 days (§2) — not a replacement for Alpaca's SIP history to 2016 (`data/bars/`), only a
live/warm-start candidate.

## 2026-10-07 follow-up: is the feed consolidated?

Web research only, no account opened, settling whether Tradier retail equity data is SIP
(CTA+UTP) consolidated or single-venue (IEX/Nasdaq Basic/Cboe One).

| source | type | what it says | date |
|---|---|---|---|
| [docs.tradier.com/docs/market-data](https://docs.tradier.com/docs/market-data#/) | primary (vendor docs) | "Real-time data is pulled from a consolidated feed from all exchanges." Gated to brokerage account holders; non-holders get no real-time solution; delayed = same feed, 15-min lag. | undated (fetched 2026-10-07) |
| [docs.tradier.com/docs/exchange-codes](https://docs.tradier.com/docs/exchange-codes#/) | primary (API reference) | `exch`/`bidexch`/`askexch` code table covers ~23 venues: NYSE (N), Nasdaq OMX (Q), NYSE Arca (P), BATS (Z), BATS Y (Y), Direct Edge A/X (J/K), CBOE (W), Chicago (M), NSX (C), ISE (I), LTSE (L), Nasdaq PSX (X), OTCBB/OTC (U/V), FINRA ADF (D), plus a SIP-synthetic "Market Independent (Generated by Nasdaq SIP)" code (E). This is the full venue set a consolidated SIP would emit, not one venue's codes. | undated (fetched 2026-10-07) |
| [docs.tradier.com/reference/brokerage-api-markets-get-quotes](https://docs.tradier.com/reference/brokerage-api-markets-get-quotes) | primary (API reference) | Sample AAPL quote: `exch="Q"` (last, Nasdaq), `bidexch="Z"` (BATS), `askexch="Q"`. Bid/ask/last sourced from *different* venues within one quote — the signature of an NBBO computed across venues, not a single reporting venue's top-of-book. | undated (fetched 2026-10-07) |
| [docs.tradier.com/docs/timesale](https://docs.tradier.com/docs/timesale#/) | primary (API reference) | REST `timesales` 1min/5min/15min fields (`time, timestamp, price, open, high, low, close, volume, vwap`) carry **no `exch` field** at bar granularity — so the bar-level `volume` can't be checked for venue mix from the docs alone; tick-level (`interval=tick`) exposes only `time, timestamp, price, volume`, also no `exch` in this page's rendering (contradicts the streaming `timesale` event's documented `exch` field found 2026-10-04 — possibly a doc-rendering gap, not a real absence). | undated (fetched 2026-10-07) |
| bullalert.ai blog listicle (already flagged 2026-10-04) | secondary, low quality | Claims Tradier real-time is "IEX-only, not SIP." No citation. | 2026 (undated post) |
| mylinedchart.com, "IBKR vs Alpaca vs Tradier" | secondary, low quality | Describes Tradier data as "bundled with the brokerage account" but explicitly **declines to state** Tradier's consolidation method, telling readers to "confirm all three directly with the broker." Does **not** repeat or corroborate the IEX-only claim. | 2026-10-07 (page's own "last updated") |
| targeted search for "Tradier" + "IEX" | negative result | No primary or secondary source ties Tradier's real-time equity feed to IEX specifically; the only hits are unrelated tools that support *both* IEX Cloud and Tradier as separate, independent data sources. | searched 2026-10-07 |
| targeted search for "Tradier" on nasdaqtrader.com / CTA-UTP vendor pages | inconclusive | Found Tradier Brokerage listed in Nasdaq's general firm directory (ticker symbols TBIS/TBIA), not a UTP/CTA market-data vendor roster; no vendor-agreement page located. | searched 2026-10-07 |
| QuantConnect Tradier dataset docs | secondary (integrator docs) | Describes Tradier as a live-trading price source via its Market Data API/websockets; does not characterize consolidation. | fetched 2026-10-07 |

**Verdict: consolidated (SIP-grade), confidence moderate-high — but the specific "does bar-level `volume` include TRF/off-exchange prints" sub-question is still unverified.**

The vendor's own docs state consolidation plainly, and two independent, non-marketing artifacts corroborate it: an exchange-code table spanning essentially every CTA/UTP-tape venue plus FINRA ADF, and a live quote example mixing bid/ask/last across different venues within one symbol — neither is explicable by a single-venue feed. The contradicting "IEX-only" claim remains traceable to exactly one uncited blog listicle; a fresh, targeted search today found no other source repeating or supporting it, and a second broker-comparison blog that discusses Tradier explicitly refuses to guess at its consolidation method rather than asserting IEX. That asymmetry (one unsupported claim vs. a vendor statement plus two independent corroborating artifacts) is why confidence is moderate-high rather than just "vendor says so."

What's *not* settled from docs alone: the REST `timesales` bar response (the endpoint this system would actually poll for 1-min `volume`) exposes no `exch` field, so there's no way to confirm from documentation whether its `volume` folds in FINRA TRF-reported off-exchange prints (the bulk of off-exchange volume — ADF carries a small fraction) or only on-exchange prints. Tradier's own "consolidated feed" language is stated for real-time data broadly, not specifically scoped to the `volume` field of 1-min bars.

**Single remaining check that would settle it without a *funded* account:** the §5 empirical script already written in this doc — a **free, unfunded Tradier sandbox token** (sandbox registration is lighter than full brokerage KYC, though it is still an account of some kind) pulling `timesales?interval=1min` for AAPL/AMZN/MSFT/NVDA/SPY and ratio-checking against the local SIP-consolidated bar cache. Ratio ≈ 1.0 closes the remaining gap definitively; a ratio matching IEX's small share of consolidated volume would overturn today's verdict.
