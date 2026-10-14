# recession-watch calibration — 2026-10-15

> research only. nothing here changes the live rubric, the `recession-watch` skill or the timer. code:
> `research/macro/calibrate.py` (rerunnable). raw output: `research/macro/results/*.json|md`. every number
> below is hindsight on a small sample: **3 recessions and 9 drawdowns of ≥15% in 30 years**. read the
> tables as "did it ever work, and how noisy is it", not as calibrated probabilities.

## bottom line

1. **The current rubric is a coincident "you're in it" gauge, not a warning.** Out of sample (2016–26),
   BASE ≥6 (warning) and ≥7 (critical) warned 0 of 1 recessions and 0 of 4 ≥15% drawdowns at least 14
   days before the 15% line was crossed (5 and 3 false episodes on the recession target). Before the
   2018, 2020 and 2022 peaks it read **0, 1 and 0**.
2. **Only one input leads recessions: the yield curve.** It works only when measured robustly. The live
   streak rule missed 2019: one positive day (2019-07-23) split a 4.5-month inversion into streaks of 60
   and 78 days, so it never scored 2 before the 2020 recession. A 63-session average catches 2001, 2008
   and 2020 (median lead 8.4 months). It also produced the record 2022–24 false alarm.
3. **Absolute credit thresholds carry no information.** HY OAS was ≥500 bp on 39% of days since 1997, and
   the live credit component's lift for recessions is 1.1×, i.e. about the base rate. *Changes* in
   spreads do better: HY at least 100 bp above its 1-year low gives 1.7× for recessions and 3/4 of the
   2016–26 drawdowns.
4. **Drawdowns are not forecastable 6 months out with these data.** The best drop signals (S&P 1-month
   return, 20-day realized vol, HY widening) usually fire with the index already 7–8% off its high.
   Their value is conditional: of the 21 declines since 1997 that reached −8%, 9 went on to −15% and 4 to
   −20%. **When the curve signal was on, 3 of 7 reached −20%, against 1 of 14 when it was off.** Deep
   drawdowns are recession drawdowns.
5. **The fitted weights are underdetermined and overfit.** 9,558 weight vectors are perfect in-sample for
   recessions, and their median out-of-sample score is 0.0. The best in-sample drawdown fit scores 1.00 in
   training and 0.00 in testing. Use few components, chosen on economic grounds.

## data used

| source | series | notes |
|---|---|---|
| FRED CSV | T10Y3M, T10Y2Y, SAHMREALTIME, ICSA, UNRATE, VIXCLS, VXVCLS (2007-12→), DGS10, DFF, USREC, UMCSENT, PERMIT, AWHMAN, NFCI, STLFSI4, T10YIE, BAA10Y, CPIAUCSL | cached in `data/macro/series/` (gitignored) |
| HY OAS `BAMLH0A0HYM2` | **FRED now serves only 2023-10 onward** (ICE licence) | 1996-12-31 → 2023-12-11 from the Internet Archive's copy of FRED's own `data/BAMLH0A0HYM2.txt`, spliced with current FRED. Overlap of 41 days, max difference 0.00 |
| S&P 500 daily | FRED `SP500` only goes back to 2016. Stooq now returns a JS challenge. **Yahoo chart API `^GSPC` worked (1990→)** | matches FRED SP500 within 0.12% on every day of the 2016–26 overlap. A price index, not the SPY the live script uses |
| breadth | **not testable before 2021** (our 59-name cache starts 2021-06) | PROXY = share of the last 50 sessions the S&P closed above its 200-day (<30% → 1, <20% → 2). Against real breadth 2022-03→2026-10: same score on 87% of days, correlation 0.74 |

Point in time: daily series lagged 1 day; claims +5 days (Thursday release); NFCI +5; STLFSI +6; monthly
series +31 to +49 days. SAHMREALTIME is already a real-time vintage. **NFCI and STLFSI are re-estimated
history, so their backtests flatter them.** ICSA and PERMIT are latest vintage (small revisions).

Targets:
- **(A)** an NBER recession starts within 12 months (USREC starts 2001-04, 2008-01, 2020-03). Days inside
  a recession are excluded, and labels after 2025-09 are unknown.
