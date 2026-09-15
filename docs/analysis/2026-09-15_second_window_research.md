# Candidate entry windows for a second, independent trade source

Research memo, 2026-09-15. Scope: US mega-caps AMZN / AAPL / NVDA / MSFT, 1-minute RTH bars,
SPY as context, five-year local cache, honest costs 3 bps + $0.005 per leg (~6.5 bps round trip).
Incumbent: v17/v18 short-only morning thrust (09:30–11:30 ET, SPY-flat + VPIN-top-quintile
filters, honest PF 1.64, ~2 trades/week). Goal: more trades from a window that does not share
the morning short's time of day, direction, or driver.

Verdict scale used below: **documented** = peer-reviewed or multi-year public backtest with
out-of-sample or sub-period evidence; **plausible** = real evidence but on other instruments,
without costs, or with a known replication problem; **folklore** = repeated claim without a
traceable multi-year test.

## 0. Three findings that reorganise the search

1. **Individual stocks do not behave like the index in the last half hour.** Baltussen, Da &
   Soebhag ("End-of-Day Reversal", Apr 2025 WP, US stocks 1993–2020s) show the *cross-section*
   reverses in the last 30 minutes (t > 10) even though the *market* shows momentum (Gao et al.
   2018; Baltussen et al. 2021). Their decomposition: a time-series strategy on each stock's own
   rest-of-day return earns +2.79 bps/day, the cross-sectional strategy earns −3.21 bps/day, and
   the negative part comes entirely from cross-stock covariance (−5.97 bps/day): names that
   lagged the market catch up. The effect is asymmetric — **only intraday losers revert**
   (interaction t = −6.97; positive rest-of-day returns do not revert) — and survives in the
   largest-20 % size quintile (six-factor alpha 3.41 bps/day, t = 10.61). This is the single
   best-documented single-name intraday pattern that does not overlap the morning short.
2. **The friction ceiling is the enemy, not signal absence.** Mesfin (2026, arXiv 2605.04004)
   tested 14 OHLCV signal families on MNQ 5-minute bars 2021–2025 with walk-forward validation:
   ORB, gap-fill fade, volume spikes/dry-ups, OU mean reversion, event-day trend, a
   volatility-regime classifier. Gross edge 0.07–1.5 points per trade against 2 points of
   friction; nothing passed; the most common failure mode was "one strong year (2024) masking
   flat or negative years". Heston, Korajczyk & Sadka (2010) put the periodicity premium at
   ~3 bps per half hour, "the same order of magnitude as twice the quoted half-spread". Any
   window with an expected gross move under ~10 bps is dead on arrival at 6.5 bps round trip.
3. **The published intraday-momentum wins are index-level and cost-sensitive.** Zarattini & Aziz
   (SPY, 2007–Apr 2024, 1-min) get 19.6 %/yr, Sharpe 1.33 net of $0.0035 + $0.001/share, but
   the base version without the VWAP trailing stop is Sharpe 0.61; QuantConnect's replication was
   "positive, but nowhere near as impressive" with fees eating 16.7 % of returns, and a live
   trader reported six poor months. Quantitativo's ES/NQ version (2010–2025) is Sharpe 1.25/1.67
   after costs but states it "underperforms on most individual stocks". QuantConnect's plain
   first-half-hour → last-half-hour ETF replication (SPY/IWM/IYR, 2015–2020) had Sharpe −0.63.

## 1. Ranked candidates

Ranking weighs (a) evidence quality on liquid US single names, (b) independence from the
morning short, (c) expected per-trade gross edge relative to 6.5 bps, (d) trade count on four
names. Frequency figures are estimates from base rates and must be measured from the cache.

---

### 1. End-of-day loser reversal — LONG the name that lagged, 15:30 → close  — **documented**

**Idea.** At 15:30 ET, go long a name whose return from yesterday's close to 15:00 (ROD3) is
strongly negative *relative to SPY* (idiosyncratic loser), hold to the 15:58–16:00 close.
Driver: attention-induced retail "buy the dip" plus short-sellers covering before the overnight
(both measured directly in the paper via retail order-imbalance classifiers and short-volume
data); the price pressure is transitory and reverts the next day.

**Evidence.** Baltussen, Da & Soebhag (2025): US common stocks from 1993, ROD3 negatively predicts
the 15:30–16:00 return in the cross-section; long-short quintile spread 3.78 bps/day
value-weighted (9.5 %/yr) and 6.86 bps/day equal-weighted (17.3 %/yr); bottom decile of intraday
losers gained ~400 % over 27 years held only 30 min/day. Quintile L earns 3.55 bps/day in LH vs
−0.22 bps for quintile H. Robust in 3-year rolling regressions "even in the later years",
across days of the week, in index members (S&P 500/NDX/DJIA), among the largest, most traded and
most liquid names, and after skipping the 15:00–15:30 interval to kill bid-ask bounce. The
authors say plainly it "might not be exploitable by many investors after accounting for
transaction costs" as a full cross-sectional portfolio, but that "the effect is stronger for
stocks with even more extreme intraday returns". Related: Bogousslavsky (JFE 2021) finds
mispricing corrects during the day and *worsens* at the end of the day as arbitrageurs unwind
before overnight margin/lending costs — same direction of pressure. Not anecdote.

