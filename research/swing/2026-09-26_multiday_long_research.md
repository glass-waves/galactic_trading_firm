# multi-day long research — 2026-09-26

question: is there a long edge worth building a multi-day (or event-toggled) book around, given that every
intraday long idea on AAPL/AMZN/MSFT/NVDA is flat-to-thin and the owner wants more than 1–2 trades a week?

sources of every number: `sim_daily.py` (daily strategies, results_daily.json/.md), `analyze_daily.py`
(concentration, concurrency caps, sensitivities, results_analysis.json), `controls_daily.py` (unconditional-hold
benchmarks, results_controls.json), `intraday_events.py` (1-minute event angle, results_intraday_events.json).
data: Alpaca SIP daily bars 2021-06-01..2026-09-25, split-adjusted, 59 names + SPY (`fetch_daily.py`, cached in
`daily/`); earnings dates = EDGAR 8-K item 2.02 filing dates (`fetch_earnings_edgar.py`, `earnings/`; identical to
research/entries/data/earnings_<T>.txt for the four names; one date per calendar quarter, largest reaction;
XOM has no 2.02 filings so its quarters use the largest-|gap| day). test window 2022-01-01..2026-09-25 (4.73 yr).
costs 10 bps round trip on every trade; $10k per trade; at most one open position per name; no borrowing cost,
no slippage beyond the 10 bps; regime = SPY close vs its 200-day SMA at the signal close.

## 1. literature / practice (2015–2026, liquid US large caps)

| edge | typical hold | edge after costs, large caps | capacity | decay evidence | regime |
|---|---|---|---|---|---|
| PEAD / earnings-gap continuation (Ball & Brown 1968; Bernard & Thomas 1989; Brandt, Kishore, Santa-Clara, Venkatachalam 2008 EAR) | 5–60 sessions | classic SUE drift is gone in large caps since ~2006 (Martineau 2021 "Rest in Peace PEAD"; Fink 2021 review; Quantpedia: "main contributors are small caps"). what survives in liquid names is the *price* reaction (EAR) over the first 1–2 weeks, low single-digit % annual alpha, ~50–150 bps per event | high (mega-cap events) | strong: arbitrage + faster dissemination; retail-horizon work (arXiv 2512.00280) finds most of the remaining drift accrues days 20–75, i.e. beyond a swing hold | mild; works in both, drawdowns cluster in bear years |
| short-term reversal in an uptrend (Jegadeesh 1990; Lehmann 1990; Connors & Alvarez 2009 RSI(2)) | 2–5 sessions | classic 1-month reversal "has steadily weakened… vanished in most regions" (Blitz, Hanauer, Honarvar, Huisman, van Vliet 2022/FAJ 2023); Da, Liu, Schaumburg 2014: what is left is liquidity provision, and net profits among large caps in the most liquid decade are positive but small; the largest names show short-term *momentum* instead (Medhat & Schmeling 2022). Connors-style 65–75% win rates are gross and pre-2010 heavy | high | costs are the whole story: only cost-measured versions survive | needs uptrend filter (200-day) in every practitioner version |
| 52-week-high / breakout momentum (George & Hwang 2004) | 1–6 months | positive but insignificant 1980–2014, *negative* 2001–2014 (Wang, NZFM); Barroso & Wang 2021: effect limited to small stocks, subsumed by price momentum; momentum crashes (Daniel & Moskowitz 2016) hit breakout books hardest in regime turns | medium | published 2004, decayed after | strongly regime-dependent; long-only version is a beta bet in bull years |
| overnight anomaly (Lou, Polk, Skouras 2019; Boyarchenko, Larsen, Whelan NY Fed 2020 "Overnight Drift"; Bogousslavsky 2021) | 1 night | gross close→open premium is real (SPY 2020Q3–2025Q3: +47% CO vs +30% OC, Cacciatore), concentrated in index futures around the European open. per-trade gross ≈ 1–3 bps on single names; Elm Wealth 2022 ("Night Moves"): even 1 bp round trip costs 5%/yr of the long-short return, "trade less rather than trade at the close"; Alpha Architect: "trading costs wipe out the overnight return anomaly" | very high gross, ~zero net | waning since the 2008–2015 papers | strongest in stress regimes, still net-negative for a 10 bps trader |
| regime filters (Faber 2007/2013 10-month SMA; SPY 200-day; VIX term structure, Simon & Campasano 2014; breadth) | n/a | drawdown insurance, not return: 200-day filter gives up ~3.7 pts/yr of return for ~14 pts less drawdown, shallower worst fall in 82% of 193 tickers (Cestrian, Alvarez) | n/a | whipsaws 2015–2016, 2018, 2023 | it *is* the regime |