- **(B)** a ≥15% (≥20%) drawdown from the trailing 252-session high peaks within 6 months, or is under
  way but has not yet hit the threshold. A warning must come ≥14 days before the cross.
- Drawdowns: 1998, 2000–02, 2007–09, 2010, 2011, 2018, 2020, 2022, 2025 (≥20%: 2000, 2007, 2020, 2022).
  **2015–16 is not an event** (maximum 14.2%). Training 1997–2015, test 2016–2026.
- Alarm days are merged into episodes (gaps ≤21 sessions): *false* if no day was labelled positive,
  *late* if the episode started inside an event.

## episode table — current rubric BASE (0–8), reconstructed daily

| episode | event | −12m | −6m | −3m | −1m | event | at 15% cross | first ≥4 | first ≥6 | first ≥7 | max | what fired first (months before event) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recession 2001 | 2001-04-01 | 3 | 3 | 8 | 8 | 8 | – | 2000-04-14 | 2000-11-13 | 2000-11-20 | 8 | credit 31 (always on), curve 12, HY-rise 12 |
| recession 2008 | 2008-01-01 | 2 | 2 | 3 | 2 | 2 | – | 2007-08-14 | 2008-01-17 | 2008-01-17 | 8 | curve-avg 24, HY-rise 5; labor −0.2 |
| recession 2020 | 2020-03-01 | 1 | 2 | 1 | 1 | 4 | – | 2020-02-27 | 2020-04-08 | 2020-04-21 | 8 | curve-avg 14 (live curve: flat only); labor −0.8 |
| dd 1998 (−19%) | 1998-07-17 | 0 | 0 | 0 | 0 | 0 | 4 | 1998-08-26 | – | – | 5 | everything after the peak |
| dd 2000–02 (−35%) | 2000-03-24 | 2 | 3 | 1 | 2 | 3 | 8 | 2000-04-14 | 2000-11-13 | 2000-11-20 | 8 | vol/momentum 2, curve 0 |
| dd 2007–09 (−53%) | 2007-10-09 | 1 | 2 | 2 | 3 | 3 | 7 | 2007-08-14 | 2008-01-17 | 2008-01-17 | 8 | curve 21, HY-rise 2.5 |
| dd 2010 (−16%) | 2010-04-23 | 8 | 4 | 4 | 4 | 4 | 6 | (2009) | (2009) | (2009) | 8 | post-crisis carry-over |
| dd 2011 (−19%) *no recession* | 2011-04-29 | 4 | 2 | 1 | 1 | 1 | 4 | (2010) | (2010) | – | 6 | price signals after the peak |
| dd 2018 (−20%) | 2018-09-20 | 0 | 0 | 0 | 0 | 0 | 5 | 2018-12-18 | 2018-12-21 | 2019-01-02 | 7 | nothing before the peak |
| dd 2020 (−34%) | 2020-02-19 | 4 | 2 | 2 | 0 | 1 | 4 | – | 2020-04-08 | 2020-04-21 | 8 | curve-avg 14 |
| dd 2022 (−25%) | 2022-01-03 | 2 | 0 | 0 | 0 | 0 | 4 | 2022-05-03 | 2022-06-17 | 2022-09-01 | 7 | equity 1.1; curve −6.7 (after) |
| dd 2025 (−19%) | 2025-02-19 | 2 | 4 | 3 | 3 | 3 | 4 | (2024-08) | – | – | 5 | curve 31 (post-inversion tail) |
| FA 2015–16 (−14%) | 2015-05-21 | 0 | 1 | 1 | 1 | 1 | – | 2015-08-24 | 2016-02-12 | – | 6 | credit (absolute level) |
| FA 2019 inversion | 2019-05-23 | 0 | 3 | 4 | 1 | 2 | – | 2018-12-18 | 2018-12-21 | 2019-01-02 | 7 | (the 2020 recession came, for another reason) |
| FA 2022–24 inversion | 2022-10-25 | 0 | 2 | 6 | 5 | 6 | – | 2022-05-03 | 2022-06-17 | 2022-09-01 | 7 | curve, equity: **no recession** |
| FA 2023 SVB | 2023-03-08 | 3 | 5 | 6 | 3 | 3 | – | 2022-05-03 | 2022-06-17 | 2022-09-01 | 7 | no 15% drawdown, no recession |
| FA 2024-08 Sahm | 2024-08-02 | 2 | 2 | 3 | 3 | 3 | – | 2023-10-23 | – | – | 5 | curve + labor; no recession |

