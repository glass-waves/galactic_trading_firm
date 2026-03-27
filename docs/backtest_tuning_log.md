# backtest config tuning log

## phase 1: baseline validation (config v1)

**date**: 2026-03-21
**method**: full-year backtest on every trading day, 2022–2025 (4 years, ~1,000 trading days)
**costs**: 3.0 bps slippage + $0.005 half-spread (pessimistic realistic)
**capital**: $10,000
**config**: promoted seed config incorporating timing/indicator learnings from pre-bug-fix experimentation (see `docs/pre-bug-fix.md`)

### 4-year results

| year | regime | P&L | PF | trades | win rate | W/L ratio | avg hold |
|------|--------|-----|-----|--------|----------|-----------|----------|
| 2022 | bear market (SPY -19%) | +$536 | 3.20 | 545 | 57.4% | 2.35 | 58 min |
| 2023 | recovery/bull (AI hype) | +$339 | 3.29 | 424 | 55.8% | 2.55 | 59 min |
| 2024 | choppy/election year | +$294 | 2.97 | 449 | 55.4% | 2.39 | 55 min |
| 2025 | full year | +$277 | 2.91 | 429 | 58.2% | 2.07 | 61 min |
| **total** | | **+$1,447** | **~3.09** | **1,847** | **56.7%** | **2.34** | **58 min** |

### per-ticker breakdown (4-year cumulative)

| ticker | P&L | trades | share of P&L |
|--------|-----|--------|--------------|
| NVDA | +$735 | 523 | 51% |
| AAPL | +$328 | 458 | 23% |
| MSFT | +$199 | 340 | 14% |
| QQQ | +$116 | 314 | 8% |
| SPY | +$68 | 212 | 5% |

### exit reason distribution (4-year)

| exit reason | count | share |
|-------------|-------|-------|
| ScoreExit | ~980 | 53% |
| SessionClose | ~630 | 34% |
| MaxHoldTimeout | ~225 | 12% |
| HardStop/TrailingStop | ~12 | <1% |

### key findings

1. **strategy is profitable across all market regimes** — positive P&L in bear (2022), recovery (2023), choppy (2024), and recent (2025) markets.
2. **PF consistently ~3.0** — win rate 56% with winners 2.3x larger than losers.
3. **NVDA is the primary alpha source** (51% of P&L, 28% of trades) — generates the most trades and highest per-trade P&L.
4. **SPY is the weakest contributor** (5% of P&L, 11% of trades) — profitable but minimal edge.
5. **score-based exit is the dominant exit path** (53%) — the composite score threshold at -0.05 catches degrading positions before stops fire.
6. **stops are effectively no-ops** (<1% of exits) — in the morning-only regime, positions exit via score/timeout/session-close before ATR trailing or fixed stops trigger.
7. **per-trade expectancy**: ~$0.78 net of costs. transaction costs consume ~26% of gross P&L.

### config v1 parameters

```
scoring:
  entry_threshold: 0.40
  exit_threshold: -0.05
  timescale_weights: { 1min: 0.10, 5min: 0.60, 1hr: 0.30 }
  aggregation: WeightedSumWithGates
  hard_gate: [OneHour]

5min indicators (trend-biased):
  MACD(12/26/9): 0.40, EMA(20): 0.30, StochRSI(14): 0.15,
  RSI(14): 0.10, Bollinger(20,2.0): 0.05,
  OFI: 0.05, momentum_persistence: 0.05

1hr indicators:
  SuperTrend(10,3.0): 0.30, EMA(20): 0.25, ADX(14): 0.20,
  VWAP_distance: 0.15, Bollinger_bandwidth(20,2.0): 0.10

1min indicators:
  RSI(7), Stochastic(14), ROC(12), MACD(6/13/5)

actions:
  entry: score_threshold (composite >= 0.40)
  exit: ATR trailing (7x), fixed stop (2.5%), max hold (90 min, adaptive +30/-15),
        session close, score exit (<= -0.05)
  monitor: breakeven stop (1.5% trigger — documented no-op)
  sizing: volatility_scaled (base 5%, baseline_atr 1.0, max 3%, lookback 20)

session:
  avoid_first_minutes: 30
  no_new_entries_after: 11:00
  entry_cooldown_ms: 30000
  max_daily_loss_pct: 0.10

ticker_overrides:
  AAPL: entry_threshold 0.35
```

