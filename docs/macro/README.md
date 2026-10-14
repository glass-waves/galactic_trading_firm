# recession watch

a daily, **information-only** macro checklist for the owner. nothing it produces is read by
the trading engine, any `config_versions` row, any gate, or any script under `scripts/pipeline/`.
it exists purely to put one disciplined, cited number in front of a human each morning.

**this is a checklist, not a forecast.** these indicators (yield curve, credit spreads, the
Sahm rule, equity drawdown/breadth) have historically *led* recessions by anywhere from a few
months to over a year, and have produced real false positives (e.g. 2019's inversion). a high
score means "the classic late-cycle checklist lit up," not "a recession starts this week."
nothing here should change a position, a config, or a trade.

## pipeline

```
scripts/macro/fetch_indicators.py   (05:45 PT weekdays, no LLM)
        │  fetches FRED series + reads research/swing/daily/*.csv
        │  computes rubric BASE score (0-8)
        ▼
data/macro/<date>.json, data/macro/history.csv (base + components row)
        │
        ▼
.claude/skills/recession-watch  (claude check-in, same timer run)
        │  reads today's json, does <=8 web searches for 24h news,
        │  assigns qualitative 0-2, FINAL = min(10, base + qualitative)
        ▼
docs/macro/<date>.md (the report) + history.csv `final` column + notify.sh
```

## sources (all free, no API key)

| series | FRED id | what |
|---|---|---|
| 10y-3m spread | `T10Y3M` | primary yield-curve signal used by the rubric |
| 10y-2y spread | `T10Y2Y` | fetched and cached for context; not scored |
| HY OAS | `BAMLH0A0HYM2` | ICE BofA high-yield option-adjusted spread, **reported in percent by FRED — this script multiplies by 100 to get bps** before comparing to the bps thresholds below |
| Sahm rule | `SAHMREALTIME` | real-time Sahm rule indicator |
| initial claims | `ICSA` | weekly initial jobless claims |
| unemployment rate | `UNRATE` | fetched/cached; not currently scored, available for the qualitative read |
| VIX | `VIXCLS` | CBOE volatility index |
| 10y yield | `DGS10` | fetched/cached; context for the qualitative read |
| fed funds rate | `DFF` | fetched/cached; context for the qualitative read |
| VIX 3-month | `VXVCLS` | CBOE 3-month VIX; term-structure proxy. if FRED ever drops this series the component that uses it (equity stress term-structure add-on) is skipped, not scored as a penalty |
| breadth / SPY drawdown | `research/swing/daily/<SYM>.csv` | our own daily bar cache, 59 names + SPY (see below) |

each FRED series is cached at `data/macro/series/<SERIES>.csv` and only refetched if the cache
file is older than 20 hours — so re-running the script mid-day does not hammer FRED. a failed
fetch falls back to the stale cache if one exists, else the component is marked `"n/a"` and
scores 0 (conservative); the script never crashes on a missing series.

**known caveat**: `research/swing/daily/` is a one-time/manual fetch cache
(`research/swing/fetch_daily.py`), not auto-refreshed by anything in this pipeline. the
`equity_stress` and `breadth` components are only as fresh as that cache's last update — the
JSON output's `swing_cache_as_of` field and the script's stderr both flag when it is stale
relative to the run date. refreshing that cache is out of scope for recession-watch; if it goes
stale for an extended period, re-run `research/swing/fetch_daily.py` by hand.

## rubric (BASE score, 0-8, each component 0-2)

**yield curve** (`T10Y3M`, in percentage points — 25bps = 0.25):
- inverted (value < 0) for >= 3 months (90 days), currently still inverted → **2**
- un-inverted now, but a >=3-month inversion streak *ended* within the last 6 months → **2**
- currently inverted but < 3 months, OR not inverted and "flat" (|value| < 0.25) → **1**
- otherwise → **0**

**credit** (HY OAS, converted to bps):
- >= 500 bps → **2**
- 400-500 bps, OR +100 bps over the trailing 3 months → **1**
- otherwise → **0**

**labor** (Sahm rule + claims):
- Sahm >= 0.5 → **2**; Sahm 0.3-0.5 → **1**; else **0**
- claims 4-week average up >= 20% y/y → **+1** (this add-on, not the Sahm score alone)
- total capped at **2**

**equity stress** (SPY from the daily cache, VIX, VIX3M):
- SPY drawdown from its trailing 252-session high >= 15% → **2**
- drawdown 8-15%, OR more than 20 consecutive sessions with close below the 200-day average → **1**
- VIX >= 30 → **+1**
- VIX3M < VIX (inverted term structure) → **+1**
- total capped at **2**

**breadth** (59-name universe, excluding SPY, from the daily cache):
- < 30% of names above their 200-day average → **1**
- < 20% → **2**
- (percent above the 50-day average is also computed and reported, but does not feed the score)

**BASE = sum of the five components, capped at 8.**

## qualitative add-on and FINAL score (done by the `recession-watch` skill, not this script)

the skill reads today's BASE, does a small number of web searches for genuine 24-hour news
(credit/banking stress, sovereign/large-issuer defaults, oil/energy shocks, geopolitical
escalation with market impact, broad earnings-guidance cuts, Fed/rate surprises, major data
releases), and assigns a qualitative adjustment of **0-2** with citations. **FINAL = min(10,
BASE + qualitative)**.

## file layout

| path | written by | tracked in git |
|---|---|---|
| `data/macro/series/<SERIES>.csv` | `fetch_indicators.py` | no (regenerable cache; 20h refresh) |
| `data/macro/<date>.json` | `fetch_indicators.py` | no (derived; `history.csv` is the durable record) |
| `data/macro/history.csv` | `fetch_indicators.py` (base + components row) and the `recession-watch` skill (adds `final`, same row, upsert by date) | **yes** |
| `docs/macro/<date>.md` | the `recession-watch` skill | **yes** |
| `docs/macro/README.md` (this file) | humans / this change | **yes** |

`history.csv` columns: `date,base,yield_curve,credit,labor,equity_stress,breadth,qualitative,final`.
one row per calendar date — both writers **upsert by date** (find the row for today, replace
it) rather than append, so re-runs never duplicate a day.

## notification thresholds (the skill, via `scripts/notify.sh`)

score 0-5 → `info`; score 6 → `warning` with reasons; score >= 7 → `critical` with reasons. a
score >= 6 report also carries a required plain-language explanation of why.
