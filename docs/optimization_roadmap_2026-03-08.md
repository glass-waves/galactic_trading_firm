# optimization roadmap — 2026-03-08

current state: v91 config, ~70-75% optimized. 100-day P&L +$16,254 (PF 4.27), 2022 bear +$5,624 (PF 2.30).

this doc outlines the remaining edge to capture and the backtesting gaps to close before real capital.

---

## 1. unexplored indicators

14 of 28+ implemented indicators are in the config. the rest are sitting in the registry, tested in code but never backtested in a live config. each one is a potential source of orthogonal alpha.

### high priority (microstructure + regime)

| indicator | type | rationale | suggested test |
|---|---|---|---|
| **vpin** | `vpin` | volume-synchronized probability of informed trading. spiked before 2010 flash crash. use as **hard gate** (block entries when VPIN > 0.85) rather than scored signal | add on 5-min, test as gate: `--add-vpin-gate 0.85` |
| **momentum_persistence** | `momentum_persistence` | second derivative of price — distinguishes accelerating trends from decelerating ones. could improve entry timing | add on 5-min at weight 0.05-0.10 |
| **position_direction** | `position_direction` | meta-indicator exposing current position state to scoring. lets the scoring pipeline "know" it's already long and weight exit signals higher | add on 5-min at weight 0.05 |
| **unrealized_pnl** | `unrealized_pnl` | meta-indicator for in-position P&L. could sharpen exit timing — weight negative when losing, positive when winning | add on 5-min at weight 0.03-0.05 |
| **hold_duration** | `hold_duration` | meta-indicator for time in position. longer holds → lower score, encouraging timely exits | add on 5-min at weight 0.03 |
| **session_remaining** | `session_remaining` | meta-indicator for time until close. suppresses entries late in session more smoothly than a hard cutoff | add on 5-min at weight 0.05 |

### medium priority (additional technical signals)

| indicator | type | rationale |
|---|---|---|
| **williams_r** | `williams_r` | similar to stochastic but inverted. may capture different reversals than stoch_fast on 1-min |
| **donchian** | `donchian` | channel breakout detection. captures range expansion differently than bollinger |
| **ttm_squeeze** | `ttm_squeeze` | bollinger inside keltner = low volatility squeeze. signals imminent breakout |
| **awesome_oscillator** | `awesome_oscillator` | momentum via dual SMA of HL2. different construction than MACD |
| **cci** | `cci` | commodity channel index. strong mean-reversion signal on 1-min |
| **mfi** | `mfi` | money flow index (volume-weighted RSI). volume confirmation for entries |
| **dema** | `dema` | double EMA — faster trend following than single EMA |
| **market_breadth** | `market_breadth` | individual stock vs index performance. filters entries when a stock diverges from the market |

### testing approach

extend `ConfigOverrides` with a generic `--add-indicator TYPE:TIMESCALE:WEIGHT` flag, then sweep each on 20-day. group winners by signal type (momentum, volume, regime) to avoid correlation.

---

## 2. backtesting gaps

### 2.1 walk-forward validation

**problem**: every phase of tuning sees the same 20/100/38 dates. we're optimizing to known data.

**fix**: rolling walk-forward — train on 20 days, test on the next 5, slide forward. the `agents-ts/src/walk-forward.ts` utility already exists but hasn't been used for config validation. build a rust-native version:

```
for window in sliding_windows(all_dates, train=20, test=5):
    optimize(window.train)  # or just use promoted config
    score(window.test)      # truly out-of-sample
    record(score)
aggregate(all_scores)       # expected live performance
```

### 2.2 slippage sensitivity analysis

**problem**: we use fixed 2 bps slippage. real slippage varies with volatility, time of day, and order size.

**fix**: run the 100-day set at 1, 2, 4, 6, 8, and 10 bps. plot the P&L curve. if P&L goes negative at 6 bps, our edge is fragile. ideally we want profitability up to ~8 bps.

```bash
for bps in 1 2 4 6 8 10; do
    ./scripts/backtest_100days.sh --slippage-bps $bps
done
```