Proposed scores on the same episodes (paths at −12/−6/−3/−1 months/event/15% cross; from `episodes.md`):
- **R (recession):** 2001: 3/6/7/7/7, first ≥6 on 2000-09-29 (6 months ahead). 2008: 4/4/5/6/6, first ≥6
  on 2007-08-17 (4.5 months ahead). 2020: 0/4/4/4/5, first ≥4 on 2019-06-19 (8.4 months ahead; never ≥6
  before the start).
  False alarms: the 2022–26 curve episode held R ≥4 for about 4 years. R reached 6 on 2024-08-08 (the Sahm
  trigger) and on 2025-04-08.
- **D (drawdown):** D ≥3 came ≥14 days before the 15% cross in all 9 episodes. Only 2007 (2007-08-10,
  two months before the October peak) is a genuine early warning. The other "before the peak" hits
  (2000, 2010, 2018, 2020) were earlier wobbles inside the 6-month window, followed by new highs. In the
  declines themselves D warned at −7% to −10% from the peak: 1998 −9.7%, 2011 −6.8%, 2022 −9.3%, 2025
  −8.6%. From the warning close, a further 11–18% fall followed (2000: −38%, 2007: −54%).

## signal quality

Lift is the precision on labelled days divided by the base rate. "warned train / test" counts events
warned ≥14 days before the cross (B), or before the start (A). For B, the count of warnings that came
before the peak (full period) is in parentheses.

**(A) recession within 12 months** (base rate 0.11 of labelled days):

| signal | lift | warned train / test | median lead (mo) | false episodes train / test | episode prec | days on |
|---|---|---|---|---|---|---|
| live curve = 2 (≥90-day streak) | 3.1× | 2/2 / **0/1** | 8.7 | 0 / 1 | 0.67 | 14% |
| **curve: 63d avg inverted now or ≤12m ago** | 3.2× | 2/2 / **1/1** | 8.4 | 0 / 1 | 0.75 | 29% |
| live credit = 2 (HY ≥500 bp) | 1.1× | 2/2 / 0/1 | 6.8 | 8 / 6 | 0.13 | 39% |
| HY ≥100 bp above 1-year low | 1.7× | 2/2 / 1/1 | 11.7 | 10 / 4 | 0.32 | 37% |
| live labor ≥1 / = 2 | 0.4× / 0.0× | 1/2 / 0/1 ; 0/2 / 0/1 | 3.3 | 1 / 2 | 0.25 | 25% |
| live equity stress ≥1 | 0.9× | 2/2 / 1/1 | 9.8 | 29 / 17 | 0.13 | 29% |
| breadth PROXY ≥1 | 0.9× | 1/2 / 1/1 | 8.2 | 6 / 3 | 0.18 | 20% |
| NFCI ≥0 (revised) | 4.1× | 2/2 / 0/1 | 7.3 | 1 / 0 | 0.67 | 9% |
| current BASE ≥4 / ≥5 | 0.8× / 1.1× | 2/2 / 1/1 ; 2/2 / 0/1 | 4.6 / 8.0 | 4 / 8 ; 5 / 9 | 0.29 / 0.22 | 28 / 21% |
| **current BASE ≥6 / ≥7** | 1.2× / 2.1× | 1/2 / 0/1 ; 1/2 / 0/1 | 4.6 / 4.3 | 5 / 5 ; 1 / 3 | 0.09 / 0.20 | 17 / 12% |

**(B) ≥15% drawdown within 6 months** (base rate 0.29):

