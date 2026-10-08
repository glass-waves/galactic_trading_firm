# universe expansion screen — 2026-10-07

full write-up: `research/universe/2026-10-07_universe_expansion.md`. raw numbers:
`research/universe/results.json`. ticker → `cand_<id>` map: `research/universe/candidates.json`.
analysis script: `research/universe/analyze_universe.py`.

goal: find tickers the short book (AAPL/AMZN/MSFT/NVDA, v18 promoted config) could add to trade
more often, without dropping the five-year combined PF below 1.4.

## pre-registered rule (fixed before any combined-book number was computed)

a name qualifies if its own five-year record — under the variant picked for it (v18 first,
thrust-1h15 only as a rescue, policy fixed in advance) — has PF ≥ 1.3, ≥ 4 of 5 years positive,
≥ 60 trades, no year below −300.

## per-name results (36 candidates: 6 already-decided + 30-name screen)

v18 = plain promoted config on that one ticker. 1h15 = thrust-1h15 quality variant (AM-only entry
windows, `vpin_1m.raw_vpin ≥ 0.217`). both at 36% sizing, IEX bars, cross-index SPY, five years
(2022 – partial 2026).

| ticker | v18 PF | v18 n | v18 P&L | v18 yrs+/5 | 1h15 PF | 1h15 n | 1h15 P&L | 1h15 yrs+/5 | qualifies |
|---|---|---|---|---|---|---|---|---|---|
| LLY | 1.14 | 147 | +218 | 3 | 1.04 | 108 | +52 | 4 | no |
| COST | 0.92 | 68 | -56 | 3 | 1.06 | 48 | +27 | 2 | no |
| HD | 0.52 | 92 | -568 | 0 | 0.66 | 63 | -272 | 2 | no |
| XOM | 1.28 | 51 | +131 | 3 | 1.13 | 32 | +43 | 4 | no |
| CVX | 1.43 | 56 | +213 | 4 | 1.24 | 34 | +70 | 2 | no (n<60 both) |
| ADBE | 0.96 | 122 | -70 | 2 | 0.89 | 91 | -127 | 3 | no |
| CRM | 0.87 | 106 | -182 | 2 | 1.11 | 78 | +105 | 3 | no |
| ORCL | 1.04 | 77 | +44 | 3 | 1.16 | 47 | +101 | 1 | no |
| QCOM | 0.66 | 98 | -534 | 1 | 0.62 | 66 | -438 | 1 | no |
| CAT | 1.26 | 92 | +267 | 4 | 1.01 | 68 | +5 | 3 | no (PF) |
| GS | 0.57 | 93 | -564 | 2 | 0.76 | 53 | -165 | 2 | no |
| PG | 0.40 | 21 | -151 | 2 | 0.34 | 14 | -137 | 2 | no |
| ABBV | 0.90 | 41 | -44 | 2 | 0.57 | 30 | -227 | 1 | no |
| MRK | 1.26 | 25 | +90 | 2 | 2.04 | 21 | +208 | 2 | no (n<60 both) |
| TXN | 0.87 | 66 | -95 | 1 | 1.19 | 41 | +70 | 1 | no |
| AMAT | 0.82 | 112 | -326 | 1 | 0.89 | 74 | -138 | 2 | no |
| MA | 0.80 | 85 | -234 | 3 | 0.67 | 53 | -270 | 0 | no |
| UNH | 0.90 | 119 | -130 | 1 | 1.01 | 86 | +10 | 2 | no |
| JPM | 1.03 | 51 | +23 | 3 | 1.34 | 31 | +110 | 1 | no |
| V | 0.64 | 67 | -245 | 2 | 0.76 | 42 | -102 | 1 | no |
| AMD | 0.99 | 158 | -18 | 3 | 1.28 | 115 | +452 | 4 | no (PF 1.28<1.30, close) |
| META | 0.99 | 160 | -19 | 3 | 1.33 | 118 | +401 | 3 | no (yrs+ 3<4, close) |
| QQQ | 1.10 | 67 | +53 | 3 | 0.98 | 22 | -3 | 2 | no |
| SMH | 0.86 | 90 | -141 | 2 | 0.67 | 61 | -238 | 2 | no |
| XLK | 0.65 | 38 | -178 | 2 | 0.67 | 18 | -66 | 1 | no |
| XLF | 0.26 | 2 | -46 | 1 | 0 trades | — | — | — | no |
| IWM | 0.40 | 27 | -241 | 1 | 0.61 | 14 | -78 | 1 | no |
| XLE | 0.57 | 43 | -204 | 1 | 0.41 | 23 | -172 | 1 | no |
| TSLA | 0.85 | 223 | -569 | 1 | 0.84 | 168 | -485 | 1 | no |
| AVGO | 0.73 | 143 | -455 | 2 | 0.42 | 104 | -819 | 1 | no |
| NFLX | 1.01 | 128 | +24 | 2 | 0.84 | 93 | -206 | 2 | no |
| COIN | 0.74 | 217 | -1307 | 0 | 0.69 | 179 | -1369 | 0 | no |
| PLTR | 1.06 | 160 | +204 | 2 | 0.96 | 122 | -97 | 3 | no |
| MU | 0.87 | 139 | -317 | 2 | 0.92 | 105 | -152 | 1 | no |
| SHOP | 1.298 | 113 | +622 | 4 | 1.18 | 79 | +255 | 3 | no (PF 1.298<1.3 *and* min yr -445<-300) |
| UBER | 0.79 | 84 | -305 | 1 | 0.68 | 65 | -389 | 2 | no |