net: the only multi-day long effects with credible post-2015 large-cap evidence are (i) the first-two-weeks
earnings *price* reaction and (ii) uptrend-conditioned dip buying, both at 25–100 bps per trade before crowding.
the overnight effect and 52-week-high breakout are not tradeable at retail costs on single names.

## 2. empirical test on daily bars (2022-01-01..2026-09-25)

reading the tables: n = trades, n/yr = trades per year, bps = mean net return per trade, pf = profit factor,
mdd = max drawdown of the $10k/trade equity curve, ">200 / <200" = same trades split by SPY regime at entry.
the unconditional benchmark matters because 2023–2026 was a strong bull tape: **buying any name at the open and
holding 10 sessions returned +40 bps/trade net (PF 1.20) on the universe, +88 bps (PF 1.43) on the four names**
(`controls_daily.py`, uncond_h10). a strategy must beat that, not zero.

### a. PEAD / earnings-gap continuation (buy open of session after a reaction day with c2c > +3%, hold N sessions)

| variant | n | n/yr | pnl $ | win% | pf | bps | mdd | >200 pf/n | <200 pf/n |
|---|---|---|---|---|---|---|---|---|---|
| four, hold 3 | 32 | 6.8 | 2,645 | 56 | 1.84 | 83 | −702 | 1.91/19 | 1.74/13 |
| four, hold 5 | 32 | 6.8 | 3,312 | 69 | 2.01 | 104 | −975 | 2.38/19 | 1.64/13 |
| four, hold 10 | 32 | 6.8 | 6,487 | 63 | 2.06 | 203 | −3,239 | 2.90/19 | 1.42/13 |
| universe, hold 3 | 321 | 67.9 | 9,262 | 51 | 1.21 | 29 | −6,013 | 1.32/230 | 1.01/91 |
| universe, hold 5 | 321 | 67.9 | 12,367 | 52 | 1.25 | 39 | −7,049 | 1.18/230 | 1.40/91 |
| **universe, hold 10** | 321 | 67.9 | 34,326 | 54 | **1.55** | **107** | −15,616 | 1.57/230 | 1.50/91 |
| universe, hold 10, trigger = open gap > +3% | 299 | 63.2 | 40,348 | 57 | **1.78** | 135 | −9,739 | 1.67/211 | 2.03/88 |
| universe, hold 10, c2c > +5% | 201 | 42.5 | 23,777 | 55 | 1.57 | 118 | −10,364 | | |
| universe, hold 10, c2c > +8% | 102 | 21.6 | 19,382 | 55 | 1.92 | 190 | −6,106 | | |
| universe, hold 10 + 5% gap-aware stop | 321 | 67.9 | 26,191 | 50 | 1.41 | 82 | −12,460 | 1.35/230 | 1.55/91 |

per year, universe hold 10 (n / pf / bps): 2022 68 / 1.01 / +2 · 2023 74 / 1.83 / 117 · 2024 66 / 2.19 / 160 ·
2025 59 / 1.93 / 147 · 2026 54 / 1.46 / 118. open-gap trigger per year pf: 1.40 / 2.07 / 3.47 / 1.60 / 1.23.
excess over the unconditional 10-day hold: **+67 bps/trade, positive in every year (+43, +57, +94, +66, +82)** —
this is the one result here that is not just beta. controls: long after a *negative* (< −3%) reaction, hold 10:
PF 1.22, +46 bps (≈ unconditional, so the sign of the reaction matters); long after *any* reaction day: PF 1.41,
+74 bps. hold 3/5 are barely above unconditional (+12/+22 bps); the drift accrues in sessions 6–10.
concentration: 41 of 59 names positive; top-3 names (INTC, LLY, TSLA) = 46% of P&L; ex-INTC PF 1.47, +91 bps.
capacity for a small book (first-come cap on concurrent positions, `analyze_daily.py`): cap 2 → 30 trades/yr
PF 1.53; cap 4 → 45/yr PF 1.28; cap 8 → 61/yr PF 1.46. mean 4.4 open when active, max 16 (earnings weeks).
the four-name book alone: 6.8 trades/yr, 2022 was −162 bps/trade (PF 0.58), everything after 2023 strongly positive.

