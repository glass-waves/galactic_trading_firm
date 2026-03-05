# backtest config tuning log

iterative config tuning in two phases:
1. **20-day set** (iterations 0–28): rapid exploration across 20 fixed dates
2. **100-day set** (iterations 29–53): fine-tuning across 99 dates spanning march 2025 – march 2026

**backtest params**: `--capital 10000 --lookback-days 3 --slippage-bps 2.0 --half-spread 0.005`

---

## phase 1: 20-day tuning

**rubric** (weighted composite 0–10):
- total P&L (3x weight): >$2k=5, >$500=4, >$0=3, >-$2k=2, else=1
- win day rate (2x): >60%=5, >50%=4, >40%=3, >30%=2, else=1
- profit factor (2x): >2.0=5, >1.5=4, >1.0=3, >0.5=2, else=1
- trade frequency (1x): 2-10/day=5, 1-15=4, 0.5-20=3, <30=2, else=1
- tail risk (2x): max loss <5%cap=5, <10%=4, <20%=3, <40%=2, else=1
- win/loss asymmetry (1x): avg_win/avg_loss >2=5, >1.5=4, >1=3, >0.5=2, else=1

| # | change | total P&L | trades | win days | PF | biggest loss | score |
|---|--------|-----------|--------|----------|-----|--------------|-------|
| 0 | baseline (v5) | -$11,700 | 425 | 8/20 | 0.16 | -$5,554 | 2.9 |
| 1 | entry 0.6→0.75 | -$4,428 | 36 | 4/20 | 0.29 | -$6,269 | 2.5 |
| 2 | entry 0.65, exit -0.25, avoid 30min | -$7,414 | 164 | 10/20 | 0.38 | -$8,622 | 3.8 |
| 3 | +1hr hard gate +agreement multiplier | -$4,877 | 11 | 5/20 | 0.23 | -$6,359 | 2.3 |
| 4 | iter2 +hard gate only (no agreement) | -$7,414 | 164 | 10/20 | 0.38 | -$8,622 | 3.8 |
| 5 | +max 1 position, avoid 60min | -$7,650 | 164 | 10/20 | 0.37 | -$8,896 | 3.8 |
| 6 | disable score exits, sizing 5% | -$11,754 | 164 | 10/20 | 0.48 | -$15,535 | 3.8 |
| 7 | disable fixed stop | -$8,096 | 163 | 10/20 | 0.36 | -$9,342 | 3.8 |
| 8 | **ATR trailing 3x→6x, fixed stop 3%** | **-$3,033** | 160 | 9/20 | 0.59 | -$4,291 | 3.8 |
| 9 | ATR trailing 10x (no change from 6x) | -$3,034 | 160 | 9/20 | 0.59 | -$4,291 | 3.8 |
| 10 | **5min trend rebalance (MACD+EMA up)** | **-$1,634** | 72 | 5/20 | 0.66 | -$4,474 | 3.6 |
| 11 | entry 0.65→0.55 | -$4,187 | 279 | 8/20 | 0.49 | -$2,824 | 3.8 |
| 12 | **entry 0.60** | **-$1,189** | 115 | 9/20 | 0.80 | -$4,103 | **4.3** |
| 13 | 1min trend rebalance (too aggressive) | -$2,675 | 87 | 7/20 | 0.60 | -$5,736 | 3.4 |
| 14 | revert 1min, sizing 5% | -$1,618 | 115 | 9/20 | 0.88 | -$9,189 | 4.3 |
| 15 | disable shorts (no effect) | -$1,618 | 115 | 9/20 | 0.88 | -$9,189 | 4.3 |
| 16 | **1min weight 0.2→0.1, 5min 0.5→0.6** | **+$1,762** | 123 | 9/20 | **1.16** | -$6,610 | **6.0** |
| 17 | 1min 0.05, 5min 0.65 (too far) | -$890 | 132 | 9/20 | 0.93 | -$9,202 | 4.3 |
| 18 | max hold 90→180min (slower) | +$851 | 121 | 9/20 | 1.07 | -$7,645 | 5.8 |
| 19 | breakeven 0.8%→1.5% (no effect) | +$1,762 | 123 | 9/20 | 1.16 | -$6,610 | 6.0 |
| 20 | disable all 1min (too far) | +$206 | 143 | 9/20 | 1.01 | -$8,432 | 5.4 |
| 21 | **1hr trend rebalance (ST+EMA+ADX up)** | **+$4,273** | 116 | 8/20 | **1.67** | -$3,704 | **7.2** |
| 22 | entry 0.55 (too low) | +$2,451 | 210 | 9/20 | 1.24 | -$3,292 | 6.7 |
| 23 | exit -0.25→-0.15 (no effect) | +$4,273 | 116 | 8/20 | 1.67 | -$3,704 | 7.2 |
| 24 | **vol-scaled sizing (base 5%, atr 1.0)** | **+$3,796** | 116 | 8/20 | **2.45** | **-$1,198** | **8.1** |
| 25 | baseline_atr 1.0→2.0 (too aggressive) | +$2,656 | 116 | 8/20 | 1.69 | -$2,177 | 7.2 |
| 26 | entry 0.55 +vol sizing | +$2,534 | 210 | 9/20 | 1.47 | -$2,002 | 7.2 |
| 27 | **entry 0.58 +vol sizing** | **+$4,767** | 135 | 9/20 | **2.43** | **-$1,081** | **8.1** |
| 28 | fixed stop 3%→1.5% (worse churn) | +$4,121 | 136 | 9/20 | 2.03 | -$1,432 | 8.0 |

