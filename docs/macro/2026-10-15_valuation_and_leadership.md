# valuation, leadership rollover and an AI-confidence line — 2026-10-15

> research only, follow-up to `2026-10-15_calibration.md`. nothing here changes the live rubric, the
> `recession-watch` skill or the timer. code: `research/macro/valuation_leadership.py` (rerunnable, ~10 s
> with caches); raw output: `research/macro/results/{valuation,leadership}.{json,md}`, `current_readings.json`.
> sample: 1990–2026, **4 recessions, 10 drawdowns of ≥15%, 23 declines of ≥8%**. treat every rate as an
> anecdote with a denominator.

## bottom line

1. **Valuation is context, not a trigger and not a multiplier.** CAPE in its trailing-30-year top decile
   gives 1.5× lift for a ≥15% drawdown within 6 months, but it was on continuously from April 1995 to February 2001.
   Once a decline reaches −8%, CAPE does not predict depth: Spearman −0.09 against max drawdown. Every
   ≥30% fall came with the curve signal on (3 of 9, against 0 of 14 with it off), at any valuation. CAPE
   adds nothing to the curve for recessions: test AUC 0.71 → 0.71, and its coefficient is negative. What
   valuation does carry is the **long-horizon** return: at month-ends with CAPE ≥35, the median
   forward-36-month return was −13% and 60% were negative (effectively two episodes, 1998–2000 and 2021–22).
2. **"Record" is true of market cap to GDP, not CAPE.** CAPE is 41.4, the second highest since 1871 after
   Dec 1999 (44.2); its trailing-30y percentile is 0.96. The Z.1 equities/GDP proxy, at 2.65, *is* a record
   (percentile 1.00).
3. **Leadership rollover does not reliably lead the index.**
   - The Nasdaq-100 peaked ≥10 sessions before the S&P in 2 of 10 drawdowns (median lead 0).
   - Semis peaked first in 6 of 9 (median 49 sessions). But on all S&P new-high days, the SOX was already
     ≥10% off its own high 17% of the time, against 2 of 9 at the real peaks: no enrichment.
   - The best semis rule (SOX 3-month relative return ≤ −10%) caught 7 of 10 drawdowns, with 12 false
     episodes in 36 years and 1.7× lift. Out of sample it caught 3 of 4 with 1 false episode, but only 1 of
     those 3 came before the index was −5%.
   - Adding leadership to D changed nothing out of sample.
4. **Today the levered AI-infra layer has cracked; the leaders have not.** The 9-name basket's median stock
   is 54% below its 1-year high (−16% vs SMH in a month), while NVDA is −4.5%, our mega-cap basket and the
   Forge private-market index are at highs, and semis are −17% with the S&P within 0.2% of its high. The
   basket has no history and fell 58% in Mar–Apr 2025 before recovering fully: a reason to watch, not a forecast.

## data