### data files

- `data/backtest_2022_trades.csv` — per-trade CSV (545 trades)
- `data/backtest_2023_trades.csv` — per-trade CSV (424 trades)
- `data/backtest_2024_trades.csv` — per-trade CSV (449 trades)
- `data/backtest_2025_trades.csv` — per-trade CSV (429 trades)

---

*note: prior to config v1, 6 phases of tuning (96 iterations) were conducted with a position sizing bug that inflated P&L by 100-400x. those results are archived in `docs/pre-bug-fix.md`. the timing and directional insights from that work informed config v1's parameter choices, but all P&L validation was redone with correct sizing.*

---

## phase 2: candle pattern indicator (config v1 → v2)

**date**: 2026-03-21
**method**: A/B testing on 20-day IS set — feature isolation (reversion × confluence factorial), then weight sweep
**research basis**: Bulkowski (4.7M candle study), Quantified Strategies (SPY backtests), Lin et al. (2021, PRML)

### what was added

engulfing candlestick pattern indicator (`candle_pattern` type, instance `candle_5min`) on 5-minute timescale.

key features:
- **mean reversion mode** (default on): bearish engulfing scores positive (mean reversion entry), bullish engulfing scores negative. research: bearish engulfing traded long on SPY = 71-76% win rate, 2.5 PF.
- **confluence scoring**: final score = base_score × volume_mult × location_mult × trend_mult
  - volume: 2x+ avg → 1.3x boost, <0.5x → 0.5x discount
  - location: reversal below VWAP → 1.2x, above → 0.8x
  - trend: with-trend (EMA5 vs EMA20) → 1.1x, counter-trend → 0.8x

### A/B test results (20-day IS)

#### phase 2a: feature isolation at weight 0.25

| variant | P&L | PF | trades | win% | worst |
|---------|-----|-----|--------|------|-------|
| **baseline** | **+$47.88** | **6.37** | **53** | **63.1%** | **-$3.49** |
| A: raw (no confluence, no reversion) | +$37.43 | 4.24 | 47 | 58.8% | -$3.51 |
| B: +confluence only | +$38.14 | 3.98 | 47 | 58.8% | -$3.51 |
| C: +reversion only | +$40.82 | 6.23 | 43 | 58.8% | -$2.39 |
| D: full model (confluence + reversion) | +$45.56 | 6.84 | 44 | 58.8% | -$2.39 |

**findings:**
- reversion mode is the key differentiator — without it, the indicator degrades PF from 6.37 to 3.98-4.24
- confluence adds +$4.74 P&L when paired with reversion (C→D)
- full model preserves PF (6.84 vs 6.37) while improving tail risk (-$2.39 vs -$3.49)

#### phase 2b: weight sweep — full model

| weight | P&L | PF | trades | win% | worst |
|--------|-----|-----|--------|------|-------|
| 0.05 | +$45.74 | **7.06** | 49 | 61.1% | **-$2.10** |
| 0.10 | +$40.56 | 5.45 | 49 | 55.5% | -$2.10 |
| 0.15 | +$41.37 | 6.40 | 47 | 56.2% | -$2.59 |
| 0.25 | +$45.56 | 6.84 | 44 | 58.8% | -$2.39 |
| 0.40 | +$41.35 | 6.60 | 41 | 47.0% | -$2.44 |

**winner: full model at weight 0.05**
- PF 7.06 (+11% vs baseline 6.37)
- worst loss -$2.10 (-40% vs baseline -$3.49)
- P&L -$2.14 vs baseline (-4%) — acceptable trade-off for improved risk profile

### config v2 changes (from v1)

added 1 indicator:
```
candle_pattern on 5min, weight 0.05:
  engulfing_min_body_ratio: 0.5
  engulfing_base_score: 0.70
  mean_reversion_mode: true
  use_confluence: true
  volume_lookback: 20
  high_volume_threshold: 2.0
  low_volume_threshold: 0.5
  ema_fast: 5
  ema_slow: 20
```