### 20-day best: iteration 27 (v42) — score 8.1/10

---

## phase 2: 100-day tuning

broadened to 99 dates (every mon+wed, march 2025 – march 2026, excluding US holidays).
rubric P&L thresholds scaled 5x: >$10k=5, >$2.5k=4, >$0=3, >-$10k=2, else=1.

starting from iteration 27 (v42) config as baseline.

| # | change | total P&L | PF | biggest loss | score | kept? |
|---|--------|-----------|-----|-------------|-------|-------|
| baseline | v42 on 99 days | +$7,966 | 1.52 | -$1,973 | 6.5 | — |
| 29 | baseline_atr 0.8 | +$7,705 | 1.52 | -$1,916 | 6.5 | no |
| 30 | fixed stop 2.5% | +$7,066 | 1.47 | -$1,973 | 6.1 | no |
| 31 | max hold 120min | +$4,545 | 1.26 | -$2,362 | 5.8 | no |
| 32 | entry 0.62 | +$7,966 | 1.52 | -$1,973 | 6.5 | no (no effect — discrete score jumps) |
| 33 | 1hr 0.35/5min 0.55 | +$7,020 | 1.46 | -$1,898 | 6.5 | no |
| **34** | **5min MACD 0.40/RSI 0.10** | **+$9,322** | **1.93** | -$2,004 | 6.5 | **yes** |
| 35 | MACD 0.45/RSI 0.05 | +$6,558 | 1.75 | -$988 | 7.0 | no |
| 36 | 1hr ADX 0.25/VWAP 0.10 | +$8,627 | 1.95 | -$1,186 | 6.5 | no |
| 37 | StochRSI 0.10/EMA 0.30 | +$6,974 | 1.76 | -$1,051 | 6.7 | no |
| 38 | 1min 0.05/5min 0.65 | +$7,338 | 1.65 | -$1,997 | 6.5 | no |
| 39 | ATR trailing 5x | +$9,358 | 1.94 | -$2,004 | 6.5 | no |
| 40 | ATR trailing 4x | +$8,810 | 1.87 | -$1,941 | 6.5 | no |
| **41** | **5min BB 0.05/EMA 0.30** | **+$8,309** | **2.06** | **-$984** | **7.4** | **yes** |
| 42 | 1hr ST 0.35/BolBW 0.05 | +$6,998 | 1.81 | -$984 | 7.0 | no |
| 43 | base fraction 4% | +$8,217 | 2.06 | -$984 | 7.4 | no (wash) |
| 44 | base fraction 6% | +$8,002 | 1.97 | -$1,005 | 7.0 | no |
| 45 | disable 1min (weight 0) | +$6,437 | 1.61 | -$1,348 | 6.5 | no |
| 46 | agreement ConfidenceMultiplier 0.5 | +$5,359 | 1.98 | -$1,017 | 6.5 | no |
| 47 | vol lookback 30 | +$8,309 | 2.06 | -$984 | 7.4 | no (no effect) |
| 48 | 1hr ADX 0.25/EMA 0.20 | +$6,485 | 1.75 | -$984 | 7.0 | no |
| **49** | **fixed stop 2.5%** | **+$8,732** | **2.16** | **-$977** | **7.4** | **yes** |
| 50 | breakeven 1.0% | +$8,732 | 2.16 | -$977 | 7.4 | no (no effect) |
| 51 | max hold 75min | +$5,845 | 1.60 | -$1,674 | 6.7 | no |
| **52** | **ATR trailing 7x** | **+$8,810** | **2.19** | **-$977** | **7.4** | **yes** |
| 53 | ATR trailing 8x | +$8,851 | 2.19 | -$977 | 7.4 | no (marginal, reverted to 7x) |