### b. short-term reversal in an uptrend (3 down closes or 5-day < −4%, name > 50-day, buy close)

| variant | n | n/yr | pnl $ | win% | pf | bps | mdd | >200 pf/n | <200 pf/n |
|---|---|---|---|---|---|---|---|---|---|
| four, hold 3, SPY>200 | 132 | 27.9 | 7,773 | 53 | 1.52 | 59 | −2,934 | 1.52/132 | – |
| four, exit close > prior 5-day high (max 10), SPY>200 | 106 | 22.4 | 8,573 | 65 | 1.48 | 81 | −3,255 | 1.48/106 | – |
| four, hold 5, no regime | 146 | 30.9 | 10,369 | 53 | 1.46 | 71 | −3,319 | 1.39/120 | 1.75/26 |
| universe, hold 3, SPY>200 | 1,590 | 336 | 33,638 | 52 | 1.18 | 21 | −11,767 | 1.18 | – |
| **universe, hold 5, SPY>200** | 1,448 | 306 | 54,969 | 54 | **1.26** | 38 | −17,637 | 1.26 | – |
| universe, hold 5, no regime | 1,812 | 383 | 40,469 | 52 | 1.14 | 22 | −33,604 | 1.27/1437 | 0.79/375 |
| universe, 5-day-high exit, SPY>200 | 1,303 | 275 | 45,006 | 61 | 1.18 | 35 | −25,933 | 1.18 | – |

per year, universe hold 5 with regime: 2022 19 / 0.23 / −275 · 2023 351 / 1.21 / 29 · 2024 438 / 1.18 / 24 ·
2025 358 / 1.36 / 48 · 2026 282 / 1.48 / 80. excess over unconditional 5-day hold in the same regime: +26 bps.
the regime filter is essential (below-200 slice PF 0.72–0.79 on the universe). four-name version is NVDA:
NVDA +$9.2k, MSFT +$1.9k, AMZN −$1.5k, AAPL −$1.8k (hold 3); 2025–2026 four-name PF 1.01 / 0.85.
capped: cap 2 → 66/yr PF 1.02; cap 4 → 122/yr PF 1.11; cap 8 → 207/yr PF 1.15 — the uncapped PF 1.26 needs
~8–28 simultaneous positions (mean 8.5, max 28). this is the highest-volume candidate and the thinnest per trade.

### c. breakout momentum (buy close, 5% stop, SPY>200)

| variant | n | n/yr | pnl $ | win% | pf | bps | mdd | note |
|---|---|---|---|---|---|---|---|---|
| four, 52-wk high, hold 5 | 75 | 15.9 | 910 | 52 | 1.09 | 12 | −2,727 | |
| four, 52-wk high, hold 10 | 56 | 11.8 | 7,028 | 57 | 1.75 | 126 | −2,950 | 2024 alone +$7.6k (PF 4.99); 2023 0.73, 2026 0.66 |
| universe, 52-wk high, hold 5 / 10 | 821 / 642 | 174 / 136 | −2,046 / 19,682 | 47 / 47 | 0.98 / 1.15 | −3 / 31 | −14,321 / −11,661 | below unconditional (+40 bps) |
| universe, 20-day high + vol > 1.5×, hold 5 | 686 | 145 | 23,367 | 50 | 1.22 | 34 | −16,377 | |
| **universe, 20-day high + vol, hold 10** | 650 | 137 | 42,460 | 47 | **1.31** | 65 | −21,661 | +27 bps over unconditional |

