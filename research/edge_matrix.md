# edge matrix: anomaly × instrument class

Companion to `docs/edge_catalog.md` (full citations/mechanisms there; this file is the lookup
table + pretest priority list). Columns: **IDX** = index ETF SPY/QQQ, **SEC** = sector ETFs,
**LEV** = leveraged ETFs, **MEGA** = our 4 mega-cap tech names, **STOCK** = broad liquid single
stocks, **BOND** = bond/gold ETFs. Codes: `home` (documented instrument, replicate here first) ·
`plausible` (mechanism should transfer) · `unlikely` (mechanism says no) · `dead` (tested there
correctly, failed — cited) · `done` (tested there correctly, passed/live — cited) · `wrong-inst`
(only tested on the wrong instrument — cited) · `pass-pretest` (cleared its pre-registered stage-0
kill bar in `research/pretest/`; an engine study is the lead's call) · `needs-product` (only an overnight /
multi-day version clears the bar — the engine is intraday-only).

| # | anomaly | IDX | SEC | LEV | MEGA | STOCK | BOND |
|---|---|---|---|---|---|---|---|
| 1 | Opening-range breakout | dead¹⁸ | plausible | plausible | unlikely | unlikely | unlikely |
| 2 | Noise-area / intraday momentum | done¹ | plausible | plausible | wrong-inst² | unlikely | unlikely |
| 3 | Gao first/last half-hour momentum | dead³ | plausible | plausible | wrong-inst⁴ | unlikely | unlikely |
| 4 | Hedging-demand intraday momentum | home | plausible | plausible | unlikely | unlikely | plausible |
| 5 | End-of-day reversal (single stocks) | unlikely | unlikely | unlikely | dead⁵ | dead⁵ | unlikely |
| 6 | HKS half-hour periodicity | plausible | plausible | unlikely | dead¹⁸ | home | unlikely |
| 7 | Overnight/intraday tug of war | plausible | plausible | unlikely | home⁶ | home⁶ | plausible |
| 8 | Knuteson overnight drift (unverified) | home⁷ | plausible | unlikely | unlikely | unlikely | unlikely |
| 9 | Pre-FOMC announcement drift | dead¹⁸ | plausible | plausible | plausible | plausible | unlikely⁸ |
| 10 | FOMC-day vol compression / 0DTE | home⁹ | unlikely | unlikely | unlikely | unlikely | unlikely |
| 11 | Closing-auction price pressure | plausible | plausible | plausible | home¹⁰ | home¹⁰ | unlikely |
| 12 | LETF/dealer-gamma close rebalancing | home | plausible | home | plausible | unlikely | plausible |
| 13 | Leveraged VIX-product flows | plausible | unlikely | home¹¹ | unlikely | unlikely | unlikely |
| 14 | PEAD | unlikely | unlikely | unlikely | done¹² | done¹² | unlikely |
| 15 | Earnings-day gap reversal | unlikely | unlikely | unlikely | pass-pretest¹³ ¹⁸ | home¹³ | unlikely |
| 16 | Gap fade / gap fill (low rigor) | home | plausible | plausible | home | home | unlikely |
| 17 | Turn-of-month flows | needs-product¹⁸ | plausible | unlikely | plausible | plausible | plausible |
| 18 | Index inclusion / rebalance | unlikely | unlikely | unlikely | n/a¹⁴ | home (decayed) | unlikely |
| 19 | 0DTE / gamma pinning (unverified) | home⁹ | unlikely | unlikely | unlikely | unlikely | unlikely |
| 20 | Lunch-hour volume trough | plausible | plausible | plausible | done¹⁵ | plausible | plausible |
| 21 | Monday/weekend effect (decayed) | unlikely | unlikely | unlikely | unlikely | unlikely | unlikely |
| 22 | VWAP intraday mean reversion | unlikely | unlikely | unlikely | dead¹⁶ | home | unlikely |
| 23 | Short-term reversal (1–5 day) | unlikely | unlikely | unlikely | dead¹⁷ | dead¹⁷ | unlikely |
| 24 | Retail-flow / PFOF reversal | unlikely | unlikely | unlikely | home⁹ | home⁹ | unlikely |