**Independence.** Opposite direction (long), opposite end of the day (15:30 vs 09:30–11:30),
different driver (retail/short-cover flow vs morning information thrust). Zero time overlap.

**Frequency.** Condition ROD3(name) − ROD3(SPY) ≤ −1.5 % (or ≤ −2 %): on NVDA roughly 15–25 % of
days, AAPL/MSFT 5–10 %, AMZN 10–15 % → order of 120–200 name-days per year, ~600–1,000 over the
cache. Tightening to ≤ −2.5 % keeps ~50–90/yr with a larger expected edge.

**Test.** New window on a second session block (15:30–15:35 entry, forced flat 15:58):
`prior_day_levels` return-from-prior-close ≤ −X %, `cross_context` SPY return-from-prior-close
> −0.5 % (idiosyncratic, not market-wide), not an earnings-reaction day (`event_calendar`),
optionally VPIN *not* extreme (avoid informed selling). Exit stack: time exit at 15:58, 1 % hard
stop, no breakeven/time-stop (the 40-minute losing limit is meaningless in a 28-minute hold).
Score the sweep on mean bps per trade vs 6.5 bps, by year, by threshold. Also test the 15:00
entry variant: the paper's Figure 3 shows negative intraday returns reverting "most strongly
during SLH" (15:00–15:30) as well.

**Biggest failure risk.** Magnitude. The large-cap alpha is 3.4 bps/day on a *quintile*; you
need the extreme tail to clear 6.5 bps, so the useful trade count could be a fraction of the
above. Also the engine's session logic (`force_exit_by` 11:55, the watchdog's "position past
11:58" alert, `no_new_entries_after`) must gain a second session, and a 15:58 market exit on a
1-minute engine may fill worse than a MOC order.

---

### 2. Last-half-hour market momentum, expressed through the name — **documented (index) / plausible (name)**

**Idea.** At 15:30, when SPY's return from yesterday's close to 15:30 (ROD) is large, take the
name in that direction into the close. Driver: gamma hedging by option market makers, leveraged
ETF rebalancing, volatility-targeting and MOC flow — all mechanically trade *with* the day's
move in the last 30 minutes.

**Evidence.** Gao, Han, Li & Zhou (JFE 2018): SPY 1993–2013, first half-hour return predicts
the last half-hour (scaled slope 6.94, R² 1.6 %, 2.6 % with the 15:00–15:30 return), in- and
out-of-sample, stronger on volatile, high-volume and macro-news days; holds for ten other ETFs.
Baltussen, Da, Lammers & Martens (JFE 2021): 60+ futures 1974–2020; the rest-of-day return
predicts the last 30 minutes; 1/N equity-index-futures timing strategy on ROD earns 6.86 %/yr at
3.96 % vol, Sharpe 1.73, hit rate 55 %; requiring the first-half-hour and ROD signs to agree
gives Sharpe 1.60; "always long the last half hour" is Sharpe 0.11; robust in both 1974–1999 and
2000–2020 sub-samples; no costs applied, but they note a positive net Sharpe in ES at one-tick
cost; the effect is much stronger on negative net-gamma-exposure days. Contrary evidence:
QuantConnect's ETF replication 2015–2020 Sharpe −0.63 (only positive in the 2020 crash, 1.45);
End-of-Day Reversal shows the single-name cross-section reverses, so on a single name the
market component and the idiosyncratic component fight. Post-2022 regime risk: 0DTE options are
>50 % of SPX volume and Cboe/Dim–Eraker–Vilkov find 0DTE flows are counter-directional at the
index level, producing "stronger intraday order-flow reversals, muted momentum returns, and
lower intraday volatility".

**Independence.** Afternoon, both directions (mostly long in up-markets), market-flow driver.
No overlap with the morning short in time. Caveat: on a down day this is a short and shares
direction with the incumbent, but not the window or driver.