20-day/hold-10 per year: 2022 19 / 0.40 · 2023 180 / 0.84 · 2024 164 / 1.66 · 2025 165 / 1.23 · 2026 122 / 1.96.
top-3 names (MU, AMD, INTC) = 52% of P&L; 32/59 names positive. capped: cap 4 → 68/yr PF 1.33, cap 8 → 107/yr
PF 1.32. it is a 2024/2026 semiconductor-momentum result with a −$19k 2023 drawdown; 52-week highs do nothing.

### d. overnight only (buy close, sell next open)

| variant | n | n/yr | pnl $ | win% | pf | net bps | gross bps | mdd |
|---|---|---|---|---|---|---|---|---|
| four, unconditional | 2,272 | 480 | −19,204 | 46 | 0.83 | −8.5 | +1.5 | −20,133 |
| four, SPY>200 / SPY>50 | 1,748 / 1,628 | 370 / 344 | −11,270 / −1,436 | 46 / 47 | 0.85 / 0.98 | −6.4 / −0.9 | | |
| universe, unconditional | 33,512 | 7,084 | −278,878 | 44 | 0.80 | −8.3 | +1.7 | −279,068 |
| universe, SPY>200 / SPY>50 | 25,783 / 24,013 | | −243,658 / −183,154 | | 0.76 / 0.80 | −9.5 / −7.6 | | |

gross overnight premium on single large caps 2022–2026 is **+1.5–1.7 bps/night**; 10 bps of cost makes it the
worst strategy in the study. best year (2024, four names) was +4 bps net. dead at any retail cost model.

### e. bear-bounce at daily scale (name down > 2% while SPY < 50-day, buy close, hold N)

| variant | n | n/yr | pnl $ | win% | pf | bps | mdd | >200 pf/n | <200 pf/n |
|---|---|---|---|---|---|---|---|---|---|
| four, hold 1 / 2 / 3 | 298 / 257 / 232 | 63 / 54 / 49 | 11,208 / 12,489 / 16,622 | 55 / 52 / 52 | 1.41 / 1.33 / 1.42 | 38 / 49 / 72 | −3,052 / −4,884 / −6,032 | 1.80 / 1.63 / 1.55 | 1.31 / 1.25 / 1.37 |
| universe, hold 1 / 2 / 3 | 3,134 / 2,746 / 2,517 | 662 / 580 / 532 | −12,617 / 12,139 / 39,330 | 49 / 51 / 52 | 0.96 / 1.03 / 1.09 | −4 / 4 / 16 | −60,559 / −62,836 / −57,557 | 1.10 / 1.28 / 1.22 | 0.91 / 0.94 / 1.04 |

the four-name PF 1.4 is NVDA: hold-3 P&L NVDA +$11.9k, AMZN +$2.8k, MSFT +$1.9k, AAPL +$0.1k; 2025 PF 0.43.
on 59 names it is ≈ break-even (below unconditional) and the SPY<200 slice, which is where it should shine, is
0.91–1.04. the earlier intraday "bear-bounce" finding was a 2022-NVDA artefact; it does not generalise.

### volume question
nothing reaches ≥ 1 trade/day at PF ≥ 1.3 after costs. closest: reversal hold-5 with regime (1.2/day, PF 1.26,
needs ~8+ concurrent slots), 20-day breakout hold-10 (0.55/day, PF 1.31, semis-concentrated), PEAD hold-10
(0.27/day, PF 1.55, best per-trade edge and the only one with a year-by-year excess over beta). a 4-name book
gets 7 PEAD, ~28 reversal, ~12 breakout trades a year — no better than today's feedback rate.

## 3. intraday event angle on our own 1-minute data (AAPL, AMZN, MSFT, NVDA, 2022-01-03..2026-09-24)

reaction day = session after the 8-K date (all four report after the close); FOMC = decision day (14:00 ET
decision, so the 09:30–11:30 window is pre-announcement). long return 09:30 open → 11:30 (`intraday_events.py`):

