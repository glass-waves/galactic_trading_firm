# research history (survey 2026-10-31, read before proposing any cell)

Record notation: P&L / trades / PF; years 22/23/24/25/26. The standard record is `iex_v18` (5-year IEX replay,
36 % sizing, 3 bps + $0.005 per leg, `--cross-index SPY`): +1829 / 434 / PF 1.40; 22:+1089 23:−3 24:−17 25:+332 26:+429.
Older studies quote earlier restatements (+2154/407/1.48 before the VWAP fix, +1728/436/1.37 before the split fix).

## A. the promoted strategy (v18, config row 12)

Short-only morning book on AMZN AAPL NVDA MSFT; entries to 11:30 ET, flat by 11:55, ≤ 3 positions.
Two windows: "5m thrust short" (composite ≤ −0.35, 5m lags the other timescales by 0.1, 5m ≤ −0.5, 1h ≤ 0) and
"strong core short" (composite ≤ −0.35, 5m ≤ −0.4, 1h ≤ −0.4). Shared filters: SPY session return within ±0.2 % of
its open (`cross_1m` in [−0.4, 0.4]) and `vpin_1m.raw_vpin ≥ 0.217` (≈ 82nd–85th percentile on IEX).
Exits: losing-side max hold 40 min, winning-side 90 min; 0.5 % breakeven; 2.5 % hard stop; ATR×7 trail; score exit −0.30;
30 s entry cooldown. Live sizing 0.30, research 0.36. 2022 is 59 % of P&L; ex-2022 PF 1.19. Same book on SIP bars:
+2835 / 510 / 1.57. Cost sensitivity (v17): 5 bps → PF 1.44; 10 bps + $0.01 → PF 1.06.
How it got here: SPY-flat band + VPIN floor lifted PF 1.12 → 1.64 (plan §11.2–11.3); time stop + tighter core 1h +
breakeven took v15 (PF 1.01) to v16 (PF 1.12).

## B. everything tried