---

## best config (iteration 52, v88)

**100-day score: 7.4/10 | P&L: +$8,810 | return: +88.1% | profit factor: 2.19**

### key breakthroughs (cumulative from both phases)

1. **wider trailing stop (iter 8)**: ATR multiplier 3x→6x→7x reduced re-entry churn on crash days. fewer stops = fewer re-entries = less churning damage.

2. **trend-following indicator rebalance (iter 10, 21, 34, 41)**: shifted 5min indicator weights from 60% mean-reversion → 80% trend-following (MACD 0.40, EMA 0.30 up; RSI 0.10, BB 0.05 down). shifted 1hr indicators toward SuperTrend+EMA+ADX. prevented false "buy the dip" entries on crash days.

3. **timescale weight shift (iter 16)**: reduced 1min weight from 20%→10%, increased 5min from 50%→60%. the mean-reversion-biased 1min timescale was poisoning entries on volatile days.

4. **volatility-scaled sizing (iter 24)**: switched from fixed 5% to vol-scaled (base 5%, baseline_atr 1.0). automatically shrinks positions when ATR is high.

5. **tighter fixed stop (iter 49)**: 3.0%→2.5% improved P&L and tail risk once trend-following weights were strong enough to avoid false dip-buying entries.

### final config summary

```
scoring:
  entry_threshold: 0.58
  exit_threshold: -0.15
  timescale_weights: { 1min: 0.10, 5min: 0.60, 1hr: 0.30 }
  aggregation: WeightedSumWithGates
  hard_gate: [OneHour]

5min indicator weights (trend-biased):
  MACD: 0.40, EMA: 0.30, StochRSI: 0.15, RSI: 0.10, Bollinger: 0.05

1hr indicator weights (trend-biased):
  SuperTrend: 0.30, EMA: 0.25, ADX: 0.20, VWAP: 0.15, BolBW: 0.10

actions:
  ATR trailing stop: 7.0x multiplier
  fixed stop: 2.5%
  max hold: 90min (5,400,000ms)
  sizing: volatility_scaled (base 5%, baseline_atr 1.0, lookback 20)
  breakeven: 1.5% trigger (no effect — documented no-op)
```

### 100-day performance profile

```
days:             99 (win: 23, loss: 17, flat: 59)
total trades:     438 (avg 4.4/day on active days)
total P&L:        +$8,810
profit factor:    2.19
biggest win:      +$2,180 (2026-02-09)
biggest loss:     -$977 (2026-02-23)
avg win day:      +$705
avg loss day:     -$411
win/loss ratio:   1.71
max consec loss:  3 days
```