| day type | days | mean bps | win% | P(r > +1%) | first-15 ≥ +0.5% days | continue to 11:30: win% / mean bps |
|---|---|---|---|---|---|---|
| earnings reaction (pooled) | 77 | **−15** | 47 | 21–42% by name | 29 | 55% / +27 |
| FOMC (pooled) | 146 | +5 | 53 | 8–32% | 33 | 61% / +28 |
| ordinary (pooled) | 4,515 | +1 | 50 | 14–27% | 1,031 | 52% / +5 |

by name on reaction days: AAPL +63 bps (63% win), AMZN −21, MSFT −63 (37% win), NVDA −38 (35% win); NVDA gap-up
> 2% days: −98 bps mean over the morning (n=10), MSFT gap-up days −41 (n=9). reaction-day morning std is
2–3× ordinary (171–347 vs 101–189 bps), so the window is a coin flip with triple the variance for a long.
the "upside mornings are news days" hypothesis is not supported: event days are 4.4–4.9% of sessions and
**5.4–6.3% of the > +1% mornings** — no enrichment. continuation after a strong first 15 minutes is 55% on
reaction days (n=29) and 61% on FOMC mornings (n=33) vs 52% ordinary; both samples are too small to act on
(a 55% vs 52% difference on n=29 is ~0.3 s.e.). daily-bar cross-check on all 59 names (`analyze_daily.py`):
open-to-close on reaction days −5.6 bps (49% win) vs +3.2 bps ordinary; reaction days are 1.6% of days and 4.1%
of the > +2% open-to-close days. verdict: an event *toggle that enables* longs is refuted; an event *exclusion*
(what `event_calendar.rs` already does) is the right sign. for the four names, the earnings edge is the 10-day
drift *after* the reaction session, not the reaction morning.

## 4. ticker angle — positive intraday (open→close) drift 2022–2026 (`sim_daily.py` drift table)

top 10 by annualised Sharpe of daily o→c, with per-year Sharpe (22/23/24/25/26), regime split, positive years:

| sym | sharpe | bps/day | by year | SPY>200 | SPY<200 | +yrs |
|---|---|---|---|---|---|---|
| AAPL | 1.24 | 11.7 | 0.33 / 2.75 / 1.66 / 0.94 / 1.54 | 1.51 | 0.91 | 5 |
| LIN | 1.24 | 8.7 | 1.08 / 2.57 / 0.54 / 0.49 / 1.68 | 1.17 | 1.46 | 5 |
| T | 1.00 | 8.2 | 0.69 / 0.10 / 2.47 / 1.58 / 0.18 | 0.94 | 1.15 | 5 |
| MA | 0.98 | 8.0 | 1.34 / 1.80 / 0.41 / 0.69 / 0.28 | 0.74 | 1.46 | 5 |
| GE | 0.98 | 10.0 | 0.46 / 2.71 / 0.95 / 1.67 / −0.83 | 0.83 | 1.32 | 4 |
| ABBV | 0.96 | 7.9 | 2.27 / 0.25 / 0.67 / 0.78 / 0.78 | 0.82 | 1.30 | 5 |
| V | 0.92 | 6.9 | 0.80 / 2.05 / 0.27 / 1.19 / 0.42 | 0.81 | 1.18 | 5 |
| XOM | 0.87 | 7.9 | 2.04 / 0.35 / −0.13 / 0.33 / 1.56 | 0.46 | 1.71 | 4 |
| GOOGL | 0.83 | 8.4 | −0.59 / 2.61 / 0.73 / 0.71 / 1.37 | 1.24 | 0.10 | 4 |
| DE | 0.79 | 7.8 | 1.17 / −0.19 / 0.72 / 1.00 / 1.20 | 0.64 | 1.11 | 4 |

bottom: PEP −0.57, NKE −0.41, HON −0.36 (0 positive years), CMCSA −0.34, NEE −0.32. of our four, AAPL is the
best intraday-drift name in the whole universe (5/5 years); AMZN/MSFT/NVDA are mid-table. the drift is 7–12
bps/day, i.e. below the 10 bps cost of expressing it as a daily round trip, so this is a filter for *which names
to allow longs in*, not a strategy. it is regime-agnostic (LIN, MA, ABBV, XOM are better below the 200-day).