¹ `research/index_momentum/2026-10-07_index_intraday_momentum.md`: QQQ afternoon cell passed (candidate `qqq-noise-pm-vol`); SPY full-day leg failed. ² `docs/paper_trading_plan_2026-09.md` §13, `docs/analysis/2026-09-15_second_window_research.md` #3 — proxied through 4 names, dead there; superseded by ¹. ³ same index_momentum study, Gao variant fails SPY/QQQ at cost. ⁴ plan §13 (corr ±0.1 on 4 names). ⁵ `research/eod/README.md`, plan §13.4 — rejected, ~0 net after costs. ⁶ conflicts with the no-overnight-hold constraint for direct use; signal-only. ⁷ unverified/disputed source (see catalog #8); also blocked by no-overnight-hold constraint. ⁸ Lucca & Moench's own null result in Treasuries. ⁹ needs options/OI or tick-level data we lack (see catalog process section) — "home" but pretest is not cheap. ¹⁰ needs a closing-auction imbalance feed we lack. ¹¹ needs VIX futures data we lack. ¹² `research/swing/2026-09-26_multiday_long_research.md` — PEAD hold-10 passed, parked (needs multi-day position support). ¹³ distinct from the earnings-ORB continuation rule tested in `research/earnings/README.md` (parked, different mechanism). ¹⁴ mechanism requires a membership-change event; none of our 4 names has one. ¹⁵ mechanism already baked into `crates/indicators/src/custom/rvol.rs`, not a standalone backtest. ¹⁶ `research/vwap_short/2026-09-27_vwap_anchored_short.md` — fails as specified. ¹⁷ `research/swing/2026-09-26_multiday_long_research.md` — reversal listed dead alongside breakout momentum. ¹⁸ `research/pretest/2026-10-08_pretest_round1.md` (stage-0 python pretest, kill bars below): #9 tradable SPY 09:30→14:00 −2.5 bps/trade, PF 0.78, 1/5 yrs (paper's overnight 24h window PF 2.6 but 3/5 yrs, ≤0 in 2025–26 — decayed); #17 paper 4-session hold passes only on QQQ (PF 1.47, 4/5) and is no better than random 4-session holds, intraday d−1,+1..+3 QQQ +11 bps PF 1.29 4/5 (misses by 0.01; beats random sessions p≈0.02); #6 40-day same-clock signal IC −0.02, −7.2 bps/half-hour net, no filter value on v18 trades; #1 literal 5-min ORB QQQ −2.9 bps PF 0.88 1/5 (gross +3.3 < cost); #15 fade open→close +36 bps PF 1.40 3/5, LOYO +472 — fragile (ex-top-3 trades +9, NVDA-driven).

## top 10 untested-or-wrong-instrument `home` cells, ranked by cheap-pretest-worthiness

Excludes cells already resolved (`dead`/`done`) and #18/#20/#21 (not applicable / already built / decayed-everywhere). Ranked by: data we already have, event frequency, rule simplicity.

