# Tool Belt × ta-rs Implementation Map

**Adaptive Multi-Timescale Trading System | February 2026**

Maps each indicator from the Tool Belt Catalog to its implementation path using the `ta` crate (ta-rs v0.5). Three categories:

- **Native** — direct `ta::indicators` struct exists
- **Composable** — can be built by combining ta-rs primitives with simple Rust logic
- **Custom** — requires implementation from scratch (ta-rs doesn't help meaningfully)

ta-rs primitives available for composition: `ExponentialMovingAverage`, `SimpleMovingAverage`, `StandardDeviation`, `TrueRange`, `AverageTrueRange`, `Maximum`, `Minimum`, `EfficiencyRatio`. All implement `Next<f64>` and/or `Next<&DataItem>`.

---

## Native ta-rs Indicators

These have a direct struct in `ta::indicators`. Use as-is with our `Indicator` trait wrapping the `Next<T>` call.

| # | Catalog Indicator | ta-rs Struct | Notes |
|---|-------------------|-------------|-------|
| 1 | **RSI** | `RelativeStrengthIndex::new(period)` | Returns 0–100. We normalize to -1..+1 in our wrapper. |
| 2 | **SMA** | `SimpleMovingAverage::new(period)` | `Next<f64>`. Use for price distance, crossover signals. |
| 3 | **EMA** | `ExponentialMovingAverage::new(period)` | `Next<f64>`. Core building block for many composites. |
| 4 | **MACD** | `MovingAverageConvergenceDivergence::new(fast, slow, signal)` | Returns `MovingAverageConvergenceDivergenceOutput { macd, signal, histogram }`. |
| 5 | **MACD Histogram** | (from MACD output) | `.histogram` field on MACD output. |
| 6 | **PPO** | `PercentagePriceOscillator::new(fast, slow, signal)` | Returns `PercentagePriceOscillatorOutput { ppo, signal, histogram }`. |
| 7 | **Stochastic (Fast)** | `FastStochastic::new(period)` | `Next<&DataItem>`. Returns %K. |
| 8 | **Stochastic (Slow)** | `SlowStochastic::new(period, d_period)` | `Next<&DataItem>`. Returns %D (smoothed). |
| 9 | **CCI** | `CommodityChannelIndex::new(period, constant)` | `Next<&DataItem>`. Unbounded oscillator. |
| 10 | **MFI** | `MoneyFlowIndex::new(period)` | `Next<&DataItem>`. Volume-weighted RSI, 0–100. |
| 11 | **ROC** | `RateOfChange::new(period)` | `Next<f64>`. Percentage change from N periods ago. |
| 12 | **OBV** | `OnBalanceVolume::default()` | `Next<&DataItem>`. Cumulative volume direction. |
| 13 | **Bollinger Bands** | `BollingerBands::new(period, multiplier)` | Returns `BollingerBandsOutput { average, upper, lower }`. Uses EMA internally. |
| 14 | **ATR** | `AverageTrueRange::new(period)` | `Next<&DataItem>`. Exponentially smoothed true range. |
| 15 | **True Range** | `TrueRange::default()` | `Next<&DataItem>`. Single bar true range. |
| 16 | **Keltner Channels** | `KeltnerChannel::new(ema_period, atr_period, multiplier)` | Returns `KeltnerChannelOutput { average, upper, lower }`. |
| 17 | **Chandelier Exit** | `ChandelierExit::new(period, multiplier)` | Returns `ChandelierExitOutput { long, short }`. ATR-based trailing stop levels. |
| 18 | **Standard Deviation** | `StandardDeviation::new(period)` | `Next<f64>`. |
| 19 | **Maximum** | `Maximum::new(period)` | `Next<f64>`. Highest value in window. Building block for Donchian, etc. |
| 20 | **Minimum** | `Minimum::new(period)` | `Next<f64>`. Lowest value in window. |
| 21 | **Efficiency Ratio** | `EfficiencyRatio::new(period)` | `Next<f64>`. Kaufman's ER — building block for KAMA. |
| 22 | **Mean Absolute Deviation** | `MeanAbsoluteDeviation::new(period)` | `Next<f64>`. |

**Count: 22 native structs covering 22 catalog entries.**

---

## Composable from ta-rs Primitives

These don't have a dedicated struct but can be built with straightforward Rust logic on top of ta-rs building blocks. Implementation is typically <50 lines wrapping one or two ta-rs indicators.

### Momentum Oscillators

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 23 | **Stochastic RSI** | `RelativeStrengthIndex` → feed RSI values into `FastStochastic` logic | Run RSI, then apply stochastic formula to RSI output stream over N periods. Use `Maximum`/`Minimum` on RSI values. |
| 24 | **Williams %R** | `Maximum` + `Minimum` + arithmetic | `%R = (highest_high - close) / (highest_high - lowest_low) * -100`. Same math as Fast Stochastic, different scale. |
| 25 | **Momentum Indicator** | Ring buffer + subtraction | `momentum = close - close_n_periods_ago`. Keep a VecDeque of length N. |
| 26 | **Chande Momentum Oscillator** | Sum of gains/losses similar to RSI internals | `CMO = (sum_up - sum_down) / (sum_up + sum_down) * 100`. Track running sums over N periods. |
| 27 | **Elder-Ray Bull/Bear Power** | `ExponentialMovingAverage` + DataItem high/low | `bull_power = high - ema`, `bear_power = low - ema`. One EMA, two subtractions. |
| 28 | **Dynamic Momentum Index** | `StandardDeviation` → dynamic RSI period | Compute recent volatility with SD, map to variable RSI period (shorter in high vol). Run RSI with that period. |
| 29 | **True Strength Index** | Two `ExponentialMovingAverage` instances on price change | Double-smooth the momentum: `EMA(EMA(close_change, long), short)`. Normalize by `EMA(EMA(abs(close_change), long), short)`. |
| 30 | **Connors RSI** | `RelativeStrengthIndex` × 3 + streak counting | Three RSI components: standard RSI, streak RSI (count consecutive up/down bars, apply RSI to streak), ROC percentile. Average them. |
| 31 | **Relative Vigor Index** | `SimpleMovingAverage` on (close-open) and (high-low) | `RVI = SMA(close - open, N) / SMA(high - low, N)`. Signal line is SMA of RVI. |
| 32 | **Ultimate Oscillator** | Three periods of buying pressure / true range | Compute BP and TR per bar, sum over 3 periods (7, 14, 28), weighted average. Uses `TrueRange` from ta-rs. |
| 33 | **Intraday Momentum Index** | Gains/losses based on open→close instead of close→close | Like RSI but `gain = close - open` when close > open. Same smoothing approach. |
| 34 | **Pretty Good Oscillator** | `SimpleMovingAverage` + `AverageTrueRange` | `PGO = (close - SMA(close, N)) / ATR(N)`. Two ta-rs indicators, one division. |
| 35 | **Fisher Transform** | `Maximum` + `Minimum` + log transform | Normalize price to -1..+1 using N-period high/low, then apply `0.5 * ln((1+x)/(1-x))`. |

### Trend Indicators

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 36 | **DEMA** | Two `ExponentialMovingAverage` instances | `DEMA = 2 * EMA(close, N) - EMA(EMA(close, N), N)`. Chain two EMAs. |
| 37 | **TEMA** | Three `ExponentialMovingAverage` instances | `TEMA = 3*EMA1 - 3*EMA2 + EMA3` where each is EMA of the previous. |
| 38 | **WMA** | Ring buffer + weighted sum | Linear weights: `WMA = sum(weight_i * price_i) / sum(weights)` where `weight_i = i`. |
| 39 | **HMA** | Three `SimpleMovingAverage` (or WMA) instances | `HMA = WMA(2*WMA(N/2) - WMA(N), sqrt(N))`. Compose three WMAs. |
| 40 | **KAMA** | `EfficiencyRatio` + EMA-like smoothing | Use ta-rs `EfficiencyRatio`, convert to smoothing constant between fast/slow SC, apply adaptive EMA. |
| 41 | **ZLEMA** | `ExponentialMovingAverage` + lag correction | `ZLEMA = EMA(close + (close - close_lag), period)` where `lag = (period-1)/2`. De-lag the input. |
| 42 | **VWMA** | Ring buffer of price×volume and volume | `VWMA = sum(price_i * volume_i, N) / sum(volume_i, N)`. Rolling window sum. |
| 43 | **MA Crossover Signal** | Two `ExponentialMovingAverage` (or SMA) + comparison | Maintain fast and slow MA, detect sign change of `fast - slow`. |
| 44 | **MA Slope** | Any MA + ring buffer | `slope = (ma_now - ma_n_bars_ago) / n`. Keep recent MA values in buffer. |
| 45 | **Price Distance from MA** | Any MA + arithmetic | `distance_pct = (close - ma) / ma * 100`. |
| 46 | **ADX / +DI / -DI** | `TrueRange` + `ExponentialMovingAverage` × 3 | Compute +DM/-DM per bar, smooth with EMA, divide by smoothed TR. ADX = EMA of DX. ~40 lines of logic on top of ta-rs primitives. |
| 47 | **ADXR** | ADX + ring buffer | `ADXR = (ADX_today + ADX_n_periods_ago) / 2`. |
| 48 | **Aroon Up/Down** | `Maximum` + `Minimum` (index tracking variant) | Track how many bars since highest high and lowest low over N periods. Requires index tracking — simple with a ring buffer. |
| 49 | **Aroon Oscillator** | Aroon Up - Aroon Down | Trivial once Aroon Up/Down exists. |
| 50 | **Donchian Channels** | `Maximum` + `Minimum` | `upper = Maximum::new(N).next(high)`, `lower = Minimum::new(N).next(low)`. Direct use of ta-rs. |
| 51 | **Donchian Channel Width** | Donchian + arithmetic | `width = (upper - lower) / midpoint * 100`. |
| 52 | **SuperTrend** | `AverageTrueRange` + state tracking | `basic_upper = (high+low)/2 + mult*ATR`, `basic_lower = (high+low)/2 - mult*ATR`. Track trend direction flip. ~30 lines on ATR. |
| 53 | **Parabolic SAR** | State machine with acceleration factor | No ta-rs dependency needed but uses price high/low. Track SAR, EP, AF with acceleration logic. ~50 lines. |
| 54 | **Linear Regression Slope** | Ring buffer + least-squares math | Keep N prices, compute `slope = (N*sum(i*price_i) - sum(i)*sum(price_i)) / (N*sum(i²) - sum(i)²)`. Pure math. |
| 55 | **TRIX** | Three `ExponentialMovingAverage` + ROC | `TRIX = ROC(EMA(EMA(EMA(close, N), N), N), 1)`. Chain three EMAs, then 1-period ROC. |
| 56 | **Vortex Indicator** | `TrueRange` + `SimpleMovingAverage` | Compute VM+ and VM- per bar (absolute differences of highs/lows), sum over N periods, divide by sum of TR. |
| 57 | **Mass Index** | `ExponentialMovingAverage` × 2 + summation | `EMA(high-low, 9)`, then `EMA of EMA`, sum their ratio over 25 periods. |
| 58 | **Ichimoku Cloud** | `Maximum` + `Minimum` over multiple periods + displacement | Tenkan = (Max9+Min9)/2, Kijun = (Max26+Min26)/2, Senkou A/B with displacement. Uses ta-rs Max/Min primitives. |

### Volatility Indicators

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 59 | **Bollinger %B** | `BollingerBands` + arithmetic | `%B = (close - lower) / (upper - lower)`. One line on BB output. |
| 60 | **Bollinger BandWidth** | `BollingerBands` + arithmetic | `BW = (upper - lower) / average * 100`. One line on BB output. |
| 61 | **ATRP** | `AverageTrueRange` + division | `ATRP = ATR / close * 100`. |
| 62 | **TTM Squeeze** | `BollingerBands` + `KeltnerChannel` + comparison | Squeeze on when `BB_lower > KC_lower AND BB_upper < KC_upper`. Both are native ta-rs. |
| 63 | **STARC Bands** | `SimpleMovingAverage` + `AverageTrueRange` | `upper = SMA + mult * ATR`, `lower = SMA - mult * ATR`. Two ta-rs indicators. |
| 64 | **Chaikin Volatility** | `ExponentialMovingAverage` + ROC | `EMA(high - low, N)`, then compute ROC of that EMA. |
| 65 | **Natr** | `AverageTrueRange` + division | `NATR = ATR / close * 100`. Same as ATRP. |
| 66 | **Historical Volatility** | `StandardDeviation` on log returns + annualization | Compute log returns, feed to SD, multiply by `sqrt(252)`. |
| 67 | **Choppiness Index** | `AverageTrueRange` + `Maximum` + `Minimum` | `CI = 100 * log10(sum(ATR, N) / (Max_high_N - Min_low_N)) / log10(N)`. |
| 68 | **Ulcer Index** | `Maximum` + running sum of squared drawdowns | Track rolling max, compute `pct_drawdown = (close - max) / max`, then `sqrt(mean(pct_drawdown²))`. |
| 69 | **Intraday Range Ratio** | `SimpleMovingAverage` on (high-low) | `ratio = (high - low) / SMA(high - low, N)`. |

### Volume-Based Indicators

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 70 | **VWAP** | Cumulative sum of (price × volume) / cumulative volume | Session-reset running sums. No ta-rs needed but trivial arithmetic. |
| 71 | **VWAP Deviation Bands** | VWAP + running variance calculation | Track cumulative `sum((price - vwap)² * volume)`, compute std dev bands. |
| 72 | **VWAP Distance** | VWAP + arithmetic | `distance = (close - vwap) / vwap * 100`. |
| 73 | **Accumulation/Distribution** | Cumulative sum with CLV weighting | `CLV = ((close-low) - (high-close)) / (high-low)`, `AD += CLV * volume`. Per-bar. |
| 74 | **Chaikin Money Flow** | A/D values + `SimpleMovingAverage` (or sum) | `CMF = sum(CLV * volume, N) / sum(volume, N)`. Rolling window. |
| 75 | **Klinger Volume Oscillator** | `ExponentialMovingAverage` × 2 on volume force | Compute volume force per bar, then `KVO = EMA(VF, 34) - EMA(VF, 55)`. |
| 76 | **Force Index** | `ExponentialMovingAverage` on (close_change × volume) | `FI = EMA((close - prev_close) * volume, N)`. |
| 77 | **Ease of Movement** | `SimpleMovingAverage` on distance/box_ratio | `EMV = ((high+low)/2 - (prev_high+prev_low)/2) / (volume / (high-low))`. Smooth with SMA. |
| 78 | **Volume Rate of Change** | Ring buffer + percentage change | `VROC = (volume - volume_N_ago) / volume_N_ago * 100`. Same pattern as ROC. |
| 79 | **Relative Volume** | Time-of-day average comparison | `RVOL = current_volume / avg_volume_at_this_time`. Requires historical session data — more complex but still arithmetic. |
| 80 | **Volume Spike Detector** | `SimpleMovingAverage` on volume + threshold | `spike = volume > (SMA(volume, N) * multiplier)`. |
| 81 | **Volume-Price Trend** | Cumulative sum | `VPT += volume * (close - prev_close) / prev_close`. Per-bar accumulation. |

### Composite / Multi-Factor

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 82 | **Awesome Oscillator** | Two `SimpleMovingAverage` on midpoint | `AO = SMA((high+low)/2, 5) - SMA((high+low)/2, 34)`. |
| 83 | **Accelerator Oscillator** | AO + `SimpleMovingAverage` | `AC = AO - SMA(AO, 5)`. |
| 84 | **Balance of Power** | Arithmetic on OHLC | `BOP = (close - open) / (high - low)`. Optionally smooth with SMA. |
| 85 | **Squeeze Momentum** | `BollingerBands` + `KeltnerChannel` + momentum histogram | Squeeze detection from BB/KC (both native), plus linear regression or momentum value for direction/strength. |
| 86 | **Heikin-Ashi Trend** | Modified candle math | `HA_close = (O+H+L+C)/4`, `HA_open = (prev_HA_open + prev_HA_close)/2`, etc. Stateful per-bar transform. |
| 87 | **KST Oscillator** | Four `SimpleMovingAverage` on four ROC values | `KST = SMA(ROC(10)) + 2*SMA(ROC(15)) + 3*SMA(ROC(20)) + 4*SMA(ROC(30))`. Uses `RateOfChange` + `SimpleMovingAverage` from ta-rs. |

### Price Pattern / Structure

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 88 | **Pivot Points (all types)** | Arithmetic on prev session H/L/C | `PP = (H+L+C)/3`, `R1 = 2*PP - L`, `S1 = 2*PP - H`, etc. Pure math per session. |
| 89 | **Opening Range High/Low** | `Maximum` + `Minimum` over first N bars of session | Track session start, accumulate for N minutes, freeze values. |
| 90 | **Previous Day High/Low/Close** | Session boundary tracking | Store prior session's final H/L/C. No ta-rs needed. |
| 91 | **Gap Analysis** | Arithmetic on open vs prev close | `gap = (open - prev_close) / prev_close * 100`. |
| 92 | **Support/Resistance from Swings** | `Maximum` + `Minimum` + peak/trough detection | Find local maxima/minima using a rolling comparison window. |
| 93 | **ZigZag** | State machine on price | Track current direction, reverse when move exceeds threshold. No ta-rs dependency. |

### Breadth / Macro

| # | Catalog Indicator | Composition | Implementation Sketch |
|---|-------------------|------------|----------------------|
| 94 | **Sector Relative Strength** | Two `RateOfChange` instances + ratio | `RS = ROC(stock, N) / ROC(sector_etf, N)`. |
| 95 | **Correlation** | `StandardDeviation` × 2 + covariance | Rolling covariance / (SD_x * SD_y). Can use ta-rs SD. |
| 96 | **Beta** | Correlation × (SD_stock / SD_index) | Builds on correlation + standard deviation. |

---

## Summary

| Category | Count |
|----------|-------|
| **Native ta-rs** (direct struct) | 22 |
| **Composable** (ta-rs primitives + <50 lines Rust) | 74 |
| **Subtotal: implementable with ta-rs** | **96** |
| **Custom** (order flow, candlestick patterns, complex structure) | ~67 |
| **Total catalog indicators** | **163** |

### What requires fully custom implementation (no ta-rs help)

These fall into categories where ta-rs provides no relevant primitives:

- **Order Flow / Microstructure (14 indicators)** — cumulative delta, footprint, absorption, DOM analysis, tape speed, etc. These depend on Level 2 / time-and-sales data structures, not OHLCV.
- **Candlestick Pattern Recognition (~10 patterns)** — engulfing, hammer, doji, morning/evening star, harami, marubozu, inside bar, pin bar, three soldiers/crows. These are if/else pattern matching on candle body/wick geometry.
- **Complex Adaptive Indicators (~8)** — FRAMA, Arnaud Legoux MA, McGinley Dynamic, Guppy MMA, Alligator, Ehlers MESA, Ehlers Instantaneous Trendline, QQE. These have unique smoothing algorithms not derivable from EMA/SMA.
- **External Data Indicators (~17)** — VIX, put/call ratio, TICK, TRIN, advance/decline, McClellan, treasury yields, DXY, etc. These require separate data feeds, not OHLCV computation.
- **Structural / Profile (~8)** — Volume Profile, POC, Value Area, Market Profile shapes, Initial Balance, Fibonacci retracements. These require session-level aggregation or swing point detection algorithms.
- **Anchored/Contextual (~5)** — Anchored VWAP, gap fill probability, relative volume (time-of-day), session cumulative volume, cumulative TICK. These need session-awareness or historical context.

### Recommended implementation order

1. **Phase 1:** Wrap all 22 native ta-rs indicators in our `Indicator` trait. Immediate coverage.
2. **Phase 2:** Build the ~30 most useful composables (ADX, Donchian, SuperTrend, VWAP, Stochastic RSI, Bollinger %B/BW, TTM Squeeze, Williams %R, DEMA, HMA, Awesome Oscillator, etc.).
3. **Phase 3:** Candlestick patterns (simple geometry checks).
4. **Phase 4:** Order flow indicators (requires Level 2 data infrastructure first).
5. **Phase 5:** External data / breadth indicators (requires additional data feeds).