## 5. engine implications (not implemented)

what exists: `SessionConfig.force_exit_by` + `flatten_all` from the clock (`crates/data_feed/src/main.rs`
~910), `paper-trader.timer` 06:10 PT start / `paper-trader-stop.timer` 13:10 PT stop, startup reconciliation
that treats any broker holding as an orphan (main.rs ~499), `LiveSession` = one intraday position, VWAP/day
state reset on Eastern date change, `Timescale::OneDay` exists in the enum but nothing feeds daily candles.
a multi-day book needs: (1) a position store that survives the daily stop/start (persist open positions in
postgres, re-adopt them at startup instead of orphaning; alpaca `open_positions()` becomes the source of
truth) ~2 d; (2) a session mode / window type with no `force_exit_by`, entries keyed to a daily signal
(e.g. "reaction day + 1 open") and an exit at bar N's close ~2 d; (3) gap-aware stops evaluated on the first
bar of each day (fill at open, not stop price) plus overnight risk caps per name/book ~1 d; (4) daily-bar
indicators: an EDGAR/earnings-date feed, prior-day close/gap, 50/200-day SMAs, fed via historical daily
bars at warm-up ~2 d; (5) trade writer / cockpit / eod-report handling of multi-day trades and open-position
carry P&L ~1–2 d; (6) tests + a backtest path for daily signals (the current replay is 1-minute-only) ~2 d.
rough total **8–12 days**; the persistent-position + reconciliation change is the risky part and touches the
safety net (watchdog, missed-close flatten) that currently assumes flat-by-close.

## 6. recommendation

ranking by (edge after costs) × (trades/yr) × (1 / implementation cost):

1. **PEAD hold-10 on the 59-name universe, open-gap > +3% trigger** — +135 bps/trade, PF 1.78, 63 trades/yr
   uncapped (30–45/yr with 2–4 slots), positive every year including 2022, +67 bps/trade over the same-period
   unconditional hold, works in both regimes. costs a new product (8–12 days). this is the one to test first.
2. 20-day breakout + volume, hold 10, SPY>200 — PF 1.31, 137/yr, but 2023 −$6.8k and 52% of P&L from three
   semis; only worth revisiting if (1) is built, since it reuses the same daily plumbing.
3. reversal hold-5 in uptrend — the volume answer (306/yr) but +26 bps over beta, PF 1.26 uncapped, 1.02–1.15
   at 2–8 slots; a liquidity-provision trade that needs breadth of capital we do not have.
4. ticker filter for the existing intraday long windows (AAPL/LIN/MA/ABBV/V) — cheap (config only) but the
   drift is < 12 bps/day; at best it stops the long side bleeding, it does not create volume.
5. event toggle on the intraday long windows — refuted (reaction mornings −15 bps, no enrichment of strong
   mornings); keep the FOMC/earnings *exclusion*. overnight and bear-bounce — dead / NVDA artefact; drop.

how to test (1) before writing any engine code: paper-trade it by hand-off — a daily script after each
earnings evening lists names whose reaction-day open gaps > +3%, places a 10-session market-on-open long via
the alpaca paper account (separate sub-account or tagged client_order_id so the intraday book's reconciliation
ignores it), with a 5% gap-aware stop and a 4-slot cap. **accept-if**: after 40 trades (≈ one full earnings
season, ~9 months at cap 4) the realised mean net return per trade is ≥ +50 bps and PF ≥ 1.3, and the
realised excess over an unconditional 10-day hold of the same names on the same dates is positive; reject if
mean < +25 bps or the season's max drawdown exceeds 6 × the average trade size in $. the backtest prior is
+135 bps / PF 1.78, so +50 bps is a deliberately conservative bar for the live-vs-sim gap.
caveats: 4.7 years, one bull-heavy tape; earnings dates from 8-K filing dates (a handful of names file extra
2.02 8-Ks; taking one per quarter handles it); no borrow/slippage beyond 10 bps; equal-dollar sizing; the
"reaction day = filing day or next, whichever gaps more" rule leaks the gap direction into day selection but not
into the > +3% entry rule.