**Frequency.** |SPY ROD| ≥ 0.75 % on roughly 30–35 % of days; requiring the name to agree in
sign and not be a top-quintile loser (that is candidate 1's territory) gives ~50–80 days/yr,
but all four names fire on the same days → effectively ~1 correlated basket trade per
qualifying day, 250–400 name-trades over five years.

**Test.** Window 15:30–15:35: `cross_context` SPY return since prior close ≥ +0.75 % (long) or
≤ −0.75 % (short), name's own return since prior close same sign, name not lagging SPY by more
than 1 % (else candidate 1). Combine with candidate 1 as one "15:30 book": long lagging names on
up days is where both mechanisms point the same way. Exit 15:58, 0.75 % hard stop. Test with and
without a realised-volatility gate (Gao/Zarattini: edge concentrates on high-vol days;
Zarattini's SPY Sharpe is ~1.5 when VIX > 6 and 3.5 when VIX > 40).

**Biggest failure risk.** Single-name expression of an index effect: the End-of-Day paper shows
the cross-stock covariance term is negative and larger than the own-stock momentum term, and
the 2015–2020 ETF replication was negative. Also the concurrency cap 3 makes this a basket bet.

---

### 3. Afternoon noise-band trend continuation (Zarattini–Aziz), 12:00–15:30 signals — **documented (SPY/ES/NQ net of costs) / plausible (names)**

**Idea.** Define a "noise area" around today's open: ± the 14–90-day average absolute move from
the open *at that time of day* (scaled by recent volatility, shifted by the gap). Every 30
minutes from 12:00, if the name closes above the band go long, below it go short; trail the stop
at max(VWAP, band); exit at the close. Driver: demand/supply imbalance that persists once the
day's move exceeds normal noise (late-informed traders, gamma hedging).

**Evidence.** Zarattini & Aziz (SFI 24-97; SPY 1-min, May 2007–Apr 2024): base version Sharpe
0.61 (6.2 %/yr); VWAP trailing stop 9.7 %/yr Sharpe 1.24; with 2 % vol targeting up to 4×,
19.6 %/yr, Sharpe 1.33, max DD 25 %, daily hit 43 %, avg 12 bps/day (t 5.34), all net of
$0.0035 + $0.001/share; 5,494–7,668 trades over 17 years (~320–450/yr); Sharpe rises with VIX;
Wednesdays 18 bps (t 3.42, FOMC), Mondays 9 bps (t 1.84, n.s.). 33 futures 2007–2024: average
Sharpe 0.60, S&P 500 1.32, Nasdaq 1.23. Quantitativo's independent ES/NQ build 2010–2025: 16.8 %
/ 24.3 %, Sharpe 1.25 / 1.67, win 36–38 %, avg trade +2 / +6 bps, two losing years in 16, but
"underperforms on most individual stocks" and warns that picking stocks after the fact "is
textbook selection bias". Zarattini's FAQ claims it "absolutely" works on liquid US stocks
2018–2024 (a chart, no table). QuantConnect replication: positive but far weaker, fees 16.7 % of
returns, margin calls, a live trader's poor six months. Maróy (2025) gets Sharpe > 3 on QQQ only
with heavy optimisation — treat as overfit. Hunt Gather Trade daily-bar QQQ 2014–2024: PF 1.26,
Sharpe 1.13.

**Independence.** Signals after 12:00 only (the morning short is flat by 11:55), both
directions, trend-following rather than thrust-fading. Different driver (persistence of an
established daily imbalance).

**Frequency.** On SPY the full-day version takes a position on most days; restricted to first
band breaks after 12:00 on four names, roughly 0.3–0.6 signals per name-day → 300–600
name-trades/yr, many stopped quickly (avg trade only +2–6 bps on futures, which is the problem).

**Test.** Needs one new indicator type, `noise_band` (time-of-day-indexed average |close −
open| over N days, ± multiplier), then a window: `noise_band` breach, `session_time` ≥ 12:00,
optional realised-vol gate. Exit: trailing stop at max(VWAP, band) via `atr_trailing_stop`-style
action, forced flat 15:58, no time stop. Evaluate mean bps per trade; run the SPY-only version
first to confirm the implementation reproduces ~10 bps/day gross before touching names.

**Biggest failure risk.** Per-trade gross edge of 2–12 bps on the index vs 6.5 bps cost on a
noisier single name; Quantitativo's explicit "fails on most stocks". Also the frequency is
exactly where the cost model bites.

---

### 4. Earnings-day "stock in play" opening-range breakout (both directions) — **documented (broad universe) / plausible (mega-caps)**

**Idea.** On the first session after the name's earnings (all four report after the close),
the name is a "stock in play" (relative volume 3–5×). Enter on the 5-minute opening-range
break in the break direction, stop at the other side of the range (or the 09:30 open), hold
to the close or a fixed time.

**Evidence.** Zarattini, Barbon & Aziz (SFI, Feb 2024): 7,000+ US stocks 2016–2023, top-20
relative-volume names each day, 5-minute ORB: net total return > 1,600 %, Sharpe 2.81,
annualised alpha 36 %; 15/30/60-minute ranges also positive. QuantConnect replication on the
1,000 most liquid names (2016): Sharpe 2.40 vs SPY 0.84, 68 % of parameter combinations beat
the benchmark; a commenter reports mega-caps "completely crash in '08" and users flag ~25 %
of returns going to costs. Microstructure literature (jumps after earnings): simple rules'
returns dissipate within 5–10 minutes of the release itself — the release is pre-market for
these names, so the RTH open is a second wave, not the first.

**Independence.** Event-driven, both directions, only ~16 name-days per year; entirely
different driver from the momentum thrust (information shock vs flow). Time overlaps the
morning window, but v16/v17 "barely trade" earnings days, so there is no collision in practice.

**Frequency.** 4 names × 4 reports = 16 name-days/yr, ~80 in the cache; perhaps 60 fills.
Enough to see a sign, not enough to tune.

**Test.** `event_calendar` earnings-reaction-day flag (the 8-K dates are already in
`research/entries/data/`), `opening_range` break (5 or 15 min), `rvol` ≥ 2, direction = break
direction; stop at the opposite OR edge, exit 11:55 (truncated) and 15:58 (full) as two
variants. Judge on mean bps per trade and sign consistency across the 5 years.

**Biggest failure risk.** n ≈ 60; the 3 bps + $0.005 cost model is wrong in the first minutes of
an earnings-gap open (spreads and slippage are several times normal); mega-caps gap 5–10 % and
the entire move may be in the gap. Do not promote on this sample alone.

---

### 5. Idiosyncratic gap-down fade — LONG toward prior close, 09:45–11:30 — **plausible, recent evidence negative**

**Idea.** When the name gaps down 0.5–2 % while SPY gaps little, buy after the first 15
minutes and target the prior close (gap fill), stop under the opening low. Driver: overnight
liquidity/retail price pressure at the open (Baltussen et al. also document retail-attention
price pressure at the open, and Jones–Pyun–Wang find retail extrapolative at the open).

**Evidence.** Base rates: QQQ 1999–2023 (6,005 days), down gaps 0.5–0.99 % fully fill the same
day 59 % of the time, 1–1.99 % 47 %, ≥ 2 % 29 %; up gaps 53 / 45 / 33 %. Japanese large caps
(2 years, 1-min, 15,023 gaps): 0.5–1 % gaps fill same day only 33.7 %, ≥ 2 % 14.6 %. MDPI
2019 (S&P 500 constituents 1998–2015, jump-test-identified overnight gaps): 51.5 %/yr, Sharpe
2.38 after (HFT-level) costs, mean reversion "particularly significant 120 minutes after
market opening". Against: Mesfin 2026, MNQ 2021–2025, gap-fill fade at 09:30/09:45/10:00 all
fail (T −0.32 to −0.59, win 47–48 %); the same study's only near-pass was gap *continuation*
short (T 3.23, N = 22). This system already rejected gap-and-go and its own long book loses
2023–2025.

**Independence.** Long direction, overnight-flow driver. Time overlaps the morning short; the
two can coexist only because the concurrency cap is per ticker and direction differs — but a
gap-down day is often exactly the day the short window fires, so treat as *complementary on
the same clock*, not independent.

**Frequency.** Name gap −0.5 % to −2 % with |SPY gap| < 0.3 %: roughly 8–15 % of days per name
→ 80–150 name-days/yr.

**Test.** Window 09:45–10:30: `gap` in (−2 %, −0.5 %), `cross_context` SPY gap within ±0.3 %,
not an earnings day, price above the 09:30–09:45 low; target = prior close (`prior_day_levels`),
stop = opening-range low, time exit 11:55. Compare against a plain "long at 09:45 on gap-down"
control.

**Biggest failure risk.** The 2021–2025 futures evidence is negative and this engine's long side
has lost in 2023–2025 under every filter tried; 2022 will dominate any positive pooled number.

---

### 6. Lunch-hour long drift, 12:00 → 14:00 — **plausible at best (index-only, no costs)**

**Idea.** Quantpedia's "lunch effect": SPY falls or flattens 11:00–12:00 and rises 12:00–14:00
on average (2010–May 2024). Trade the names long from 12:00, out by 14:00, optionally only
when the name is below VWAP at noon (mean-reversion flavour).

**Evidence.** Quantpedia (SPY, 2010–2024): rules "short at 11:00, cover and go long at 12:00,
sell at 14:00"; the write-up shows an equity curve but no stated return/Sharpe, no
transaction costs and no out-of-sample split, and the authors themselves hedge about regime
change. Volatility U-shape (open/close high, lunch low) is well documented, but that says
nothing about drift. No single-name evidence found. Verdict: plausible for SPY, folklore for
names.

**Independence.** Different hours, long only, calendar/flow driver.

**Frequency.** Unconditional: every day × 4 names; conditioned on name below VWAP at 12:00 and
SPY flat, ~40 % of days → ~400 name-trades/yr.

**Test.** Window 12:00–12:05: `vwap_distance` < 0 (or < −0.3 %), `cross_context` SPY session
return within ±0.3 %; exit 13:55 or on touch of VWAP; 0.75 % hard stop.

**Biggest failure risk.** Expected gross move of a few bps over two hours on a 1.5 %-vol name
is far below the cost line; Mesfin's OU mean-reversion and volume-signature tests are precise
nulls after friction. Cheap to test, likely to fail.

---

### 7. FOMC-afternoon index trend, 14:05 → close on the eight decision days — **plausible (index) / folklore (names), too few events**

**Idea.** After the 14:00 statement (press conference 14:30), trade the names in the
direction SPY has moved by 14:05–14:35 and hold to the close.

**Evidence.** Gao et al.: intraday momentum is stronger on macro news days. Zarattini–Aziz:
Wednesdays are the best day (18 bps, t 3.42) and they attribute it to FOMC trends. Against:
the pre-FOMC drift (49 bps, 1994–2011, Lucca–Moench) "essentially disappeared after 2015"
(Kurov et al., FRL 2020), the post-announcement move tends to *reverse* the pre-drift, and
Mesfin finds event-day drift only in the first five 5-minute bars after 08:30 releases
(from bar +6, T 0.14–0.69). Knox & Vissing-Jorgensen (FEDS 2026-023) is the current survey.

**Independence.** Afternoon, event-driven, both directions.

**Frequency.** 8 days/yr × 4 names = 32 name-days/yr, 160 in the cache; far too few to
validate a standalone window.

**Test.** Not as its own window: add `event_calendar` FOMC-day as a *condition variant* of
candidates 2 and 3 (does the afternoon trend window do better on FOMC days?) and report it as
a sub-table only.

**Biggest failure risk.** Sample size and the 2:30 presser whipsaw.

---

### 8. Index-relative residual reversal (name vs SPY) in the afternoon, 13:00–15:00 — **plausible, public replications negative after costs**

**Idea.** Beta-adjust the name to SPY intraday; when the residual since the open exceeds
−2 σ (name has underperformed SPY by an abnormal amount without news), go long for a
reversion toward the residual mean, exit on the mean or by 15:00. The earlier-in-the-day
cousin of candidate 1.

**Evidence.** The End-of-Day paper's decomposition shows the cross-stock covariance term is
the source of the reversal, but only in the last half hour; HKS 2010 show within-hour
reversals are liquidity/bid-ask bounce. Public intraday pairs replications: QuantStart
SPY/IWM 1-min z-score (2007–2014) — authors state it "would certainly perform very poorly"
with costs and flag a look-ahead bias; a 2025 factor-residual intraday engine on GitHub lost
1.82 % (PF 0.43) in its validation run. Quantpedia's short-term-reversal note: intraday
returns reverse over the following *week* (not the same day) and it is stronger in illiquid
names.

**Independence.** Long, afternoon, liquidity driver.

**Frequency.** Residual ≤ −2 σ at 13:00–14:00 on ~5–8 % of name-days → 50–80/yr.

**Test.** Residual = name session return − β × SPY session return (`cross_context`, β from
prior 20 days, computed offline first); window 13:00–14:30, exit at residual 0 or 15:00.
Compare with candidate 1 (same signal, later entry) — if the 15:30 version is better, drop
this one.

**Biggest failure risk.** Reversion half-life vs cost: nothing in the literature puts the
same-day reversal before the last half hour.

---

### 9. Midday VWAP-band mean reversion (± 2 σ) — **folklore**

**Idea.** Short (long) the name when it is 2 standard deviations above (below) session VWAP
between 11:30 and 14:30 with low ADX; target VWAP.

**Evidence.** The widely repeated "QuantConnect 2022 backtest on 100 NASDAQ stocks, 63 % win
rate, 1.5:1" exists only on content-farm pages (tradezella, chartswatcher, tradealgo); nothing
on quantconnect.com matches it. Mesfin: OU mean reversion fails at every threshold after
friction (T −1.1 to −4.5); HKS: sub-hour reversals are bid-ask bounce. No peer-reviewed
single-name VWAP-band result found. VWAP *is* a documented institutional benchmark (Haendler,
Heston, Korajczyk & Sadka 2025: VWAP-like trading explains the open/mid-day periodicity), which
is a reason prices are *attracted* to VWAP, not evidence that fading 2 σ is profitable.

**Independence.** Both directions, midday, liquidity driver.

**Frequency.** High (multiple signals per name-day).

**Test.** One config line: `bollinger_pct_b`/`vwap_distance` ≥ 2 σ, `adx` < 20, 11:30–14:30,
exit at VWAP touch or 60 min. Expect failure; worth 30 minutes of replay to close the question.

**Biggest failure risk.** Cost line vs sub-10-bps expected move; trending days (ADX filter is
after the fact).

---

### 10. Same-half-hour periodicity tilt (Heston–Korajczyk–Sadka) — **documented but uneconomic alone; use as a filter**

**Idea.** A stock's return in a given half hour predicts its return in the same half hour on
subsequent days (lags 13, 26, 39 … half-hour intervals), especially the first and last half
hours. Trade: prefer (or only allow) candidate 1/2 entries in names whose 15:30–16:00 return
was positive yesterday, and the mirror for the morning short.

**Evidence.** HKS (JoF 2010; NYSE 2001–2005 TAQ-matched): decile winners minus losers at the
same interval next day 3.01 bps per half hour (smallest daily-lag t-stat 9.62), persists ≥ 40
days at > 1 bp for 5 days; "of the same order of magnitude as twice the quoted half-spread",
i.e. cost-neutral as a standalone trade. Haendler, Heston, Korajczyk & Sadka (2025): institutional
VWAP trading explains all of the open/mid-day periodicity and up to 30 % of the close
periodicity — the close residual is the one worth conditioning on.

**Independence.** n/a — overlay.

**Frequency.** n/a.

**Test.** Add "yesterday's 15:30–16:00 return sign" as a condition variant on candidates 1–2.

**Biggest failure risk.** 3 bps is the whole effect; as a trigger it cannot pay costs.

---

### Overlay A. Volatility-regime gate — **documented as a conditioner**

Gao et al.: momentum stronger on volatile/high-volume days. Zarattini: Sharpe ~1.5 when VIX > 6,
3.5 when VIX > 40. Baltussen et al.: stronger on negative net-gamma days. Mesfin's VVG classifier
(top tercile of |first-30-min return|, |gap| and first-bar volume; 4.4 % of days) produced a
77.6 % "peak-reversal before close" rate yet every directional strategy on those days failed.
Use: realised 10-day SPY range or ATR percentile as a gate on candidates 2 and 3; report each
window in low/mid/high terciles. The 0DTE era (2022+) is the regime to watch: lower realised
vol and more counter-directional afternoon flow.

### Overlay B. Day-of-week — **folklore for mega-caps**

Birru: Monday/Friday cross-sectional anomaly returns occur intraday but in *speculative*
stocks; a 2007–2022 study finds Monday-afternoon returns reverse over the rest of the *week*
(not intraday); Zarattini's Monday is the one non-significant day; the End-of-Day reversal
coefficients "do not differ significantly across days of the week". Not a window; at most a
sub-table.

## 2. What was covered and what the evidence actually says

| family | best evidence | verdict for four mega-caps |
|---|---|---|
| last-hour / power-hour momentum | Gao 2018, Baltussen 2021 (index, no costs, sub-period robust); QC 2015–20 replication negative; 0DTE regime | documented (index) / plausible (name) — candidate 2 |
| end-of-day single-name reversal | Baltussen–Da–Soebhag 2025 (large caps t 10.6) | documented — candidate 1 |
| 11:30–14:00 lunch reversal | Quantpedia SPY 2010–24, no costs/OOS | plausible / folklore — candidate 6 |
| close-to-open split (Lou–Polk–Skouras) | JFE 2019: momentum profits accrue overnight, reversal/liquidity effects intraday; Xu 2017: first-2-hour returns carry information, last-2-hour returns carry liquidity | documented at monthly horizon; not an intraday window; supports "morning = information, afternoon = liquidity" framing |
| first-half-hour → last-half-hour (Gao et al.) | as above; individual-stock version weak (Swedish OMX thesis 2023–24, "weak predictability") | index-only |
| VWAP mean reversion | no traceable multi-year single-name test; Mesfin OU nulls | folklore — candidate 9 |
| time-of-day seasonality in mega-caps | HKS 2010 (3 bps/half-hour); HHKS 2025 (VWAP/MOC explain it) | documented but uneconomic — overlay 10 |
| EOD rebalancing / last 30 min | LETF (Cheng–Madhavan 2009) impact not economically significant in longer samples (JFM 2018); MOC volume up, closing-auction reversals 21–43 % overnight (Jegadeesh–Wu) — needs imbalance data the system lacks | mechanism supports candidates 1–2; no bar-only trigger |
| index-relative pairs vs SPY | QuantStart, GitHub replications negative after costs | plausible-weak — candidate 8 |
| vol-regime conditioning | Gao, Zarattini, Baltussen | documented as overlay A |
| gap-fill statistics | QQQ 1999–2023 59 % same-day fill for 0.5–1 % down gaps; MNQ 2021–25 fade fails | plausible/negative — candidate 5 |
| day-of-week / FOMC / earnings | Birru; Lucca–Moench drift gone post-2015; Zarattini stocks-in-play Sharpe 2.81 | folklore / too few events / documented for the universe — candidates 7, 4 |
| practitioner multi-year | Zarattini–Aziz SPY 2007–24; Quantitativo ES/NQ 2010–25; QC replications; Mesfin MNQ falsification | documented on indices; single-stock versions fail or are unpublished — candidate 3 |

## 3. Suggested order of work

1. **Build the afternoon session plumbing once** (second entry/exit block, watchdog thresholds,
   MOC-style 15:58 exit), because candidates 1, 2, 3, 6, 7, 8 all need it.
2. **Candidate 1 first**, with a threshold sweep on ROD3 − SPY ROD3 (−1 %, −1.5 %, −2 %, −2.5 %,
   −3 %) and entry 15:00 vs 15:30. Pre-register: promote only if mean net bps/trade > 0 in ≥ 4 of
   5 years and pooled PF ≥ 1.3, same bar as v17.
3. **Candidate 2 as the up-day companion** (long lagging names when SPY ROD ≥ +0.75 %); test
   whether adding it to 1 raises the trade count without lowering PF.
4. **Candidate 3** only after reproducing ~10 bps/day gross on SPY itself in the replay.
5. Candidates 4 (earnings ORB) and 5 (gap fade) as sign checks, not tuning targets.
6. 6, 8, 9 as one-line configs to close the questions; 7, 10, A, B as sub-tables on the others.

## Sources

- Gao, Han, Li, Zhou, "Market Intraday Momentum", JFE 2018 — https://www.sciencedirect.com/science/article/abs/pii/S0304405X18301351 ; SSRN https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2440866 ; PDF https://assets.super.so/e46b77e7-ee08-445e-b43f-4ffd88ae0a0e/files/ee7dac49-530b-4950-b5d0-e0b5eee08f2e.pdf
- Baltussen, Da, Lammers, Martens, "Hedging Demand and Market Intraday Momentum", JFE 2021 — https://academicweb.nd.edu/~zda/intramom.pdf ; SSRN https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3760365
- Baltussen, Da, Soebhag, "End-of-Day Reversal", working paper April 2025 — https://academicweb.nd.edu/~zda/EOD.pdf ; EFMA 2024 version http://www.efmaefm.org/0EFMAMEETINGS/EFMA%20ANNUAL%20MEETINGS/2024-Lisbon/papers/EndofDayReversal_withnames.pdf
- Heston, Korajczyk, Sadka, "Intraday Patterns in the Cross-section of Stock Returns", JoF 2010 — https://arxiv.org/abs/1005.3535
- Haendler, Heston, Korajczyk, Sadka, "The Intra-day Stock Return Periodicity Puzzle", 2025 — https://www.kellogg.northwestern.edu/academics-research/research/detail/2025/the-intra-day-stock-return-periodicity-puzzle/?p=1
- Bogousslavsky, "The Cross-Section of Intraday and Overnight Returns", JFE 2021 — https://econpapers.repec.org/article/eeejfinec/v_3a141_3ay_3a2021_3ai_3a1_3ap_3a172-194.htm ; https://bogousslavsky.github.io/
- Bogousslavsky, Muravyev, "Who Trades at the Close?", JFM 2023 — https://www.sciencedirect.com/science/article/abs/pii/S1386418123000502
- Jegadeesh, Wu, "Closing Auctions: Nasdaq versus NYSE", JFE 2022 — https://www.sciencedirect.com/science/article/abs/pii/S0304405X21005092
- Lou, Polk, Skouras, "A Tug of War: Overnight versus Intraday Expected Returns", JFE 2019 — https://personal.lse.ac.uk/polk/research/TugOfWar.pdf
- Xu, "Reversal, Momentum and Intraday Returns", 2017 — https://papers.ssrn.com/sol3/Delivery.cfm/SSRN_ID2991183_code1628596.pdf?abstractid=2991183 ; CXO summary https://www.cxoadvisory.com/momentum-investing/when-to-look-for-momentum-and-reversal-intraday/
- Zarattini, Aziz, "Beat the Market: An Effective Intraday Momentum Strategy for S&P500 ETF (SPY)", SFI 24-97, 2024 — https://www.sfi.ch/en/publications/n-24-97-beat-the-market-an-effective-intraday-momentum-strategy-for-s-p500-etf-spy ; PDF https://alexandria.unisg.ch/bitstreams/a99aba00-f967-49b3-aceb-f544dc386e0b/download ; CXO https://www.cxoadvisory.com/momentum-investing/complex-intraday-time-series-momentum-strategy-applied-to-spy/
- QuantConnect forum replication of the above — https://www.quantconnect.com/forum/discussion/17091/beat-the-market-an-effective-intraday-momentum-strategy-for-s-amp-p500-etf-spy/
- QuantConnect, "Intraday ETF Momentum" (SPY/IWM/IYR 2015–2020) — https://www.quantconnect.com/learning/articles/investment-strategy-library/intraday-etf-momentum
- Quantitativo, "Intraday Momentum for ES and NQ" — https://www.quantitativo.com/p/intraday-momentum-for-es-and-nq
- Maróy, "Improvements to Intraday Momentum Strategies…", SSRN 2025 — https://papers.ssrn.com/sol3/papers.cfm?abstract_id=5095349
- Hunt Gather Trade, intraday momentum replication part 1 — https://newsletter.huntgathertrade.com/p/intraday-momentum-researched-based
- Zarattini, Barbon, Aziz, "A Profitable Day Trading Strategy for the U.S. Equity Market", SFI 2024 — https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4729284 ; https://concretumgroup.com/a-profitable-day-trading-strategy-for-the-u-s-equity-market/
- QuantConnect, "Opening Range Breakout for Stocks in Play" — https://www.quantconnect.com/research/18444/opening-range-breakout-for-stocks-in-play/
- Mesfin, "Structural Limits of OHLCV-Based Intraday Signals in MNQ Futures: A Systematic Falsification Study", arXiv 2605.04004 (2026) — https://arxiv.org/pdf/2605.04004
- Quantpedia, "Lunch Effect in the U.S. Stock Market Indices" — https://quantpedia.com/lunch-effect-in-the-u-s-stock-market-indices/
- Quantpedia, "Novel Market Structure Insights From Intraday Data" (Shen & Shi 2020) — https://quantpedia.com/novel-market-structure-insights-from-intraday-data/
- Quantpedia, "Short-Term Return Reversals and Intraday Transactions" — https://quantpedia.com/short-term-return-reversals-and-intraday-transactions/
- Epic Trader, QQQ gap-fill statistics 1999–2023 — https://epicctrader.com/gap-fills/
- Japanese large-cap gap-fill study (15,023 gaps, 1-min) — https://note.com/pino_world/n/nc2509698fd09?hl=en
- "Statistical Arbitrage with Mean-Reverting Overnight Price Gaps on High-Frequency Data of the S&P 500", JRFM 2019 — https://www.mdpi.com/1911-8074/12/2/51
- QuantStart, SPY/IWM intraday pairs — https://www.quantstart.com/articles/Backtesting-An-Intraday-Mean-Reversion-Pairs-Strategy-Between-SPY-And-IWM/
- shokodata, factor-residual intraday mean reversion — https://github.com/shokodata/factor-residual-intraday-mean-reversion
- Lucca, Moench, "The Pre-FOMC Announcement Drift", JoF 2015 — https://onlinelibrary.wiley.com/doi/abs/10.1111/jofi.12196
- Kurov et al., "The disappearing pre-FOMC announcement drift", FRL 2020 — https://pmc.ncbi.nlm.nih.gov/articles/PMC7525326/
- Knox, Vissing-Jorgensen, "The Effect of the Federal Reserve on the Stock Market", FEDS 2026-023 — https://www.federalreserve.gov/econres/feds/files/2026023pap.pdf
- Birru, "Day of the Week and the Cross-Section of Returns" — https://www.aeaweb.org/conference/2017/preliminary/paper/fNkhhEd6
- "Reversal of Monday returns: It is the afternoon that matters", FRL 2024 — https://www.sciencedirect.com/science/article/pii/S1544612324005555
- Dim, Eraker, Vilkov, "0DTEs: Trading, Gamma Risk and Volatility Propagation" — https://papers.ssrn.com/sol3/Delivery.cfm/4692190.pdf?abstractid=4692190 ; Cboe, "0DTE Index Options and Market Volatility" — https://cdn.cboe.com/resources/education/research_publications/gammasqueezes.pdf ; Quantpedia summary https://quantpedia.com/do-sp500-0dtes-options-increase-market-volatility/
- "Do leveraged ETFs really amplify late-day returns and volatility?", JFM 2018 — https://www.sciencedirect.com/science/article/abs/pii/S1386418117302604
- Julin, "Intraday Momentum and Return Predictability" (OMXS30 and constituents, 2023–24 thesis) — https://www.diva-portal.org/smash/get/diva2:1878991/FULLTEXT01.pdf
- Li, Sakkas, Urquhart, "Intraday time series momentum: Global evidence…", JFM 2022 — https://www.sciencedirect.com/science/article/abs/pii/S138641812100001X
- "Warp speed price moves: Jumps after earnings announcements", arXiv 2601.08962 — https://arxiv.org/pdf/2601.08962
- Unverified VWAP-band claim (content-farm pages, listed only to document that no primary source exists) — https://chartswatcher.com/pages/blog/a-practical-guide-to-vwap-strategy-trading ; https://www.tradezella.com/blog/vwap-trading-strategy
