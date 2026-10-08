# pretest round 2 — matrix items 6-10 (2026-10-08)

Stage-0 pretests of ranked-list items 6-10 (#16, #7, #4, #10, #19 in `docs/edge_catalog.md`). Same
harness/cost model as round 1. Per the standard change: a cell is called dead only after scanning a
small pre-registered neighbourhood — holding window / threshold-or-decile cut (2-3 values each),
time-of-day where relevant, and the home-class instrument set (SPY+QQQ+IWM for index cells; the 4
names + the other ~30 large caps in `data/bars_iex` for single-stock cells). Every grid point for
every instrument is in `results/c0[6-9]*.json` / `c10_*.json` (not just the primary point).
Finer verdicts used below: **dead** (no gross edge anywhere in the scan), **sub-cost** (a gross
edge exists somewhere, net < cost), **decayed** (positive early years, negative in the last two),
**fragile** (passes but concentrated), **pass** (clears the bar, or — for 9/10 — the regime is
measurable). Nothing proposed to `scripts/pipeline/`.

## 6. #16 gap fade/fill, size-dependent — 45 cells scanned (3 exits x 3 thresholds x 5 instruments: 4
names, SPY, QQQ, IWM, 34-name pool) — **sub-cost**
Rule: `|gap| < thresh` fades, `>= thresh` continues; earnings days excluded where we have a
calendar (COIN/PLTR/SHOP/UBER have none, treated as unexcluded like the ETFs).

| best by | config | n | gross bps | net bps | PF | yrs+ |
|---|---|---|---|---|---|---|
| PF | 4 names, thresh=1%, exit close | 4687 | - | -5.6 | 0.92 | 1/5 |
| net bps | IWM, thresh=2%, exit close | 1190 | +2.2 | -4.3 | 0.91 | 1/5 |

17/45 cells have a positive *gross* edge (mostly IWM and the 1% threshold), but none clear even
half the ~6 bps round-trip cost, and the 34-name pool (40,368 trades) is flat-to-negative
everywhere, 0/5 years on every exit. No decay pattern (IWM's best cell is worst in 2022, flips
positive in 2025) and no single symbol or window is close to the bar — nothing in the 45-cell scan
reaches PF 1.1 on n>=1000. **Verdict: sub-cost at best** (small positive gross on IWM/low-threshold
cuts, eaten by cost everywhere else and in aggregate) — mechanically **dead** against the
pre-registered bar (0-2/5 years positive, best PF 0.92).

## 7. #7 overnight-return decile lean — 18 cells scanned (3 cuts: 5%/10%/20% x 3 windows: 20/60/120
sessions x 2 instruments: 4 names, 34-name pool) — **dead**
Rule: yesterday's close->open return's trailing-window percentile; top decile -> short lean,
bottom -> long lean, open->close, signal-only.

Best cell: 4 names, 20% cut, 20-session window: -7.6 bps, PF 0.90, 1/5 years. **0 of the 18 cells
have a positive gross edge** (best gross is -1.1 bps) — this is dead at the mechanism level, not a
cost problem. The 34-name pool loses more (-12 to -21 bps) at every cut and window, so it is not a
small-sample artifact of the 4 names. No decay: the worst year is 2022 and results improve toward
2026 (mildly positive on the widest cut, still net-negative) — if anything the opposite of decay.
The predicted sign (short after an overnight winner) is simply the wrong sign to trade here, at
every width and lookback tried. **Verdict: dead** (no gross edge anywhere in the scan).

## 8. #4 hedging-demand last-N-min momentum — 18 cells scanned (gate in {50,60,70}% x hold in
{20,30,45} min x 3 instruments: SPY, QQQ, IWM) — **dead**
Rule: sign of the 09:30->(close-hold) return in the last `hold` minutes, gated on realized range
`(high-low)/open` over the same span clearing its trailing-percentile (the explicit hedging-demand
proxy, since we lack options OI/order flow).