(min-year and exact PF/trade-count detail for every row is in `results.json` / the full write-up;
trimmed here to fit the table.)

## data gap found and fixed

`candidates.json` had `null` instead of `cand_30`…`cand_43` for the 14 new tickers even though
their v18 sweeps were already on disk, so `analyze_universe.py` was silently scoring them on
thrust-1h15 only. Filled in the real tags from `phase_direct.sh`'s `IDS` map and re-ran — result
unchanged (still zero qualifiers), but all 36 names are now actually checked against v18 first,
as the pre-registered variant policy requires.

## qualifying names

**none.** 0 of 36. Confirmed independently by `pipeline.py list`: all 36 ticker candidates show
`backtest_failed` under the `default-ticker` gate.

## combined book vs v18

Identical to baseline — no qualifiers to add:

- v18 baseline (AAPL/AMZN/MSFT/NVDA): **+1,728 / 436 trades / PF 1.37** · 2022:+991 2023:-3 2024:-20 2025:+332 2026:+429
- combined (base 4 + qualifiers): **same** — +1,728 / 436 / PF 1.37

## concurrency check

Run on the (unchanged) base-4 book. 325 trading days had a position open; **3 of them**
(2023-04-10, 2024-01-02, 2024-07-17) had **4 positions open at once**, exceeding the live
3-position cap — a pre-existing property of the current book, not caused by this screen (there
was nothing to add). Worth a separate look at how the live engine actually handles the 4th
concurrent signal on those days; out of scope here.

## leave-one-year-out (LOYO)

Re-ran the selection rule 5 times, once per held-out year, using only the other 4 years' trades
per candidate. **Every fold selected the empty set** — no candidate clears even the relaxed
training-set version of the rule in any 4-year window. The held-out-year numbers reported are
therefore just the base-4 book's own per-year P&L (991 / -3 / -20 / 332 / 429), a consistency
check on the baseline rather than a test of a (nonexistent) selection.

## verdict

**No expansion — nothing to propose.** Zero of 36 screened tickers qualify under the
pre-registered rule on either v18 or thrust-1h15. The combined book is unchanged from v18
(+1,728/436/PF 1.37), so no `universe_v1.patch.json` was written and no
`pipeline.py propose` call was made.

The closest misses: SHOP (v18 PF 1.298 — just under 1.3 — with 4/5 years positive and 113 trades,
but one catastrophic year of -445 sinks it on the floor check too), AMD and META on thrust-1h15
(PF 1.28<1.30, and years+ 3/5<4, respectively — one check each away). CVX and MRK clear PF/years
comfortably but never reach 60 trades on either variant. No candidate misses on only one axis in
a way a small rule tweak would plausibly fix without reopening the pre-registered rule itself.

Recommendation: stop screening single-name ticker additions under v18/thrust-1h15 as currently
defined — AAPL/AMZN/MSFT/NVDA remain a meaningfully stronger, already-selected set than anything
in this mega-cap/ETF/high-beta screen. If more trade volume is wanted, a new window/config idea
(e.g. the `spy-sqrt-band` / `spy-sqrt-band-1h15` candidates proposed 2026-10-07) is a more likely
lever than a wider ticker universe.

## notes on how this screen was run (provenance, not part of the pre-registered rule)

- v18 gate sweeps for all 30 screen tickers were produced by a direct driver
  (`research/universe/phase_direct.sh`) that fetches IEX bars and runs the backtest sweep itself
  under a blocking `flock -w 14400` on `logs/.research.lock`, then calls
  `pipeline.py backtest ticker:<T>` to gate the already-swept result — a workaround for a
  lock-starvation bug in `pipeline.py`'s own (non-blocking, polling) lock, not a change to
  `pipeline.py` itself.
- thrust-1h15 variant sweeps (`uq_<ticker>`) ran under a **second** lock lane
  (`logs/.research.lock.b`), one ticker at a time, so they didn't queue behind the main sweep —
  this second lane was a session-lead decision made to speed up this screen, not something the
  system owner had approved in advance.