| series | source | cached as | notes |
|---|---|---|---|
| Shiller CAPE, monthly 1871→ | multpl.com `shiller-pe/table/by-month` (Shiller's series; Dec 1999 44.19, Mar 2009 13.32 and Sep 1929 32.56 match Shiller) | `CAPE_multpl.csv` | Yale's `ie_data.xls` was last saved 2023-09 and is BIFF (no stdlib parser). A month counts from the 1st of the next month |
| equities / GDP | FRED `NCBEILQ027S` / `GDP` (Wilshire `WILL5000INDFC` is **404 on FRED**) | `NCBEILQ027S.csv`, `GDP.csv` | Quarterly, usable 75 days after quarter end, moved with the S&P since the quarter end |
| ^GSPC, ^NDX (1985→), ^SOX (1994-05→), SPY, RSP (2003→, total return), SMH; ORCL, VRT, CRWV, NBIS, APLD, IREN, CIFR, WULF, CORZ, NVDA, ^FPMI, DXYZ | Yahoo chart API | `yahoo_*.csv` | Today's in-progress bar is dropped, so readings are **as of the 2026-10-13 close** |
| MEGA8 (AAPL MSFT NVDA AMZN GOOGL META AVGO TSLA), SEMI8 | our `research/swing/daily` cache, 2021-06→ | — | Equal weight, rebalanced daily. Today's names, so **hindsight-selected** |
| T10Y3M, USREC | FRED (as in the main study) | — | Curve = 63-session average inverted now or within 12 months |

Every signal is lagged one session. Targets and episode merging follow `calibrate.py`: B15 = a ≥15% S&P
drawdown peaks within 6 (or 12) months, with a warning counting only if it comes ≥14 days before the 15%
cross; A = an NBER start within 12 months. Train 1990–2015, test 2016–2026.

## A. valuation

**(i) Timing.** Lift, events warned and false episodes, 1990–2026, B15 within 6 months (base rate 0.25):

| signal | lift | warned | false ep | days on | train / test warned |
|---|---|---|---|---|---|
| CAPE pct30 ≥0.8 (top quintile) | 1.2× | 7/10 | 6 | 49% | 3/6 / 4/4 |
| CAPE pct30 ≥0.9 (top decile) | 1.5× | 4/10 (98, 00, 22, 25) | 1 | 23% | 2/6 / 2/4 |
| CAPE ≥30 (level) | 1.9× | 6/10 | 2 | 30% | 2/6 / 4/4 |
| equities/GDP pct30 ≥0.9 | 1.2× | 7/10 | 3 | 55% | 3/6 / 4/4 |

At month-ends in the top decile, 54% were followed by a ≥15% peak within 12 months, against 33% overall.
Those months come from three clusters, though (1995–2001, 2020–22, 2024–26). The median lead to the cross
was 8.8 months, and the signal was continuously on from April 1995 to February 2001. **Verdict: weak.** It is on so long
that it cannot say *when*. The equities/GDP proxy has trended up and was ≥0.9 on 96% of test days, which
makes it useless as a trigger.

**(ii) Severity given −8%.** These are the 23 declines that reached −8%, split into terciles by the
trailing-30y CAPE percentile at the peak:

| tercile (pct30) | n | ≥15% | ≥20% | ≥30% | median max dd | median sessions to trough / to recover | recessionary |
|---|---|---|---|---|---|---|---|
| low (0.49–0.77) | 7 | 2 | 0 | 0 | 11.1% | 63 / 148 | 2 |
| mid (0.81–0.90) | 8 | 4 | 2 (07, 20) | 2 | 14.5% | 38 / 146 | 2 |
| high (0.93–1.00) | 8 | 3 | 2 (00, 22) | 1 (00) | 11.4% | 40 / 70 | 1 |

| at the −8% day | n | ≥20% | ≥30% |
|---|---|---|---|
| curve on (=2), CAPE top decile | 3 | 1 (2000 −49%) | 1 |
| curve on, CAPE below | 6 | 2 (2007 −56%, 2020 −34%) | 2 |
| curve off, CAPE top decile | 6 | 1 (2022 −25%) | 0 |
| curve off, CAPE below | 8 | 0 (2011 −19%, 2018 −20%) | 0 |

The Spearman correlation of CAPE percentile with max drawdown is −0.09, and of CAPE level 0.06. The curve
correlates at +0.25. High-valuation declines recovered *faster* (median 70 sessions, driven by 1997–99);
2000 is the exception at 1,803 sessions. **Verdict: not a multiplier.** Depth is a recession property, and
valuation is not.

**(iii) Recession within 12 months.** The robust curve alone warned 4/4 (1990, 2001, 2008, 2020) with 1
false episode, 7.8 months median lead, 3.8× lift. Requiring CAPE ≥0.8 as well drops 1990 and adds 2 false
episodes; CAPE ≥0.9 alone warns 1/4. Monthly logit (train 1990–2015): adding CAPE lifts the in-sample
log-likelihood −46.4 → −43.1 but with a *negative* sign; test AUC 0.71 → 0.71; CAPE alone AUC 0.55 train /
0.37 test. **Verdict: no.**

**Conclusion: context.** Show CAPE level and percentile, plus the equities/GDP percentile, as a report
line. When CAPE ≥35, add the long-horizon sentence: forward 12 months median +1%, 49% negative; forward
36 months median −13%, 60% negative, median worst drawdown 26%; effectively 2 episodes. No points.

## B. leadership rollover

Episode table. "Lead" is in sessions (+ = leaders peaked first); "dd" is the leaders' distance below their
own high on the S&P peak day; rs = the leaders/S&P relative-strength line; R = recessionary.

| drawdown | S&P peak → −5% | NDX | SOX | SOX rs | cap/EW rs | MEGA8 | SOX 3m rel ≤−8% fired before −5% |
|---|---|---|---|---|---|---|---|
| 1990-07 (−20%) R | 07-16 → 08-03 | 0 | – | – | – | – | – |
| 1998-07 (−19%) | 07-17 → 07-29 | −1 | +228 (−34%) | +228 | – | – | 1998-01-16 (6 mo) |
| 2000-03 (−35%) R | 03-24 → 04-13 | −1 | +10 (−3%) | +10 | – | – | – |
| 2007-10 (−53%) R | 10-09 → 11-07 | −16 | +59 (−9%) | +59 | −62 | – | 2007-10-12 (after the peak) |
| 2010-04 (−16%) | 04-23 → 05-06 | 0 | −1 | −37 | +200 | – | – |
| 2011-04 (−19%) | 04-29 → 06-06 | −58 | +49 (−5%) | +58 | +178 | – | 2011-05-20 (after the peak) |
| 2018-09 (−20%) | 09-20 → 10-11 | +15 (−1%) | +134 (−4%) | +75 | −25 | – | 2018-09-13 (5 sessions) |
| 2020-02 (−34%) R | 02-19 → 02-25 | 0 | 0 | +18 | −20 | – | – |
| 2022-01 (−25%) | 01-03 → 01-19 | +29 (−0.4%) | +5 | +18 | +22 | 0 | – |
| 2025-02 (−19%) | 02-19 → 03-04 | 0 | +153 (−10%) | +167 | +153 | +42 (−4%) | 2024-08-28 (5.5 mo) |

**Null check** — on all 981 S&P new 252-session highs, share with leaders already below their own high vs
at the real peaks: SOX ≥5% 27% vs 4/9; SOX ≥10% 17% vs 2/9; SOX rs line ≥10% 27% vs 4/9; cap/EW rs ≥5%
32% vs 1/7; NDX ≥5% 3% vs 0/10. Semis are usually somewhere below their high, so "semis peaked first" is
close to the base rate; the Nasdaq-100 *is* the index's top weight and peaks with it.

**Rules: a pre-registered grid, chosen on 1990–2015.** The grid covers NDX, SOX, and cap- vs equal-weight;
"−5%" means the rule fired before the S&P's first −5% close.

| rule | train warned / before −5% / false | test warned / before −5% / false | full ep prec | lift | days on |
|---|---|---|---|---|---|
| NDX ≥10% off high, S&P within 3% *(brief)* | 2/6 / 2 / 6 | 0/4 / 0 / 1 | 0.30 | 0.9× | 2% |
| NDX 3m relative ≤ −8% *(brief)* | 2/6 / 1 / 9 | 1/4 / 0 / 1 | 0.33 | 1.4× | 7% |
| SOX ≥10% off high, S&P within 3% *(brief)* | 4/6 / 4 / 18 | 2/4 / 2 / 7 | 0.29 | 1.1× | 12% |
| SOX 3m relative ≤ −8% *(brief)* | 4/6 / 3 / 17 | 3/4 / 2 / 2 | 0.41 | 1.6× | 15% |
| **SOX rs line ≥10% below 1-yr high** *(train pick)* | 5/6 / 4 / 11 | 3/4 / 3 / 10 | 0.38 | 1.3× | 41% |
| SOX 3m relative ≤ −10% *(best precision)* | 4/6 / 2 / 11 | 3/4 / 1 / 1 | 0.50 | 1.7× | 11% |
| cap/EW 3m relative ≤ −2% (best of that family) | 3/6 / 3 / 6 | 2/4 / 1 / 5 | 0.42 | 1.0× | 10% |

Recessions (12-month window): the SOX rules warned 2–3 of 4, the NDX 1-month rule 4 of 4 but with 39
false episodes. **As a +1 add-on to D** (1997–2026), with D ≥3 versus D + leadership ≥3: training was
unchanged at 5/5 (false 8 → 7); in testing it was 4/4 either way (false 5 → 7), and the number warned
before −5% did not change (3 and 2).

**Verdict.** Leadership rollover is not a usable *lead*. The rule that lands most often (the SOX rs line)
is on 41% of days. The precise rule fires about when the index cracks. The 2022 bear, the one that is most
"leaders-led" in hindsight, shows the Nasdaq-100 peaking 29 sessions early but only 0.4% below its high on
the S&P peak day. Use semis' relative strength as one input of the AI line, not of D.

## C. AI-confidence component — design (no backtest possible)

Today (2026-10-13 close, `current_readings.json`): AI-infra basket (ORCL CRWV NBIS APLD VRT IREN CIFR WULF
CORZ) median −54% from its 1-year high and −16% vs SMH over a month (since 2024-04 the median was ≥50% below
on 6% of days, ≥40% on 18%); SOX −17%, NVDA −4.5%, MEGA8 0%, NDX −2%; ^FPMI at its all-time high, DXYZ −58%
(premium collapse). Press, not in the data: on 10-14 CRWV −15.5%, ORCL −8.8%, CoreWeave 2032 notes below
80c (~15% yield; SiliconANGLE/Bloomberg); MS put ORCL 5y CDS at 260 bp from 57 bp (10-07); S&P cut ORCL to
BBB- (07-09).

| observable | source (free?) | cadence | use |
|---|---|---|---|
| leaders' relative strength (SOX, SMH, MEGA8, NVDA) | Yahoo + our cache (yes) | daily | core |
| levered AI-infra equity basket | Yahoo (yes) | daily | core |
| private-market index ^FPMI (Forge, Schwab since 2026-03) | Yahoo, weekly since 2018-12, daily since 2021 (yes) | daily | core |
| AI-infra / hyperscaler bonds (CRWV 2030–32, ORCL 2035/2055, Meta–Blue Owl "Beignet" 2049, APLD and WULF secured) | FINRA TRACE via finra.org Bond Center: free to view, non-commercial, no CSV or API per CUSIP; TRACE data products are paid | daily, manual | qualitative (a few bellwethers read weekly) |
| ORCL / CRWV CDS | none free (S&P/IHS Markit, Bloomberg); press headlines only | event | qualitative |
| IG/HY credit on FRED | ICE BofA series truncated to 3 years since Apr 2026 and no tech sub-index; Moody's BAA10Y unrestricted | daily | already in R/D, not AI-specific |
| GPU rental: Ornn OCPI H100/H200/B200 (data.ornn.com) | free JSON, last 3 months only (accumulate it ourselves); ICE futures announced | daily | qualitative (12-week change) |
| GPU rental: getdeploying.com index (88 providers) | free weekly dataset since mid-2024/2025 (history start inconsistent) | weekly | qualitative |
| Silicon Data SDH100RT / SemiAnalysis index | 30-day view or paid; Bloomberg | daily | no |
| semis implied vol: Cboe VXSMH (2025-09→), VXN | free CSV at cdn.cboe.com | daily | context |
| hyperscaler capex calendar | IR pages: META 10-21, MSFT and GOOGL 10-28, AMZN ~10-29, NVDA mid/late Nov, ORCL mid-Dec; TSMC monthly revenue ~10th | quarterly + monthly | qualitative (guide cut = hard datapoint) |
| venture funding | Crunchbase News monthly (headline), PitchBook-NVCA and CB Insights quarterly PDFs | monthly/quarterly | qualitative, slow |
| lab secondaries | Forge prices on Yahoo (OPAI.PVT, ANTH.PVT: staircases, 4–26 changes a year), Clarity/Hiive index (no download), DXYZ (quarterly NAV) | irregular | qualitative |
| allocator sentiment | BofA FMS monthly (press only; Oct: long semis most crowded, AI bubble #2 tail risk); AAII weekly (bulk download 403); NAAIM paywalled since 2026-08; CFTC NQ positioning (free weekly API) | weekly/monthly | context only (contrarian) |

**Daily quantitative core A_q (0–2)** = min(2, sum), thresholds a priori and uncalibrated (log on-rates
from day one):
- *levered layer* +1: basket median drawdown from its 252-session high ≥50% **and** median 21-session return
  vs SMH ≤ −15% (today 54% / −16% → +1);
- *leaders* +1: SOX 63-session return vs the S&P ≤ −10% (7/10 drawdowns, 12 false episodes 1990–2026), or
  MEGA8 ≥10% off its high with the S&P within 5%; +2 instead if MEGA8 or NVDA is ≥20% off (today 0);
- *private marks* +1: ^FPMI ≥10% below its 252-session high (today 0).

**Weekly cited qualitative read q_AI (0–2):** 1 = one hard, cited repricing datapoint — a hyperscaler
capex guide cut or flat guide; OCPI H100/B200 rental −15% over 12 weeks; a bellwether AI-infra bond +150 bp
in a month or below 80; a lab down-round or secondary mark −20%; a pulled or failed AI-infra financing.
2 = two independent ones, or one from a top-4 hyperscaler or NVDA. Sentiment surveys and "bubble"
commentary alone score 0.

**Combination.** A = A_q + q_AI (0–4) is a **separate report line**: not added to D (the leadership
add-on showed no gain and A has no history), not a multiplier (valuation failed that test). It acts as a
**gate**: A ≥3 can stand in for q_D = 2 in the D warning rule; A ≥3 with D ≥3 escalates to critical
(provisional, by analogy with the tested R ≥4 escalation); A ≥3 alone is an info "AI-confidence watch".

**What to expect.** *Catches:* an endogenous unwind that starts in levered infra, GPU pricing, private
marks or semis and spreads to the leaders; it names the reason when D starts moving. *Does not:* time the
index (leaders did not lead above); see exogenous shocks — COVID, tariffs, rates (D and R do); see private
repricing that public marks hide (Forge prices are staircases, NAVs quarterly). *False alarms:* the basket's
2025 round trip would have scored; semis-led rules made 12–25 false episodes in 36 years.

## PROPOSAL, revised (merges the main calibration's R/D; not applied)

Three scored lines plus one context line, reported together. **Notify only on level changes.**

| line | components (0/1/2 each) | weight / cap | levels |
|---|---|---|---|
| **R** recession (0–8 + q_R 0–1) | curve: 63-session T10Y3M average < 0 now or in last 12 months → 2, today < 0.25 → 1; HY OAS − 252-session min ≥100/200 bp; labor (live: Sahm 0.3/0.5, claims ≥20% y/y add-on, cap 2) | curve ×2, others ×1 | ≥4 late-cycle watch (info); ≥6 **warning** |
| **D** drawdown (0–8 + q_D 0–2) | S&P 21-session return ≤ −5/−10%; 20-day realized vol ≥20/30%; HY OAS − 252-session min ≥100/200 bp; equity stress (live, cap 2) | ×1 each | base ≥3, or base 2 with q_D = 2 **or A ≥3** → **warning**; base ≥5 → **critical** |
| **A** AI confidence (A_q 0–2 daily + q_AI 0–2 weekly) | levered basket (median dd ≥50% and 21-session vs SMH ≤ −15%); leaders (SOX 63-session relative ≤ −10% or MEGA8 −10% with the S&P within 5% → 1; MEGA8/NVDA −20% → 2); ^FPMI ≥10% off high | A_q cap 2 | A ≥3 → AI-confidence watch (info); never a warning alone |
| valuation (context, no points) | CAPE level, trailing-30y and since-1871 percentiles; Z.1 equities/GDP percentile | — | at CAPE ≥35, print the forward-36-month sentence |

**Escalation to critical:** D ≥3 with R ≥4 (tested: at the first −8% day, 3 of 7 such declines reached
−20%, against 1 of 14 otherwise); D ≥3 with A ≥3 (provisional).

**Qualitative layers:** q_R 0–1; q_D 0–2, counted only when D base ≥2, never a critical alone; q_AI 0–2
weekly with citations, inside A. Log all three separately and score them after 12 months.

**Report only:** the NDX, cap/EW and SOX rs-line rollover flags, VXSMH, BofA FMS and AAII.

**Today:** R = 0, D = 0, A = 1 (A_q 1 from the levered layer; q_AI is for the skill to read, and the
10-14 CoreWeave bond and ORCL CDS headlines would argue for 1–2). Valuation context: CAPE 41.4 (2nd highest
since 1871), equities/GDP at a record.

## what could not be tested

AI-specific series before 2024 (the cycle has no history); point-in-time constituents and top-10 by market
cap (proxies only; MEGA8/SEMI8 are hindsight baskets from 2021); hyperscaler credit spreads (no free
series); CAPE earnings revisions (multpl revises recent months); significance (n = 4 recessions, 10 drawdowns).