### B1 entry windows / signals (short)
| idea | where | result | verdict |
|---|---|---|---|
| 28 candle/OFI/gap/ORB/peer triggers | docs/analysis/2026-09-12_entry_screen.md | all negative standalone | patterns confirm, never lead |
| 9 candle patterns, standalone 5m short | plan §11.4 | engulfing −2820/1222/0.81 … best three black crows +62/186/1.03 | none is a trigger |
| SPY-flat band + VPIN (→ v17/v18) | plan §11.3 | +2785/469/1.64 (SIP) | promoted |
| band × VPIN plateau grid | plan §11.6, §14.1 | every IEX cell PF 1.21–1.58; wider band = more trades, lower PF | v19 cell (±0.3 %/0.18) +1708/578/1.26 not promoted |
| 12 trigger cells | research/trigger/2026-10-02_entry_trigger_study.md | none passes additive-config | "no trigger adds volume that holds up"; only thrust 1h ≤ −0.15 survives (#29) |
| pullback entry | same | +175/221/1.06, 23:−405 | dead: thrust bars already sit near the low |
| 1m-led confirmation | same | lead −160/73/0.83; lag +240/180/1.10 | dead: "5m lags 1m" is part of the edge |
| VPIN-slope window | same | d_s5 +1736/508/1.30 vs no-slope +1900/537/1.31 | slope inert; fails LOYO |
| composite ceiling −0.30 | trigger, qualvol | +1719/439/1.36 | never binds |
| core 1h loosened −0.3/−0.2 | research/qualvol/2026-10-07_quality_for_volume.md | PF 1.10–1.23 on 549–943 trades; LOYO years −315/−369 | worst lever tried |
| thrust 1h loosened −0.10 | same | PF 1.26–1.33, 3/5 years | unstable |

### B2 quality / volume levers
| idea | where | result | per year | verdict |
|---|---|---|---|---|
| thrust-1h15 (#29) | trigger study | +1714/278/1.63 | 22:+644 23:+55 24:−56 25:+396 26:+676 | quality pass; same dollars on 36 % fewer trades |
| VPIN 0.26 (#4) | status.md | +1646/324/1.48, ex-22 1.28 | 22:+820 23:+42 24:+105 25:+210 26:+469 | quality end of the trade-off |
| percentile VPIN floor (W1950 P0.8) | research/volume/pctile.md | +2216/440/1.44 | 22:+1711 23:−141 24:+33 25:+341 26:+272 | parity with the raw floor |
| tiered sizing below floor | research/volume/tiered.md | best +2032/593/1.36 | 23, 24 negative every cell | sub-floor trades are 70 % PF 0.6 population |
| quality-for-volume grid (19 cells) | research/qualvol/ | every cell > 436 trades has PF 1.32–1.39 | v150_b06 +1925/486/1.38 | nothing > 1.4 with > 436 trades |
| spy-sqrt-band (#44) | research/timefilter/2026-10-07_time_consistent_filter.md | +2195/460/1.46 | 22:+1144 23:+77 24:+116 25:+371 26:+486 | pass, 5/5 years, beats v18 in every LOYO fold |
| spy-sqrt-band-1h15 (#45) | status.md | +2044/309/1.70, ex-22 1.60 | 22:+708 23:+125 24:+81 25:+428 26:+703 | best ex-2022 PF of any candidate |
| spy-sqrt-band-v020 (#49) | RQ-6 | +2445/492/1.48 | 22:+1275 23:+60 24:+475 25:+166 26:+468 | vs #44 noise (+149, CI [−291,+603]); NVDA 73 % of P&L |
| size-vpin26-x1.25 (#28) | research/sizing/README.md | +2155/436/1.40 | 22:+1194 23:+46 24:+10 25:+410 26:+496 | leverage on existing trades |
| sqrt band k 0.04/0.06, 1h dial, 11:00 clock | timefilter | sq05_1h15 +1881/333/1.60, 5/5; sq05_1h10 +1860/406/1.46 | 25:+437 26:+589 / 25:+399 26:+587 | under the trade bar / fails volume gate |

### B3 time of day
| idea | result | verdict |
|---|---|---|
| full-day session (entries to 15:30) | x1530 +1331/488/1.24; sq05_x1530 +1517/637/1.23 | every half-hour after 11:30 loses (−137…−328 each) |
| entries 09:45–11:00 only | +599/282/1.21 | 228 of 436 entries are before 09:45 — the open is the edge |
| skip first 15 min | 2026 +997 vs +2065 | rejected |
| rolling SPY 15 m band | r15_15 +893/565/1.14; c_r10 +1434/472/1.28 | loses the open's filter |
| 10:00–11:00 extension with sqrt band | 68 trades/+217 → 111/+650 | the one extension that worked (→ #44) |

### B4 exits / holds
| idea | result | verdict |
|---|---|---|
| losing limit 40 min / winning 90 | h_l40w90 +875 vs base +112 | adopted (v16) |
| hard stop 1.5 %, ATR 3×, profit ext 60 min, score exit −0.50 | wash or worse | rejected |
| max-hold extensions | "every extension loses" | winners exit near 77 % of favorable excursion |
| score exit off / hold 3 h (stress) | identical / −94 | no effect |

### B5 long side
| idea | result | per year | verdict |
|---|---|---|---|
| long book (v13) | −2790 | 22:−2049 23:+512 24:−690 25:−2659 26:+2095 | a bet on morning drift; disabled since v15 |
| ORB15/30, gap-and-go, VWAP reclaim, squeeze, pullback, reversal (long) | 2026 +12…+816; 2025 −2204…−2709 | | breakouts get faded in down-drift years |
| v17-filtered longs | −1710 | 22:+545 23:−645 24:−432 25:−1524 26:+346 | strong core long is a consistent loser |
| longs in up-regime (sma50/sma20/ret20/lowvol) | best sma20 +444/914/1.05 | 25:−753 | up-regime gates keep the losing half |
| bear-bounce longs, SPY < SMA50 (#1) | +2179/828/1.23 | 22:+1422 23:+371 24:−436 25:+61 26:+762 | fails pf_5y, min_year |
| RS long (RS vs SPY ≥ 0.3) (#5) | +1997/1586/1.11 | 22:+1490 23:+238 24:−197 25:−213 26:+679 | tighter RS monotonically worse |
| rs_solo_x0.5 long | +3443/1941/1.17 | 25:+69 26:+1026 | |
| rl_none (ungated longs) | | 25:−204 26:+775 (PF 1.29) | 2025 is a hole for every long variant |
| mirrored RS short | −1755/1183 | all years negative | starves v18's shorts |

### B6 universe
| idea | result | verdict |
|---|---|---|
| 6 more mega-caps (META GOOGL TSLA AMD AVGO NFLX) | pooled −201/1140/0.99 | "not a generic mega-cap property" |
| 36-ticker screen (#2, #3, #8–#43) | 0 of 36 pass; SHOP +622/113/1.30 (22:+708 23:−445 24:+4 25:+43 26:+312), CAT +267/92/1.26 (24:−129 25:+203), CVX +213/56/1.43 (25:+68 26:+139), LLY +218/147/1.14, PLTR +204/160/1.06, XOM +131/51/1.28, TSLA −569/223/0.85, COIN −1307/217/0.74 | binding check is combined_years_not_worse (2023/2024 have no room). on thrust-1h15: AMD +452/PF 1.28/4 pos years, META +401/1.33/3 |
| QQQ as fifth name | PF 0.54 | dropped |

### B7 regime
walk-forward quarterly refit +602 vs static +589 (shorter memory worse); realised-vol/SMA regimes for longs rejected;
FOMC-day compression real (SPY 10:30–14:00 36.9 % tighter, z −7) but the windows it would gate are dead; OpEx dead;
weekday / FOMC / earnings exclusions on v16 negative ("barely trades those days").

### B8 indicators
scale-free MACD k=1 % (RQ-8) +421/231/1.15 (22:+666 23:−363 24:−100 25:+136 26:+83): invariance holds, strategy off.
RQ-12 MACD `normalization_pct` k 0.003–0.007 (research/macd/pct_*.patch.json) and RQ-13 directional 1h
(research/dir1h/dir1h.patch.json: standard supertrend + signed ADX) are queued, never run.

### B9 sizing
top tier ×1.25 at VPIN ≥ 0.26 (#28) leverage only; stress sizing ×1.5 same trades more size; research 36 % vs live 30 %.

### B10 events
| idea | result | verdict |
|---|---|---|
| earnings-day 5-min ORB | +560/74/1.38; 22:+676 23:−417 24:−316 25:+453 26:+164 | parked; engine lacks far-side stop |
| earnings-gap fade | 4-name PF 1.40; 30-name 598 events PF 1.10 | dead (NVDA 81 % of P&L, p 0.34) |
| pre-FOMC drift | SPY 09:30→14:00 −2.5 bps PF 0.78 | decayed |
| turn-of-month day +2 (pretest c11/c15) | QQQ 25:+211 (PF 7.9) 26:+46, 23:−11, LOYO 3/5; SPY 5/5 years PF 1.77, LOYO +341 5/5, p 0.026, 25:+134 26:+66; 4 mega-caps 232 trades +37.7 bps 4/5 (25:+1115 26:+374, 23:−203); 34-name pool 1972 trades +30.9 bps (25:+8065 26:+2479, 23:−538) | "fragile" after controls; never run in the engine (RQ-11 proposed; research/tom/) |
| PEAD hold-10 | universe PF 1.55, 63 trades/yr | needs multi-day engine |

### B11 end of day
EOD loser reversal −194…+42 (only 2022 pays); last-hour momentum / midday fade / Gao below cost; LETF close momentum dead.

### B12 stress / volatility
stress mode 12 triggers none pass; s15_core (#6, SPY ≤ −1.5 %, core window) +2003/459/1.41, added +274/23/3.32 (18 of 23
trades in 2022); name-move triggers n15/n25 dead (102 false-alarm days −2826); VWAP-anchored short fails (VWAP stop is the
loss centre); trend-day-ride (#7) +2142/803/1.23 (22:+2003 23:−260 24:−307 25:+461 26:+246); crash-day early trigger
SPY ≤ −1 % by 10:30 fires ~4 days/yr; NVDA leads 56–72 % of tail days at ~2× SPY beta.

### B13 index momentum
noise-area SPY −894/0.84, QQQ −186/0.97, Gao −2218/0.49; qqq-noise-pm-vol (#48) +1095/275/1.50 (22:+600 23:+168 24:−22
25:+227 26:+122), corr −0.03 vs v18, fails at 2× cost, needs 22-day warm-up; 5-min ORB QQQ 81 cells best PF 1.07; HKS dead.

## C. strongest in 2025–2026 (regardless of 5-year verdict)
spy-sqrt-band-1h15 25:+428 26:+703 (PF 1.70/309) · thrust-1h15 +396/+676 · sq05_1h15 +437/+589 · sq05_1h10 +399/+587 ·
size-vpin26-x1.25 +410/+496 · spy-sqrt-band +371/+486 · stress-s15-core +376/+429 · vpin-0.26 +210/+469 ·
v18 on SIP +410 (1.51)/+726 (1.93) · trend-day-ride +461/+246 · bear-bounce-sma50 +61/+762 · qqq-noise-pm-vol +227/+122 ·
ToM day+2 SPY +134/+66, QQQ +211/+46, mega-caps +1115/+374 · earnings ORB +453/+164 · VWAP-stretch fade 12:30 hold 60 (c14)
+178 (1.13)/+494 (1.81), n 481, 3/5 years · rs-long −213/+679 · rs_solo_x0.5 +69/+1026 · rl_none −204/+775 ·
SHOP +43/+312, CAT +203/+37, CVX +68/+139 · RQ-8 k01 +136/+83 · PEAD hold-10 PF 1.93/1.46 (daily bars).

## D. killed with a structural reason (needs a genuinely new angle to retry)
1. longs of every type (a bet on morning drift; regime gates keep the losing half; 2025 loses in every variant).
2. exit tuning (winners already exit near 77 % of MFE; every extension loses; re-entry churn kills; 30 s cooldown exists for that).
3. afternoon / full-day windows (every half-hour after 11:30 loses; EOD reversal under cost).
4. pullback / confirmation / delayed entry (earliest in the thrust is the edge).
5. VPIN levers (lower floor admits a PF-0.6 population; slope inert; percentile floor is portability only).
6. loosening the 1h confirmations (worst lever tried); composite floor never binds.
7. stress mode / crash capture (SPY touch reverses half the days; VWAP retest never fires when it matters).
8. universe expansion (0 of 36; edge specific to AAPL/AMZN/MSFT/NVDA).
9. walk-forward / recent-only refit.
10. dead pretests: ORB, HKS, gap fill, overnight lean, hedging-demand momentum, OpEx, LETF close, pre-FOMC, overnight holds.
11. multi-day reversal/breakout/bear-bounce ≈ beta once capped. 12. full-day index momentum fails at our costs. 13. candlesticks as triggers.

## E. open, never run
RQ-12 MACD pct sweep; RQ-13 directional 1h; RQ-14 restatement bundle (max-hold clock at fill bar, warmup 12, VPIN session
sigma); RQ-11 ToM day+2 long SPY/QQQ/IWM (fails the standalone gate's active-days clause by construction); RQ-10 VWAP-fade
engine study (needs a normalised-stretch metadata key on vwap_distance); RQ-9 IEX-native VPIN neighbourhood
(`bucket_divisor` {0.2,0.4,0.8} × `lookback_buckets` {20,40}); RQ-7 phase 2; RQ-8 phase C (SHOP AMD META CVX CAT under
scale-free MACD). From studies: earnings ORB with a far-side stop (engine lacks it); "a genuinely new trigger added alongside,
not loosened"; NVDA-relative sizing for stress; long book with ticker-specific inclusion (NVDA/AAPL only); long ticker filter
(AAPL LIN MA ABBV V by intraday drift); VPIN upgrade-on-cross; afternoon variant of the 1h dial.

## F. pitfalls
- feed: live is IEX (~3 % of volume); `data/bars_iex` matches live; SIP is `data/bars`; raw point-in-time caches `*_raw`.
  VPIN thresholds are feed-specific. IEX misses 1.6 % of SPY / 6 % of QQQ decision minutes. IWM cache ends 2026-10-13.
- restatements: pre-09-27 numbers used a multi-day VWAP; before b048cdc shorts were credited the spread; a plain replay of
  the split-adjusted cache prints +1318/360/1.34 vs point-in-time +1829/434/1.40 because MACD is scored in dollars.
- gate sweeps use `--sizing-fraction 0.36 --max-position-pct 0.36 --cross-index SPY`; live is 0.30. a sizing candidate
  needs `--max-position-pct 0.45` to lift the cap. use `--cross-lag 1` as the live-timing check (v18 at lag 1 +1544/442/1.32).
- lookback 8 = live warm-up; `noise_area` needs 22; 1h ADX needs 12 after holiday weekends.
- replay fills at the next bar's open; live at the signal bar's close. max-hold clock differs by one bar. replay runs tickers
  independently (3-position cap not simulated; 3 days had 4 concurrent).
- statistics: ~2100 configurations on the same data; best-of-N null reaches PF 1.48 at the 95th percentile; shadows cannot
  tell PF 1.4 from 1.0. 2023 and 2024 are flat in every short-side cell ever tried.
- patch mechanics: patches replace instances by `instance_id`; to change a window's clock, disable the promoted windows and
  re-add `_am` copies with their own clocks (the scaffold must reproduce v18 trade for trade first); `session` must be opened
  (`no_new_entries_after`, `force_exit_by`) for any afternoon window.