| signal | lift | warned train / test (before peak) | median lead to cross (mo) | false episodes train / test | episode prec | days on |
|---|---|---|---|---|---|---|
| live curve ≥1 | 1.3× | 3/5 / 2/4 (5) | 7.4 | 1 / 0 | 0.89 | 25% |
| live credit = 2 | 1.2× | 4/5 / 0/4 (3) | 8.8 | 4 / 2 | 0.55 | 39% |
| HY ≥100 bp above 1-year low | 0.9× | 5/5 / 3/4 (3) | 3.2 | 5 / 2 | 0.65 | 37% |
| HY +50 bp in 1 month | 1.8× | 4/5 / 4/4 (6) | 4.7 | 12 / 7 | 0.49 | 13% |
| live equity stress = 2 | 2.0× | 4/5 / 3/4 (4) | 5.2 | 2 / 6 | 0.67 | 18% |
| S&P 1-month return ≤ −5% | 2.0× | 5/5 / 4/4 (7) | 5.5 | 12 / 6 | 0.55 | 10% |
| S&P ≥5% below its 50-day | 2.2× | 4/5 / 3/4 (3) | 3.5 | 7 / 4 | 0.61 | 7% |
| 20-day realized vol ≥20% | 1.9× | 4/5 / 4/4 (8) | 7.4 | 8 / 4 | 0.61 | 24% |
| VIX > VIX3M (2008→) | 1.6× | 2/5 / 4/4 (6) | 6.9 | 13 / 14 | 0.40 | 6% |
| NFCI +0.10 in 4 weeks (revised) | 2.8× | 2/5 / 1/4 (1) | 2.2 | 1 / 0 | 0.88 | 6% |
| breadth PROXY ≥1 | 0.5× | 1/5 / 1/4 (0) | 0.9 | 4 / 2 | 0.40 | 20% |
| current BASE ≥3 / ≥4 | 1.4× / 1.6× | 4/5 / 4/4 (6) ; 3/5 / 1/4 (3) | 6.3 / 7.8 | 8 / 6 ; 2 / 3 | 0.52 / 0.72 | 39 / 28% |
| **current BASE ≥6 / ≥7** | 1.8× / 2.0× | 2/5 / 0/4 (1) ; 1/5 / 0/4 (0) | 4.5 / 1.0 | 1 / 2 ; 0 / 0 | 0.62 / 1.00 | 17 / 12% |

≥20% drawdowns (base rate 0.17, `signal_quality.md`): BASE ≥6/≥7 warned 2/2 in training, 0/2 in testing;
the curve's lift rises to 1.7×. Dropping breadth leaves BASE no worse on either target.

## re-weighted rubric: grid fit on 1997–2015, evaluated on 2016–2026

Grid: integer weights 0–2 on the five live components plus the robust curve, HY-rise and the two best
training-period extras for each target (19,683 weight vectors per target). The warning threshold is
chosen in training. Objective = events warned × episode precision (1.0 is perfect).

| fit | weights (non-zero) | threshold | train | **test** | note |
|---|---|---|---|---|---|
| A separate | curve-avg ×1 | ≥2 | 1.00 | **0.50** | 9,558 vectors tie at 1.00 in training; their test score: median 0.00, max 0.75 |
| A, day-F1 variant | live curve 2, curve-avg 2, HY-rise 1, NFCI 2 | ≥7 | 0.67 | 0.75 | depends on revised NFCI |
| B15 separate | 1-month return 1, permits 2 | ≥2 | 1.00 | **0.00** | housing-cycle overfit (2 ties) |
| B15, day-F1 variant | live curve 2, credit 2, labor 2, equity 1, curve-avg 2, 1-month return 1 | ≥8 | 0.60 | 0.50 | 2/4 test drops, 0 false |
| **one blended score** | live curve 1, labor 1 | ≥2 | A 1.00, B15 0.60 | **A 0.00, B15 0.25, B20 0.00** | |
| current BASE ≥6 | — | ≥6 | A 0.08, B15 0.33 | **A 0.00, B15 0.00** | |

**Two scores against one blended score:** the separate recession fit beat the blended one on held-out
recessions (0.50 vs 0.00). On drawdowns, both fitted versions are poor (0.00 separate, 0.25 blended).
With 1 test recession and 4 test drawdowns this is not a meaningful contest. The economic case is
clearer than the statistics: the two targets are driven by different signals with different lead times.
The curve leads recessions by 8–24 months, while price, volatility and spread *changes* are coincident
with drops. A blend dilutes both.