### no-ops discovered

parameters with zero effect in the backtest engine:
- `entry_threshold` in range 0.58–0.62: composite score jumps discretely; no entries land in this range
- `exit_threshold` (-0.15 to -0.25): positions exit via stops/timeouts before score trigger fires
- `short_threshold`: composite never drops below -0.60
- `breakeven_trigger` (0.8%–1.5%): breakeven stop rarely activates given current ATR trailing width
- `avoid_first_minutes`: not enforced in backtest engine
- `max_concurrent_positions`: not enforced in backtest engine or TradingEngine
- `baseline_atr` in range 0.8–1.0: minimal effect on position sizing
- `vol_lookback` 20 vs 30: identical with 3-day backtest lookback
- ATR trailing above 7x: 2.5% fixed stop catches exits before ATR trailing triggers

### remaining weaknesses (addressed in phase 3)

- 59 flat days (60% of days have no qualifying entries)
- april 7, 2025 (major selloff): best configs still produce churn losses
- no re-entry cooldown per ticker: stop-out → immediate re-entry churn on volatile days → **fixed**
- no daily loss circuit breaker: no mechanism to halt after accumulating losses → **fixed**
- no correlation-aware sizing: multiple simultaneous entries in correlated names → **fixed**
- session config not enforced in backtest engine (tuning on incomplete simulation) → **fixed**

---

## phase 3: new features A/B testing

implemented all 13 proposed features and 5 bug fixes from `docs/proposed_additions.md`, then A/B tested each feature independently on the 20-day set before combining winners on the 100-day and 2022 bear market sets.

### new features implemented

**actions:**
- `score_scaled` sizing (min/max fraction scaled by composite score)
- adaptive `max_hold_timeout` (`profit_extension_ms`, `loss_reduction_ms`)

**indicators (4 new):**
- `relative_volume` (RVOL = current_volume / avg_volume)
- `market_breadth` (stock vs index relative performance)
- `momentum_persistence` (ROC of ROC, second derivative)
- `cross_ticker_correlation` (pairwise correlation signal)

**engine-level:**
- `entry_cooldown_ms` (blocks re-entry for N ms after position close)
- `max_daily_loss_pct` (circuit breaker — blocks entries after cumulative loss exceeds threshold)
- `exit_threshold` enforcement (`ExitReason::ScoreExit`)
- `avoid_first_minutes`, `no_new_entries_after`, `max_concurrent_positions` now enforced
- `force_exit_by` removed from `SessionConfig` (handled by `session_close` action)
- `entries_blocked` field on `MarketState` for cross-ticker position limiting

**CLI:**
- `--verbose` flag for per-trade detail output
- `--output-equity` flag for equity curve chaining
- `--compound` mode in shell scripts
- CLI override system (`ConfigOverrides`) for A/B testing without modifying DB config

### 20-day A/B test results (individual features)

baseline: v88 config, $10k capital, 3-day lookback, slippage 2bps + $0.005 spread

| variant | P&L | trades | PF | win% | worst day |
|---------|-----|--------|-----|------|-----------|
| **baseline** | +$6,145 | 105 | 14.13 | 90.0% | -$468 |
| cooldown 30s | **+$6,428** | 79 | 8.59 | 80.0% | -$467 |
| cooldown 60s | +$5,831 | 68 | 6.73 | 80.0% | -$467 |
| cooldown 120s | +$5,413 | 56 | 5.80 | 80.0% | -$467 |
| adaptive hold +30m/-15m | +$6,220 | 105 | 14.29 | 90.0% | -$468 |
| adaptive hold +15m/-15m | +$6,025 | 105 | 12.94 | 90.0% | -$468 |
| adaptive hold +30m/-30m | +$5,880 | 105 | 11.34 | 90.0% | -$468 |
| score-scaled 2-8% | +$4,961 | 105 | 8.55 | 80.0% | -$379 |
| rvol weight=0.10 | +$5,262 | 104 | 4.17 | 63.6% | -$1,058 |
| rvol weight=0.15 | +$5,262 | 104 | 4.17 | 63.6% | -$1,058 |
| rvol weight=0.20 | +$5,262 | 104 | 4.17 | 63.6% | -$1,058 |
| daily loss 5% | +$4,767 | 67 | 6.24 | 77.8% | -$468 |
| daily loss 10% | +$5,279 | 81 | 7.86 | 80.0% | -$468 |
| daily loss 15% | +$5,279 | 81 | 7.86 | 80.0% | -$468 |