### 2.3 monte carlo trade shuffling

**problem**: our P&L depends on trade ordering. a few big wins on specific days drive most returns.

**fix**: randomly shuffle trade outcomes 1,000x and compute P&L distribution. if the 5th percentile is negative, our edge isn't statistically significant — it's just lucky sequencing. implement as a post-processing step on backtest trade output.

### 2.4 regime-tagged performance

**problem**: we report aggregate metrics. we don't know if the system makes money in chop, trends, or both.

**fix**: tag each trading day with a regime (bull trend, bear trend, range-bound, high-vol, low-vol) using simple heuristics (daily ATR percentile, daily return sign + magnitude). break down P&L by regime. if we only make money in one regime, we're fragile.

### 2.5 live vs backtest reconciliation

**problem**: historical bars differ from live streaming bars (finalized vs intra-bar OHLCV).

**fix**: during paper trading, log the indicator scores and entry/exit decisions. at end of day, run the backtest for the same date and compare. divergences reveal data feed issues before they cost money.

---

## 3. structural improvements

### 3.1 per-ticker parameter exploration

currently one config for all 5 tickers. SPY (broad ETF, tight spreads, massive liquidity) likely has different optimal parameters than NVDA (single stock, wider spreads, momentum-driven). test:
- SPY-only backtest vs NVDA-only backtest with same config
- if sharpe differs by >0.5, explore per-ticker indicator weights or entry thresholds

### 3.2 short-side activation

`short_threshold=-1` means we never short. in the 2022 bear market, shorts would have been profitable. test:
- `short_threshold=-0.58` (symmetric to entry)
- `short_threshold=-0.65` (more conservative)
- only on SPY/QQQ (not single stocks, where short squeezes are dangerous)

### 3.3 dynamic regime switching

dynamic fusion had zero effect because the sigmoid inputs (ADX, BB-BW) don't vary enough. alternatives:
- use **realized volatility** (rolling 20-day stdev of returns) instead of BB-BW
- use **trend persistence** (autocorrelation of 5-min returns) instead of ADX
- binary regime switch: if VIX > 25 (or ATR percentile > 80), shift to defensive config (higher entry threshold, tighter stops)

### 3.4 VPIN as a hard gate

rather than scoring VPIN, use it as a circuit breaker: when VPIN > 0.85, block all entries regardless of score. this protects against informed-flow events (earnings leaks, flash crashes) where our technical indicators are blind.

---

## 4. execution edge

### 4.1 order type optimization

backtest assumes market orders. in live, limit orders at the bid/ask midpoint could save 1-2 bps per trade. with 235 trades over 100 days, that's ~$50-100 recovered per 100-day period (small but free).

### 4.2 entry timing within the bar

we enter at next-bar open. but if the signal fires at bar close, placing a limit order during the next bar's formation (rather than a market order at open) could improve fill prices in trending markets.

### 4.3 adaptive slippage model

replace fixed slippage with a function of: current spread, recent volume, time of day, and position size. calibrate from paper trading fills. feed back into backtest for more accurate simulation.

---

## 5. suggested execution order

| priority | item | effort | expected impact |
|---|---|---|---|
| 1 | slippage sensitivity analysis | 30 min | validates edge robustness |
| 2 | VPIN hard gate test | 1 hr | potential tail risk reduction |
| 3 | position context meta-indicators | 2 hr | smarter exits |
| 4 | walk-forward validation | 4 hr | true out-of-sample estimate |
| 5 | per-ticker analysis | 2 hr | identifies ticker-specific edge |
| 6 | regime-tagged breakdown | 3 hr | identifies fragile regimes |
| 7 | short-side on SPY/QQQ | 2 hr | new profit source in bear markets |
| 8 | monte carlo significance | 3 hr | statistical confidence in results |
| 9 | momentum_persistence + ttm_squeeze | 2 hr | new indicator alpha |
| 10 | live reconciliation framework | 4 hr | pre-deployment safety |