Pre-declared parsimonious designs (5 per target, each with a 0–8 base; design and threshold chosen in
training; `proposal.md`):
- **Recession:** all five designs (curve ×2 plus two of HY-rise, labor, claims, permits, live credit)
  are perfect in training at ≥5. In testing at ≥4, the four that use the 63-session curve hit 1/1 but
  carry the 2022–26 false episode; the one using the live streak curve hits 0/1.
- **Drawdown:** the training-best design was 1-month return + 20-day vol + HY-rise + STLFSI at ≥2 (train
  0.69, test 0.64). Replacing the revised STLFSI with the live equity-stress component (post hoc) gives
  train 0.60 and test 0.58 at ≥3.

## faster signals: which earn a place

- **Earn a place (drawdown score):** S&P 1-month return (≤−5%: 9/9, lift 2.0×; ≤−10%: lift 3.4×, 0
  false, late); 20-day realized vol ≥20% (8/9, lift 1.9×, on 24% of days); HY rise vs its 1-year low;
  the live equity-stress component.
- **Promising, not usable as-is:** NFCI/STLFSI (best precision, re-estimated history); HY +50 bp in a
  month (good recall, 19 false episodes).
- **No place:** VIX > VIX3M (4/4 in testing but 13–14 false episodes per half, 2008→ only); the claims
  "Sahm-like" rule (lift 0.7× drawdowns, 1.8× recessions, coincident); permits ≤−15% y/y (lift 3.4× for
  recessions in-sample, but 0/1 and 0/4 in testing); sentiment; breadth proxy (0.5×).

## news layer — case studies (hindsight-contaminated)

From ~35 web searches by a research subagent, scoring the live 0–2 rule as a diligent daily reader would
have applied it. The searches knew the outcomes, so they find the headlines that mattered and miss the
noise. Real-time "first 1" dates would be *earlier* and false 2s *more frequent* than shown.

| episode | drop start | news first 1 → first 2 | quant on the same dates (BASE / R / D) | verdict |
|---|---|---|---|---|
| 2007 subprime | 07-19, 10-09 | 06-22 (Bear funds) → 07-26 (Countrywide, MBS downgrades) | 2/4/0 → 2/5/1; D ≥3 on 08-10 | news 2 led D by 2 weeks and the October peak by 2.5 months. R already 4–5 (curve) |
| 2008 Bear | in a bear | 03-05 → 03-13 | 03-14: 7/7/6 | both coincident; news 2 at the leg's low |
| 2008 Lehman | 09-12 | → 07-11 (IndyMac) | 8/4/5 from July | both saturated; 0–2 could not tell IndyMac from Lehman |
| 2011 downgrade | 07-22 | 07-13 → 07-29 | 07-13: 2/1/1; 07-29: 3/0/1; D ≥3 on 08-02 | news led D by 4 days to 3 weeks |
| 2015 yuan (−12%, not an event) | 08-18 | 07-27 → 08-24 | 08-24: 4/2/6 | false positive for both |
| 2018 Q4 | 09-20 | 09-17 → December | 0/0/0 until 10-10; 12-24: 6/2/7 | both late; the news 2 came at the bottom |
| 2020 COVID | 02-19 | 01-23 → 02-17 | 02-19: 1/4/0; D ≥3 on 02-25; 02-28: 4/5/5 | **best case: news 2 led D by about 1 week** |
| 2022 rates | 01-03 | 01-05 → 02-24 | 0/0/0 → 02-24: 2/0/3 | both late (blind to a rates-driven bear) |
| 2023 SVB (no event) | 03-06 | 03-02 → 03-10 | 03-10: 5/4/3 | false positive for both; 2 months at news 2 |
| 2025 tariffs | 02-19 | about 02-01 → 02-21–28 | 02-19: 3/5/0; D ≥3 on 03-10; 04-08: 4/6/8 | news led D by about 2 weeks; maxed out at the low |

Other loud scares with news at 2 and no event: Brexit (2016-06-24), 2019 inversion and trade war, and the
2024-08-05 carry unwind (quant 5/5/3 that day).