Best cell: QQQ, ungated, hold=45: -6.3 bps, PF 0.58, 1/5 years. **0 of 18 cells have a positive
gross edge** (best gross -0.03 bps, i.e. breakeven before cost). Gating on the range proxy never
helps — SPY/QQQ/IWM gated PF (0.40-0.55) is uniformly *worse* than ungated (0.45-0.58), the
opposite of what the hedging-demand story predicts. No decay (QQQ ungated is flat-to-negative
2022-2026, not front-loaded). Confirms the matrix's own fallback bluntly: the realized-range proxy
does not distinguish from plain momentum (#3, already dead) at any gate/hold combination tried.
**Verdict: dead**.

## 9. #10 FOMC-day range compression — 9 cells scanned (3 windows x SPY/QQQ/IWM) — **regime
confirmed (pass-pretest)**
Not a trade: realized range `(high-low)/price` over a time-of-day window on the 38 FOMC days vs
all other full sessions.

| window | SPY | QQQ | IWM |
|---|---|---|---|
| 10:00-14:00 | -39% (z -7.6) | -37% (z -7.0) | -31% (z -9.0) |
| 10:30-14:00 (task) | -37% (z -6.9) | -33% (z -5.3) | -29% (z -7.8) |
| 10:30-14:30 | **+4%** (z +0.4) | **+3%** (z +0.3) | **+18%** (z +2.1, *wider*) |

All 3 instruments compress strongly through 14:00 in both of the pre-announcement windows (median
check: 84% of FOMC days fall below the non-FOMC *median* range on SPY/QQQ, so this isn't a few
outlier days). The moment the window extends 30 min past the 14:00 announcement, compression
reverses to expansion (IWM even significantly *wider*) — exactly "compresses into the
announcement, then jumps" per the catalog. **Verdict: pass-pretest, robust across all 3
instruments and both pre-announcement windows; not fragile.** What a strategy would do with it:
gate the FOMC-day entry windows in `c01_pre_fomc` (all of which failed on raw P&L) to expect
quieter 10:30-14:00 action — tighter stops / suppress breakout entries pre-announcement, and
treat 14:00-14:30 as the window where the move actually happens.

## 10. #19 days-to-monthly-OpEx range compression — 9 cells scanned (3 windows x SPY/QQQ/IWM) —
**regime not confirmed (dead)**
Same shape as #9: monthly OpEx (3rd Friday, calendar-computed) vs all other full sessions.

Best cell: IWM, 10:30-14:30: +5.1% compression, z -0.72 — the largest of all 9, still far short of
the 10%/|z|>=2 bar. Every instrument x window combination is in [-0.1%, +5.1%] compression at
|z| <= 0.7; nothing is close to a measurable effect (the next-best after IWM is SPY 10:30-14:30 at
+3.8%, z -0.45). **Verdict: dead as a regime gate.** The one-expiry-a-month proxy is too coarse for
daily 0DTE positioning, as the catalog itself flagged as the lowest-confidence entry — discard
without an options-data upgrade, per the pre-registered bar.

## round result
| cell | scanned | best gross / net bps | best PF | verdict |
|---|---|---|---|---|
| c06 gap fade/fill | 45 | +2.2 / -4.3 | 0.92 | sub-cost (mechanically dead) |
| c07 overnight decile | 18 | -1.1 / -7.6 | 0.90 | dead (no gross edge) |
| c08 hedging momentum | 18 | -0.03 / -6.3 | 0.58 | dead (no gross edge) |
| c09 FOMC compression | 9 | n/a (regime) | n/a | pass-pretest (robust) |
| c10 OpEx compression | 9 | n/a (regime) | n/a | dead (no measurable regime) |

The only finding worth acting on is c09: a real, robust quieting of SPY/QQQ/IWM 10:30-14:00 on FOMC
days that reverses into the announcement — a filter candidate for the already-dead FOMC windows,
not a new strategy. Nothing was proposed to the pipeline.