1. **#9 Pre-FOMC drift (IDX).** — *pretest 2026-10-08: **dead**¹⁸.* Rule: long SPY/QQQ the ~24h before scheduled FOMC, flat otherwise (Lucca & Moench). Cheapest possible — FOMC dates already in `event_calendar.rs`, ~8 events/yr, only daily or 1m bars needed. Kill: net bps/trade after 3bps+$0.005/share/leg must be positive in ≥4/5 years; PF > 1.3. Expected trades/yr: ~8 (one per meeting).
2. **#17 Turn-of-month flows (IDX).** — *pretest 2026-10-08: **needs-product**¹⁸ (intraday version near-miss).* Rule: long SPY/QQQ last trading day of month through day 3–4 of next month (Ariel; McConnell & Xu). Pure calendar rule, daily bars only. Kill: net bps/trade positive ≥4/5 yrs, PF > 1.3. Expected trades/yr: ~12 (monthly).
3. **#6 HKS half-hour periodicity (MEGA).** — *pretest 2026-10-08: **dead**¹⁸.* Rule: same-clock-half-hour return continuation, 40-trading-day persistence (Heston-Korajczyk-Sadka), tested as a filter/overlay on our 4 names. 1m bars only, pure lag-correlation computation. Kill: predictive R² and net bps/half-hour must clear 3bps+$0.005/leg — the 2026-09-15 memo already flags this as cost-neutral alone, so the bar is "adds value as a filter on an existing window," not standalone PF. Expected signal frequency: every half-hour, every day (filter, not a trade trigger by itself).
4. **#1 Literal 5-min ORB (IDX, QQQ).** — *pretest 2026-10-08: **dead**¹⁸.* Rule: Zarattini & Aziz's exact 5-min-range breakout with their stop/target, on QQQ only (not the noise-area analog already tested). 1m bars only. Kill: net bps/trade after costs positive ≥4/5 yrs, PF > 1.3 (same bar the index_momentum study used). Expected trades/yr: high (daily opportunity, filtered by breakout firing).
5. **#15 Earnings-day gap reversal (MEGA).** — *pretest 2026-10-08: **pass-pretest**, fragile¹⁸.* Rule: fade (not continue) the earnings-day gap, since ~50% of it is non-fundamental (Ben-Rephael). We already have earnings dates (`research/entries/data/earnings_*.txt`) and 1m bars. Kill: net bps/trade positive ≥3/5 yrs (earnings-day sample is thin — same bar `research/earnings/` used), PF > 1.3. Expected trades/yr: ~4/name × 4 names ≈ 16.
6. **#16 Gap fade/fill (MEGA + IDX).** Rule: size-dependent gap continuation/fade filter (no-news gaps fill more). Daily + 1m bars, no new data. Kill: PF > 1.3, net positive ≥4/5 yrs — low prior given the practitioner-only sourcing, so the kill bar should be strict. Expected trades/yr: ~30–50/name (any gap ≥1%).
7. **#7 Overnight/intraday cross-predictability (MEGA, signal-only).** Rule: use yesterday's overnight-return decile to bias today's intraday long/short lean, not as a carried position (Lou-Polk-Skouras). Daily bars, already used in `research/swing/`. Kill: next-session intraday alpha must be positive ≥4/5 yrs net of cost; if not cleanly separable from PEAD (#14), discard. Expected signal frequency: daily.
8. **#4 Hedging-demand intraday momentum (IDX).** Rule: last-30-min momentum conditioned on a dealer-hedging-flow proxy (crude: realized intraday range as a stand-in for hedging demand, since we lack options OI) on SPY/QQQ. 1m bars, approximate proxy only — flag explicitly as approximate. Kill: PF > 1.3, net positive ≥4/5 yrs; if the proxy doesn't distinguish from plain momentum (#3, already dead), discard without an options-data upgrade. Expected trades/yr: daily opportunity, filtered by the proxy threshold.
9. **#10 FOMC-day vol compression (IDX, crude proxy).** Rule: expect compressed realized range 10:30–14:00 ET on FOMC days (no options data — proxy via realized range vs. non-FOMC days). Daily/1m bars, dates from `event_calendar.rs`. Kill: this is a regime-filter candidate, not a standalone trade — useful only if it measurably changes our existing windows' hit rate on FOMC days; if not, discard. Expected trades/yr: ~8 FOMC days.
10. **#19 0DTE / gamma pinning (IDX, crude proxy).** Rule: expect range compression on expiry-heavy days (approximate via days-to-monthly-OpEx as a stand-in for 0DTE positioning, since we lack options data). 1m bars, calendar only. Kill: same regime-filter bar as #9 — must measurably change an existing window's behavior to be worth building; discard rather than pursue a dedicated strategy without options data. Expected relevance: every trading day (0DTE is now daily for SPX/SPY), but the proxy is weak — lowest-confidence entry on this list.

Not ranked (data-blocked, see `docs/edge_catalog.md` process section): #11 (closing-auction imbalance feed), #12's dealer-gamma leg and #13 (VIX futures / options OI), #24 (tick-level retail-flow data). Revisit if/when we add any of those feeds.