The news layer reached 1 before 7 of 10 drops, but 2 *before* a drop only in 2007 (first leg), 2020 and
2008 (quant already at its ceiling), with at least 5 loud 2s that went nowhere. It is roughly coincident:
1–3 weeks ahead of the drawdown score at best, and noisier.

**Weight:** keep it at **0–2 on the drawdown score, but gated**. It counts only when D base ≥2, and it can
never create a critical alert on its own. Give it **0–1 on the recession score**: no case study shows news
leading the curve, and the scale saturates. **Not 0–3:** the case studies show the extra point would
mostly amplify false positives.

## PROPOSAL (not applied)

Two scores, each a 0–8 base plus a qualitative add-on, reported side by side. Both are computed from
FRED and our own data with no new dependencies (HY OAS needs the spliced history only for backfill; live,
FRED's 3-year window is enough). Today (2026-10-13): **R = 0, D = 0**.

**(a) Recession score R — "is the cycle turning?"** (slow; target: recession within 12 months)

| component | 0 / 1 / 2 | weight | replaces |
|---|---|---|---|
| curve: T10Y3M | 2 = 63-session average < 0 now or at any time in the last 12 months; 1 = today < 0.25 (inverted or flat) | **×2** (0/2/4) | the live streak rule (a one-day un-inversion reset it in 2019) |
| credit turn: HY OAS minus its 252-session minimum | ≥100 bp → 1; ≥200 bp → 2 | ×1 | the absolute 400/500 bp levels (lift 1.1×) |
| labor: live definition (Sahm 0.3/0.5, claims ≥20% y/y add-on, cap 2) | unchanged | ×1 | — (coincident; kept as confirmation) |

- Equity stress and breadth are dropped from R (lift ≤0.9× for recessions).
- Qualitative q_R is 0–1.
- R ≥4 = *late-cycle watch* (info, on crossing): 3/3 recessions, median lead 8.4 months, 6 false
  episodes (one lasting about 4 years, 2022–26), on 35% of days.
- R ≥6 = **warning**: training 2/2 (median lead 5.3 months), 0 false; testing 0/1, 2 false (Aug–Nov 2024,
  Apr 2025); on 11% of days.

**(b) Drawdown score D — "is a ≥15% drop under way or imminent?"** (fast; target: drawdown within 6 months)

| component | 0 / 1 / 2 | weight |
|---|---|---|
| S&P 1-month (21-session) return | ≤ −5% → 1; ≤ −10% → 2 | ×1 |
| S&P 20-day realized vol (annualized) | ≥20% → 1; ≥30% → 2 | ×1 |
| HY OAS minus its 252-session minimum | ≥100 bp → 1; ≥200 bp → 2 | ×1 |
| equity stress (live: drawdown 8/15%, >20 sessions below the 200-day, VIX ≥30, VIX > VIX3M; cap 2) | unchanged | ×1 |

- Qualitative q_D is 0–2 and counts only if D base ≥2.
- D base ≥3, or D base 2 with q_D = 2 → **warning**: 9/9 drawdowns ≥14 days before the 15% line, usually
  at −7% to −10%; episode precision 0.61, 12 false episodes in 30 years (testing 4/4, 5 false); 24% of days.
- D base ≥5 → **critical**: precision 0.75 but only 5/9 in time (1/4 in testing), coincident; 12% of days.
- **Escalation:** D ≥3 while R ≥4 → **critical**. At the first −8% day, R ≥4 was followed by −20% in 3 of
  7 cases against 1 of 14 when R <4; this rule warned 2000, 2007 and 2020 (the last via an Aug-2019 wobble), not 2022.
- **Notify on level changes, not daily.** Breadth stays in the report as context only.

**Not proposed:** NFCI and STLFSI as scored inputs (revised history; display them as context only), VIX
term structure as a separate component (it is already inside equity stress), and any weight fitted by the
grid.

## what could not be tested

Real breadth before 2021 (proxy only); real-time vintages of NFCI, STLFSI, claims and permits; SPY versus
the S&P price index (immaterial for drawdowns); recessions after 2025-09 (NBER lag); statistical
significance (n = 3 recessions, 9 drawdowns). The news layer cannot be tested as a statistic from
hindsight: start logging q_R and q_D separately from today.
