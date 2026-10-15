# research/macro — recession-watch calibration (2026-10-15)

A one-off calibration study of the daily recession-watch rubric (`scripts/macro/fetch_indicators.py`,
`.claude/skills/recession-watch`). It never feeds the live rubric, the skill, the timer or the trader.
The write-up and the proposal are in `docs/macro/2026-10-15_calibration.md`. Nothing has been applied.

## run

```bash
python3 research/macro/calibrate.py            # uses data/macro/series/ caches younger than 7 days, fetches the rest
python3 research/macro/calibrate.py --refresh  # refetch every series
python3 research/macro/calibrate.py --no-fetch # offline; caches only
```

A run takes about 5 minutes; nearly all of it is the integer-weight grid (3^9 vectors × thresholds × 2
targets, plus the blended search). It needs only the Python 3 standard library and numpy.

## data (cached in `data/macro/series/`, gitignored)

- **FRED CSV:** the live series plus USREC, UMCSENT, PERMIT, AWHMAN, NFCI, STLFSI4, T10YIE, BAA10Y,
  CPIAUCSL and SP500. Kept for a week unless `--refresh` is passed. A file the live script also uses is
  rewritten in the same raw FRED format, so the live cache stays valid.
- **`BAMLH0A0HYM2_wayback_20231212.txt`:** FRED now serves HY OAS only from 2023-10 onward. The full
  history 1996-12-31 → 2023-12-11 comes from the Internet Archive snapshot of FRED's own text download,
  and is spliced with current FRED. The 41-day overlap matches exactly.
- **`GSPC_yahoo.csv`:** S&P 500 daily closes from 1990, from Yahoo's chart API. FRED SP500 goes back only
  to 2016 and Stooq is behind a JS challenge. It matches FRED SP500 within 0.12% over 2016–26.
- **`_calibration_daily.csv`:** the reconstructed daily panel. Columns: the current BASE and components,
  every candidate signal (0/1/2), the two proposed scores, and the raw values.

## outputs (`results/`, tracked)

| file | what |
|---|---|
| `meta.json` | data ranges, the HY splice check, the breadth-proxy check against real breadth (2022→), event lists, label counts |
| `rubric_monthly.json` | the current rubric reconstructed point-in-time: month-end BASE and components, 1997→, plus the proposed R and D |
| `signal_quality.json` / `.md` | every component and candidate at levels ≥1/≥2, and BASE ≥3..8, for targets A (recession within 12 months), B15 and B20 (≥15% and ≥20% S&P drawdown within 6 months). Metrics: day precision and lift, recall, events warned, warnings before the peak, lead, false and true episodes; full / train 1997–2015 / test 2016–26 |
| `grid.json` | integer-weight grid fit on train, scored on test (episode objective and day-F1 variant). Includes the out-of-sample spread of the tied-best set and the one-blended-score comparison |
| `proposal.json` / `.md` | pre-declared parsimonious designs: the design and threshold picked on train, with every threshold shown on train and test |
| `final_proposal.json` | the proposed recession score R and drawdown score D: levels, the escalation rule, warning timing (loss already taken, fall still ahead) and the "after an 8% decline" conditional table |
| `episodes.json` / `.md` | the episode table: BASE at −12/−6/−3/−1 months and at the event, first dates ≥4/6/7, component onsets, the R and D paths |

## conventions

- **Point in time.** Daily series are lagged 1 day. Claims are lagged 5 days, NFCI 5 and STLFSI 6.
  Monthly series are lagged 31–49 days. SAHMREALTIME is already a real-time vintage. NFCI and STLFSI are
  re-estimated history, so their results are optimistic.
- **Targets.**
  - A: an NBER (USREC) start within 365 days. Days inside a recession, and days after the last USREC
    date minus 365, are excluded.
  - B: a drawdown peak within 182 days, or a decline under way that has not yet crossed the threshold.
    A warning counts only if it comes ≥14 days before the cross. Days inside the drawdown are excluded.
    Drawdowns are measured close to close from the trailing 252-session high, and runs closer than 126
    sessions are merged.
- **Episodes.** Alarm days are merged when the gap is ≤21 sessions. An episode is *false* if it has no
  positive-labelled day, and *late* if it started inside an event.
- **Breadth** cannot be tested before 2021. The proxy is the share of the last 50 sessions the S&P closed
  above its 200-day. It gives the same score as the real 59-name breadth on 87% of days from 2022-03.
- **Small sample.** There are 3 recessions and 9 drawdowns. Every precision figure is an anecdote with a
  denominator.

## follow-up: valuation and leadership (`valuation_leadership.py`)

```bash
python3 research/macro/valuation_leadership.py            # ~10 s with caches; reuses calibrate.py's loaders and evaluate()
python3 research/macro/valuation_leadership.py --no-fetch # offline
```

Write-up: `docs/macro/2026-10-15_valuation_and_leadership.md`. New caches in `data/macro/series/`:
`CAPE_multpl.csv` (Shiller CAPE via multpl.com, 1871→), `NCBEILQ027S.csv` + `GDP.csv` (Z.1 equities/GDP;
FRED dropped Wilshire), `yahoo_*.csv` (^GSPC/^NDX from 1985, ^SOX, SPY, RSP, SMH, the AI-infra names, ^FPMI,
DXYZ). The calendar is 1990→ (4 recessions, 10 drawdowns ≥15%). Outputs: `valuation.json/.md` (CAPE timing,
depth given −8%, recession add-on), `leadership.json/.md` (rule grid fit 1990–2015 / tested 2016–26, episode
table, null check, D add-on), `current_readings.json` (today's leadership, AI-infra basket and valuation readings).
