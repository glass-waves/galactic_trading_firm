# Split adjustment of historical bars (2026-10-14)

## The bug

Every historical bar request (backtest loader, `--fetch-bars`, live warm-up) omitted `adjustment`,
so Alpaca returned `raw`: `data/bars_iex` (41 names) and `data/bars` (SIP, 12) were unadjusted (NVDA
1208.42 on 2024-06-07, 121.93 on 2024-06-10). A window spanning a split reads it as a 90 % crash.

## Splits in the caches (2022-01 → 2026-10)

Found by the old/new close factor on every symbol (a constant run ending the day before the split;
identical prices on every later day).

| name | ratio | ex-date | cache | caught by the day-over-day [0.5, 2] screen? |
|---|---|---|---|---|
| AMZN | 20:1 | 2022-06-06 | both | yes |
| SHOP | 10:1 | 2022-06-29 | IEX | yes |
| GOOGL | 20:1 | 2022-07-18 | SIP | yes |
| TSLA | 3:1 | 2022-08-25 | both | yes |
| SMH | 2:1 | 2023-05-05 | IEX | **no** (2:1 on an up day lands inside [0.5, 2]) |
| NVDA | 10:1 | 2024-06-10 | both | yes |
| AVGO | 10:1 | 2024-07-15 | both | yes |
| NFLX | 10:1 | 2025-11-17 | both | yes |
| XLE | 2:1 | 2025-12-05 | IEX | barely (×0.498) |
| XLK | 2:1 | 2025-12-05 | IEX | **no** |

Volume scales inversely; no other symbol differs on any bar.

## The fix

- `crates/backtest/src/alpaca_loader.rs`: `DEFAULT_ADJUSTMENT = Split`. `fetch_bars_range` (the `--date`
  path without `--bars-dir`) and `fetch_day_bars` use it. `fetch_bars_range_feed` now takes the
  adjustment as an argument. Dividends stay unadjusted so prices match the live stream on ex-dates.
  A 250 ms pause between pages keeps a full cache rebuild under the 200 requests/min limit.
- `crates/backtest/src/main.rs`: `--fetch-bars … --adjustment raw|split` (default `split`, any other
  value exits). The run logs `dir= feed= adjustment=`. Merging new bars never re-adjusts older ones,
  so after each fetch the run checks session-over-session closes inside the fetched range and prints
  `[fetch-bars] WARNING … possible split/basis break` for any ratio outside [0.7, 1.45]. That range
  catches 3:2 and 2:1 splits. Unit test: `session_close_jumps_flags_only_split_sized_moves`.
- `crates/data_feed/src/alpaca_feed.rs`: the live warm-up (`fetch_bars_paged`) requests `split`.
- `scripts/export_day.sh`: no flag needed (default is split); append warnings, formerly sent to
  /dev/null, now land in `data/live/<date>/bar_cache_warnings.log` + `notify.sh warning`.
- Caches were refetched 2022-01-01 → 2026-10-13. Every symbol has the same session count over the
  old range, no day-over-day ratio outside [0.5, 2], and identical prices on every post-split day.
  The old caches are kept as `data/bars_iex_raw` and `data/bars_raw` until 2026-10-21. Side findings
  (not split-related, after the 2026-09-10 sweep end; the refetch fixed both): IEX QQQ was missing
  2026-10-09; SIP days appended since 2026-09-14 held pre-revision prints (cents on a few bars).

## Restated numbers (5y IEX, 36 % sizing, 3 bps + $0.005; P&L / trades / PF · 2022 23 24 25 26)

The difference between runs was **not** confined to the weeks after each split. MACD scores are the
histogram in dollars (`macd_5min`, `macd_fast_1min`, `normalization_factor` 1.0, clamped to ±1). A
fully split-adjusted history therefore puts pre-split NVDA at $40–90 and AMZN at $80–170 instead of
the $400–900 and $1,600–3,700 the engine actually saw at the time. That changes MACD on every
pre-split bar (Δscore up to 0.9 on 2024-02-20). Whole-share sizing and per-share costs move as well.
So there are three records:

**raw** is the old record and **adj** a plain replay of the new cache. **pit** (point in time)
replays every day on the price basis in force that day: for a single-split name, the raw run before
the split and the adjusted run from the split on. Tickers replay independently (AAPL and MSFT are
bit-identical across all runs), so the composite is exact. **pit is the live-consistent record.**