**key findings:**
- cooldown 30s: clear winner (+$283 over baseline), cuts overtrading
- adaptive hold +30m/-15m: marginal positive, lets winners run longer
- score-scaled sizing: worse — replaces vol-scaled which is load-bearing
- RVOL: harmful at all weights tested (degraded PF from 14→4)
- daily loss 10%: conservative but safe, 5% too aggressive

### combined feature test — 100-day set

**winning config**: cooldown 30s + adaptive hold (+30m/-15m) + daily loss 10% circuit breaker + vol-scaled sizing (unchanged)

| metric | baseline (v88) | winning combo | delta |
|--------|----------------|---------------|-------|
| P&L | +$9,988 | **+$12,698** | **+$2,711** |
| PF | 2.47 | **3.10** | +0.63 |
| trades | 450 | 212 | -238 |
| biggest win | +$2,180 | **+$3,008** | +$828 |
| biggest loss | -$977 | **-$859** | +$118 |
| win/loss ratio | 1.75 | **2.51** | +0.76 |
| return on cap | +99.9% | **+127.0%** | +27.1% |
| **score** | **7.4/10** | **8.1/10** | **+0.7** |

notable: 2025-04-09 (volatile day) flipped from +$73 (177 trades, churn) to +$3,008 (55 trades, cooldown prevented re-entry churn).

### combined feature test — 2022 bear market (jan–may, 38 days)

| metric | baseline | winning combo | delta |
|--------|----------|---------------|-------|
| P&L | +$4,060 | **+$4,718** | +$658 |
| PF | 1.56 | **1.75** | +0.19 |
| trades | 376 | 192 | -184 |
| biggest loss | -$1,648 | **-$1,139** | +$509 |
| win/loss ratio | 1.14 | **1.75** | +0.61 |
| return on cap | +40.6% | **+47.2%** | +6.6% |
| **score** | **7.4/10** | **7.2/10** | -0.2 |

degrades gracefully in sustained downturn. higher P&L, better tail risk, fewer trades.

### failed combination: all features + score-scaled sizing

when score-scaled sizing replaced vol-scaled sizing with all features enabled:
- 100-day P&L dropped from +$9,988 to +$5,699
- 2025-04-09 catastrophic: -$5,682 (single day, 56.8% of capital)
- root cause: score-scaled doesn't adjust for volatility; vol-scaled is load-bearing

### iteration 54 (phase 3 winner) — promoted config

changes from v88:
- `session.entry_cooldown_ms`: 0 → 30000 (30 seconds)
- `session.max_daily_loss_pct`: null → 0.10 (10%)
- `max_hold_timeout.profit_extension_ms`: 0 → 1800000 (+30 minutes for winners)
- `max_hold_timeout.loss_reduction_ms`: 0 → 900000 (-15 minutes for losers)

```
100-day performance:
days:             99 (win: 21, loss: 17, flat: 61)
total trades:     212 (avg 2.1/day)
total P&L:        +$12,698
profit factor:    3.10
biggest win:      +$3,008 (2025-04-09)
biggest loss:     -$859 (2026-02-23)
avg win day:      +$892
avg loss day:     -$355
win/loss ratio:   2.51
max consec loss:  4 days
return on cap:    +127%
score:            8.1/10
```