| tag | raw (old) | adj cache | **pit** |
|---|---|---|---|
| `iex_v18` | +1728 / 436 / 1.37 · +991 −3 −20 +332 +429 | +1318 / 360 / 1.34 · +879 −167 −155 +332 +429 | **+1829 / 434 / 1.40** · +1089 −3 −17 +332 +429 |
| `sip_v18_vwapfix` | +2754 / 510 / 1.56 · +1108 +328 +182 +410 +726 | +1902 / 421 / 1.44 · +954 −53 −134 +410 +726 | **+2835 / 510 / 1.57** · +1259 +328 +113 +410 +726 |
| #4 vpin-0.26 | +1646 / 324 / 1.48 | +939 / 277 / 1.31 | +1646 / 324 / 1.48 |
| #6 stress-s15-core | +2003 / 459 / 1.41 | +1744 / 385 / 1.43 | +2104 / 457 / 1.44 |
| #28 size-vpin26-x1.25 | +2155 / 436 / 1.40 | +1539 / 360 / 1.34 | +2257 / 434 / 1.43 |
| #29 thrust-1h15 | +1714 / 278 / 1.63 | +1269 / 228 / 1.59 | +1812 / 277 / 1.69 |
| #44 spy-sqrt-band | +2195 / 460 / 1.45 | +1709 / 375 / 1.43 | +2296 / 458 / 1.49 |
| #45 spy-sqrt-band-1h15 | +2044 / 309 / 1.70 | +1530 / 249 / 1.68 | +2142 / 308 / 1.75 |
| #48 qqq-noise-pm-vol | +1095 / 275 / 1.50 | unchanged (QQQ never split) | unchanged |

**Trades the bug changed (raw → pit)** — small, on or just after the split day:
- `iex_v18`: two shorts leave, AMZN 2022-06-06 (−97.74) and NVDA 2024-06-10 (−3.55); net +101.
- `sip_v18_vwapfix`: AMZN 06-06 (−48) and NVDA 06-10 (−13) leave; AMZN 06-10 (+103) and NVDA
  06-10 (−82) arrive; net +81.

Across all runs: 1–4 trades, all 0–4 days after a split. Replays use an 8-day lookback, so a window
spanning a split mostly produced no entry. In the live warm-up a straddling window lasts longer.

**Gate verdicts.** `pipeline.py regate` refuses `shadow` candidates ("regate applies to
backtest_passed/backtest_failed only"), so the same gate math ran read-only (no DB writes, no stage
change), each candidate under its own gate against `iex_v18` on the same basis.
**pit: no verdict changes** (all 7 shadow candidates pass, all screen names fail). On the **adj**
cache #4 fails `pf_5y` + `years_positive` and #29 fails `years_positive` — the MACD scale effect,
not the bug.

## Universe-screen names (additive-ticker, standalone 5y: raw → adj → pit)

| name | raw | adj | pit | verdict |
|---|---|---|---|---|
| TSLA (#36) | −569 / 223 / 0.86 | −607 / 215 / 0.84 | −604 / 223 / 0.85 | fail (unchanged) |
| AVGO (#37) | −455 / 143 / 0.73 | −497 / 89 / 0.63 | −455 / 143 / 0.73 | fail (unchanged) |
| SHOP (#42) | +622 / 113 / 1.30 | +623 / 107 / 1.31 | +627 / 112 / 1.30 | fail (unchanged) |
| NFLX (#38) | +24 / 128 / 1.01 | −69 / 43 / 0.90 | −14 / 129 / 0.99 | fail (unchanged) |
| XLE (#35) | −204 / 43 / 0.57 | −58 / 24 / 0.78 | −193 / 42 / 0.59 | fail (unchanged) |

GOOGL was never screened (SIP cache only; the screen ran on IEX).

## Live side

The daily warm-up of the 5m/1h windows is now split-adjusted: the stream's basis, and exactly point
in time for the day it runs. Before, the sessions after the next AAPL/AMZN/MSFT/NVDA split would
have started with a 1h window reading as a crash.

## Open item (decision needed)

Plain replays of the adjusted caches give **adj**, not **pit**, on any day before a name's last
split (NVDA < 2024-06, AMZN < 2022-06). The pipeline stays consistent (candidates and `iex_v18` share
a basis), but the 2022–24 record is understated by ~500 on v18. Two fixes:

1. **Point-in-time rebasing in `load_cached_bars`.** Keep a `splits.csv` per cache dir, derived from
   raw-vs-split daily bars, and multiply bars by the product of later split ratios. This reproduces
   pit exactly, with the composites above as the test oracle.
2. **Make MACD scale-free.** Divide the histogram by price. This is a strategy change and needs its
   own validation.

Until then, quote pit. The raw caches stay until 2026-10-21 if a revert is wanted.
