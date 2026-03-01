# Tool Belt Catalog — Indicators & Actions

**Adaptive Multi-Timescale Trading System | Draft v0.1 | February 2026**

A comprehensive inventory of tools that could populate the indicator and action registries. Organized by category, with notes on which timescale(s) each tool is most relevant to and what knobs (parameters) it exposes.

Timescale shorthand: **1m** = 1-minute, **5m** = 5-minute, **1h** = hourly, **1d** = daily, **1M** = monthly.

---

# PART 1: INDICATORS (Sensing Tools)

Indicators are pure functions: market state in, normalized score out. They do not make trade decisions.

---

## 1.1 Momentum Oscillators

Measure speed and magnitude of price changes. Core tools for overbought/oversold detection and divergence.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 1 | **RSI (Relative Strength Index)** | Ratio of average gains to average losses over N periods, scaled 0–100 | `period` (typically 14), `overbought_threshold` (70), `oversold_threshold` (30) | 1m, 5m, 1h |
| 2 | **Stochastic Oscillator (%K/%D)** | Compares closing price to its price range over N periods; %D is smoothed %K | `k_period` (14), `d_period` (3), `slowing` (3), `overbought` (80), `oversold` (20) | 5m, 1h |
| 3 | **Stochastic RSI** | RSI of the RSI — applies stochastic formula to RSI values for extra sensitivity | `rsi_period` (14), `stoch_period` (14), `k_smooth` (3), `d_smooth` (3) | 1m, 5m |
| 4 | **Williams %R** | Measures where current close sits relative to the high-low range over N periods; -100 to 0 scale | `period` (14), `overbought` (-20), `oversold` (-80) | 5m, 1h |
| 5 | **CCI (Commodity Channel Index)** | Measures deviation of price from its statistical mean; unbounded oscillator | `period` (20), `constant` (0.015), `overbought` (+100), `oversold` (-100) | 5m, 1h |
| 6 | **Momentum Indicator** | Simple difference between current close and close N periods ago | `period` (10) | 1m, 5m |
| 7 | **Rate of Change (ROC)** | Percentage change between current close and close N periods ago | `period` (12) | 5m, 1h |
| 8 | **True Strength Index (TSI)** | Double-smoothed momentum oscillator using two EMAs of price change | `long_period` (25), `short_period` (13), `signal_period` (7) | 1h, 1d |
| 9 | **Connors RSI** | Composite of RSI, streak RSI (consecutive up/down days), and ROC percentile rank | `rsi_period` (3), `streak_period` (2), `roc_period` (100) | 5m, 1h |
| 10 | **Dynamic Momentum Index (DYMI)** | Variable-length RSI that shortens period in volatile markets and lengthens in calm | `base_period` (14), `volatility_lookback` (10), `min_period` (3), `max_period` (30) | 1m, 5m |
| 11 | **Intraday Momentum Index (IMI)** | Candlestick-based RSI variant — compares open-to-close gains vs. losses | `period` (14) | 1m, 5m |
| 12 | **Detrended Price Oscillator (DPO)** | Removes trend to isolate cycles; shows price relative to displaced MA | `period` (20) | 1h, 1d |
| 13 | **Coppock Curve** | Long-term momentum indicator — sum of two ROC values, smoothed with WMA | `wma_period` (10), `roc_long` (14), `roc_short` (11) | 1d, 1M |
| 14 | **Pretty Good Oscillator (PGO)** | Distance of current close from SMA, measured in ATR units | `period` (14) | 5m, 1h |
| 15 | **KST Oscillator (Know Sure Thing)** | Weighted sum of four different ROC values, each smoothed | `roc_periods` [10,15,20,30], `sma_periods` [10,10,10,15], `signal_period` (9) | 1h, 1d |
| 16 | **Chande Momentum Oscillator (CMO)** | Measures momentum as ratio of up-sum minus down-sum to total sum | `period` (9), `overbought` (+50), `oversold` (-50) | 5m, 1h |
| 17 | **Elder-Ray Bull/Bear Power** | Bull power = high minus EMA; bear power = low minus EMA | `ema_period` (13) | 5m, 1h |
| 18 | **Ultimate Oscillator** | Weighted average of three different timeframe oscillators to reduce false signals | `period_1` (7), `period_2` (14), `period_3` (28), `weights` [4,2,1] | 1h |
| 19 | **Relative Vigor Index (RVI)** | Compares closing price relative to trading range, smoothed | `period` (10), `signal_period` (4) | 5m, 1h |
| 20 | **Fisher Transform** | Converts prices into Gaussian normal distribution to sharpen turning points | `period` (10) | 1m, 5m |

---

## 1.2 Trend Indicators

Identify direction and strength of trends. Most are overlays on price.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 21 | **SMA (Simple Moving Average)** | Arithmetic mean of close over N periods | `period` (20, 50, 200) | all |
| 22 | **EMA (Exponential Moving Average)** | Weighted MA giving more weight to recent prices | `period` (9, 12, 21, 26, 50) | all |
| 23 | **DEMA (Double Exponential MA)** | 2×EMA minus EMA(EMA) — reduces lag | `period` (21) | 1m, 5m, 1h |
| 24 | **TEMA (Triple Exponential MA)** | Further lag reduction over DEMA using triple smoothing | `period` (21) | 1m, 5m |
| 25 | **HMA (Hull Moving Average)** | Weighted MA designed to eliminate lag while maintaining smoothness | `period` (9, 16) | 1m, 5m |
| 26 | **WMA (Weighted Moving Average)** | Linearly weighted — most recent price gets highest weight | `period` (20) | all |
| 27 | **KAMA (Kaufman Adaptive MA)** | Adapts speed based on noise ratio; fast in trends, slow in chop | `period` (10), `fast_sc` (2), `slow_sc` (30) | 5m, 1h |
| 28 | **FRAMA (Fractal Adaptive MA)** | Adjusts smoothing based on fractal dimension of price | `period` (16), `fast_sc` (1), `slow_sc` (200) | 1h, 1d |
| 29 | **VWMA (Volume Weighted MA)** | Moving average weighted by volume — high volume candles count more | `period` (20) | 5m, 1h |
| 30 | **ZLEMA (Zero-Lag EMA)** | EMA with de-lagging correction applied to input data | `period` (21) | 1m, 5m |
| 31 | **Arnaud Legoux MA (ALMA)** | Gaussian-weighted MA with offset parameter to shift center of mass | `period` (9), `offset` (0.85), `sigma` (6) | 5m, 1h |
| 32 | **McGinley Dynamic** | Self-adjusting MA that tracks market speed automatically | `period` (14) | 5m, 1h |
| 33 | **Guppy Multiple MA (GMMA)** | Set of 12 EMAs (6 short-term + 6 long-term) — expansion/compression shows trend | `short_periods` [3,5,8,10,12,15], `long_periods` [30,35,40,45,50,60] | 1h, 1d |
| 34 | **MA Crossover Signal** | Detects when fast MA crosses slow MA (golden cross / death cross) | `fast_period`, `slow_period`, `ma_type` | all |
| 35 | **MA Slope** | Rate of change of a moving average — measures trend acceleration | `ma_period` (20), `slope_lookback` (5) | all |
| 36 | **ADX (Average Directional Index)** | Measures trend strength on 0–100 scale regardless of direction | `period` (14), `strong_trend_threshold` (25) | 5m, 1h, 1d |
| 37 | **+DI / -DI (Directional Indicators)** | Positive and negative directional movement — trend direction components of ADX | `period` (14) | 5m, 1h |
| 38 | **ADXR (ADX Rating)** | Smoothed ADX — average of current ADX and ADX N periods ago | `period` (14) | 1h, 1d |
| 39 | **Parabolic SAR** | Trailing dots that accelerate toward price; flips on trend reversal | `acceleration_start` (0.02), `acceleration_increment` (0.02), `acceleration_max` (0.20) | 5m, 1h |
| 40 | **Ichimoku Cloud** | Multi-component system: Tenkan, Kijun, Senkou A/B, Chikou Span — defines trend, support, momentum | `tenkan_period` (9), `kijun_period` (26), `senkou_b_period` (52), `displacement` (26) | 1h, 1d |
| 41 | **SuperTrend** | ATR-based trend-following overlay that flips bullish/bearish | `atr_period` (10), `multiplier` (3.0) | 5m, 1h |
| 42 | **Aroon Up/Down** | Measures time since highest high (Aroon Up) and lowest low (Aroon Down) over N periods | `period` (25) | 1h, 1d |
| 43 | **Aroon Oscillator** | Aroon Up minus Aroon Down; +100 = strong uptrend, -100 = strong downtrend | `period` (25) | 1h, 1d |
| 44 | **Vortex Indicator (VI+/VI-)** | Measures positive and negative trend movement using true range | `period` (14) | 1h, 1d |
| 45 | **TRIX** | Triple-smoothed EMA shown as percentage ROC — filters noise aggressively | `period` (15), `signal_period` (9) | 1h, 1d |
| 46 | **Linear Regression Slope** | Slope of least-squares regression line through last N closes | `period` (14) | all |
| 47 | **Linear Regression Channel** | Regression line with upper/lower bands at N standard deviations | `period` (100), `std_dev` (2.0) | 1h, 1d |
| 48 | **Mass Index** | Detects trend reversals by measuring range expansion/contraction using EMA of range | `ema_period` (9), `sum_period` (25), `reversal_threshold` (27) | 1h, 1d |
| 49 | **Alligator (Bill Williams)** | Three smoothed MAs (jaw, teeth, lips) — convergence = sleeping, divergence = trending | `jaw_period` (13), `teeth_period` (8), `lips_period` (5), offsets [8,5,3] | 1h, 1d |
| 50 | **Price Distance from MA** | Current price as percentage above/below a key moving average (mean reversion signal) | `ma_period` (20), `ma_type` | all |

---

## 1.3 Volatility Indicators

Measure how much price is moving; critical for position sizing and regime detection.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 51 | **ATR (Average True Range)** | Average of true range (max of: high-low, high-prev_close, prev_close-low) over N periods | `period` (14) | all |
| 52 | **ATRP (ATR Percentage)** | ATR as a percentage of closing price — normalizes across price levels | `period` (14) | all |
| 53 | **Bollinger Bands** | SMA ± N standard deviations — expands in volatile markets, contracts in calm | `period` (20), `std_dev` (2.0) | all |
| 54 | **Bollinger %B** | Where current price sits within the Bollinger Bands (0 = lower, 1 = upper) | `period` (20), `std_dev` (2.0) | all |
| 55 | **Bollinger BandWidth** | Width of bands as percentage of middle band — measures volatility regime | `period` (20), `std_dev` (2.0) | 1h, 1d |
| 56 | **Keltner Channels** | EMA ± ATR multiplier — similar to Bollinger but uses ATR instead of std dev | `ema_period` (20), `atr_period` (10), `multiplier` (1.5) | 5m, 1h |
| 57 | **Donchian Channels** | Highest high and lowest low over N periods — breakout detection | `period` (20) | 5m, 1h, 1d |
| 58 | **Donchian Channel Width** | Width of Donchian channel as percentage of midpoint — measures range expansion | `period` (20) | 1h, 1d |
| 59 | **TTM Squeeze** | Detects when Bollinger Bands contract inside Keltner Channels — impending breakout | `bb_period` (20), `bb_mult` (2.0), `kc_period` (20), `kc_mult` (1.5) | 5m, 1h |
| 60 | **Standard Deviation** | Statistical dispersion of closing prices over N periods | `period` (20) | all |
| 61 | **Historical Volatility (HV)** | Annualized standard deviation of log returns over N periods | `period` (20) | 1d, 1M |
| 62 | **Choppiness Index** | Measures whether market is trending or choppy using ATR ratio; 0–100 scale | `period` (14), `choppy_threshold` (61.8), `trending_threshold` (38.2) | 1h, 1d |
| 63 | **Ulcer Index** | Measures downside volatility / depth and duration of drawdowns | `period` (14) | 1d, 1M |
| 64 | **STARC Bands** | SMA ± ATR multiplier — Short Term Average Range Channels | `sma_period` (5), `atr_period` (15), `multiplier` (1.33) | 5m, 1h |
| 65 | **Chaikin Volatility** | Percentage change in EMA of high-low range — measures volatility acceleration | `ema_period` (10), `roc_period` (10) | 1h, 1d |
| 66 | **Natr (Normalized ATR)** | ATR divided by close, expressed as percentage — cross-instrument comparison | `period` (14) | all |
| 67 | **Intraday Range Ratio** | Current bar's range relative to average range over N periods — detects expansion | `period` (20) | 1m, 5m |
| 68 | **VIX Level** | CBOE Volatility Index — market's expectation of 30-day forward volatility | `regime_thresholds` [15, 20, 30] | 1M |
| 69 | **VIX Term Structure** | Ratio of short-term to long-term VIX futures — contango vs backwardation as fear signal | `front_month`, `back_month` | 1M |
| 70 | **Implied Volatility Rank/Percentile** | Where current IV sits relative to its range over past year | `lookback` (252 days) | 1d, 1M |

---

## 1.4 Volume-Based Indicators

Volume confirms price moves. Essential for distinguishing real breakouts from fakeouts.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 71 | **VWAP (Volume Weighted Average Price)** | Cumulative average price weighted by volume — resets each session | `anchor` (session start) | 1m, 5m, 1h |
| 72 | **VWAP Deviation Bands** | Standard deviation bands around VWAP — intraday Bollinger-like | `std_devs` [1, 2, 3] | 1m, 5m |
| 73 | **VWAP Distance** | Current price distance from VWAP as percentage — mean reversion signal | (derived from VWAP) | 1m, 5m |
| 74 | **Anchored VWAP** | VWAP anchored from a specific event (earnings, gap, swing high/low) | `anchor_point` | 1h, 1d |
| 75 | **OBV (On-Balance Volume)** | Cumulative volume: +volume on up closes, -volume on down closes | (no params, or `signal_period` for smoothing) | 5m, 1h |
| 76 | **Accumulation/Distribution Line** | Cumulative volume weighted by close's position within high-low range | (no params) | 5m, 1h |
| 77 | **Chaikin Money Flow (CMF)** | Average A/D value over N periods — sustained buying/selling pressure | `period` (21) | 1h, 1d |
| 78 | **Money Flow Index (MFI)** | Volume-weighted RSI — combines price momentum with volume | `period` (14), `overbought` (80), `oversold` (20) | 5m, 1h |
| 79 | **Klinger Volume Oscillator** | Compares volume flowing through a security based on trend direction | `fast_period` (34), `slow_period` (55), `signal_period` (13) | 1h, 1d |
| 80 | **Force Index (Elder)** | Price change × volume — measures strength of bulls/bears per bar | `period` (13) | 5m, 1h |
| 81 | **Ease of Movement (EMV)** | Relates price movement to volume — high values = price moving easily on low volume | `period` (14) | 1h, 1d |
| 82 | **Negative Volume Index (NVI)** | Tracks price changes on days when volume decreases — institutional activity proxy | `signal_period` (255) | 1d, 1M |
| 83 | **Positive Volume Index (PVI)** | Tracks price changes on days when volume increases — retail activity proxy | `signal_period` (255) | 1d, 1M |
| 84 | **Volume Rate of Change** | Percentage change in volume vs N periods ago — detects volume spikes | `period` (14) | 1m, 5m |
| 85 | **Relative Volume (RVOL)** | Current volume as ratio to average volume at same time of day — context-aware | `lookback_days` (20) | 1m, 5m |
| 86 | **Volume Spike Detector** | Flags when current bar volume exceeds N× average — potential institutional activity | `period` (20), `spike_multiplier` (2.5) | 1m, 5m |
| 87 | **Volume Profile (Session)** | Distribution of traded volume across price levels for the current session | `row_size` (tick increments or auto), `value_area_pct` (70%) | 5m, 1h |
| 88 | **Volume Profile POC (Point of Control)** | Price level with the highest traded volume — acts as magnet/pivot | (derived from Volume Profile) | 5m, 1h |
| 89 | **Volume Profile Value Area High/Low** | Upper and lower bounds of the value area (where 70% of volume traded) | `value_area_pct` (70%) | 5m, 1h |
| 90 | **Twiggs Money Flow** | Smoothed version of Chaikin Money Flow using Wilder smoothing | `period` (21) | 1h, 1d |
| 91 | **Trade Volume Index (TVI)** | Accumulates volume based on tick direction — detects accumulation at bid vs ask | `min_tick_value` | 1m, 5m |
| 92 | **Volume Weighted Moving Average** | Moving average where each price is weighted by its bar's volume | `period` (20) | 5m, 1h |
| 93 | **Session Cumulative Volume** | Running total of volume within current session — compares to average session pace | `lookback_days` (20) | 1m, 5m |
| 94 | **Volume-Price Trend (VPT)** | Cumulative volume adjusted by percentage change in price — similar to OBV but proportional | (no params) | 5m, 1h |

---

## 1.5 Order Flow / Market Microstructure

Analyze the mechanics of how orders are executed. Most valuable at 1m timescale. Require Level 2 / time-and-sales data.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 95 | **Cumulative Delta** | Running net difference between volume traded at ask (buying) vs bid (selling) | `reset` (per session or rolling) | 1m, 5m |
| 96 | **Delta per Bar** | Net buy vs sell volume within each candle — positive = buyers aggressive | (per bar calculation) | 1m |
| 97 | **Delta Divergence** | Price makes new high but cumulative delta doesn't confirm (or vice versa) | `lookback` (20 bars) | 1m, 5m |
| 98 | **Bid-Ask Imbalance Ratio** | Ratio of volume at bid vs volume at ask for recent trades — aggressor detection | `window` (N ticks or N seconds) | 1m |
| 99 | **Footprint Imbalance** | Within a candle, identifies price levels where buy/sell volume ratio exceeds threshold | `imbalance_ratio` (3.0) | 1m |
| 100 | **Absorption Detection** | Detects when large orders absorb aggressive selling/buying without price movement — hidden accumulation | `volume_threshold`, `price_threshold` | 1m |
| 101 | **Exhaustion Detection** | High volume with declining price movement — aggressive participants losing control | `volume_lookback` (10), `price_movement_threshold` | 1m |
| 102 | **Large Print Detection** | Flags individual trades exceeding N× average trade size — institutional footprint | `size_multiplier` (5.0), `lookback` (100 trades) | 1m |
| 103 | **Tape Speed / Velocity** | Rate of trade execution (trades per second) — acceleration indicates urgency | `window` (10 seconds) | 1m |
| 104 | **DOM Depth Imbalance** | Ratio of resting bid size vs ask size in the order book at N levels | `levels` (5, 10), `update_frequency` | 1m |
| 105 | **Iceberg Order Detection** | Identifies repeating same-size fills at a single price level — hidden large order | `repeat_threshold` (3), `size_tolerance` | 1m |
| 106 | **Aggressor Ratio** | Percentage of total volume initiated by market orders (vs limit) — measures urgency | `window` (N bars or N seconds) | 1m |
| 107 | **Spread Analysis** | Current bid-ask spread relative to average — wider spread = less liquidity | `lookback` (100 bars) | 1m |
| 108 | **Up/Down Tick Ratio** | Ratio of upticks to downticks in recent time window — short-term directional pressure | `window` (60 seconds) | 1m |

---

## 1.6 Price Pattern / Structure

Analyze price levels and geometric patterns that reflect market memory and participant behavior.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 109 | **Pivot Points (Standard)** | Support/resistance levels calculated from prior period's high, low, close | `pivot_type` (standard), `period` (daily) | 5m, 1h |
| 110 | **Pivot Points (Fibonacci)** | Fibonacci-ratio based pivots from prior period's range | `period` (daily) | 5m, 1h |
| 111 | **Pivot Points (Camarilla)** | Tighter intraday levels based on prior session — designed for day trading | `period` (daily) | 1m, 5m |
| 112 | **Pivot Points (Woodie)** | Gives extra weight to prior close vs high/low | `period` (daily) | 5m, 1h |
| 113 | **Fibonacci Retracement** | Key ratios (23.6%, 38.2%, 50%, 61.8%, 78.6%) between swing high and low | `swing_lookback` (auto or manual) | 5m, 1h, 1d |
| 114 | **Fibonacci Extensions** | Projected levels beyond swing range (127.2%, 161.8%, 200%, 261.8%) — profit targets | `swing_lookback` | 5m, 1h |
| 115 | **Support/Resistance from Swing Points** | Identifies recent swing highs/lows as horizontal S/R levels | `swing_strength` (N bars left/right), `max_levels` | all |
| 116 | **Previous Day High/Low/Close (PDH/PDL/PDC)** | Prior session's extremes and close — key intraday reference levels | (no params — fixed per session) | 1m, 5m, 1h |
| 117 | **Opening Range High/Low** | High and low of the first N minutes of the session — ORB strategy reference | `opening_minutes` (5, 15, 30) | 1m, 5m |
| 118 | **Gap Analysis** | Distance between current open and previous close; classified as gap up/down, full/partial | `significance_threshold` (0.5%) | 5m, 1h, 1d |
| 119 | **Gap Fill Probability** | Historical likelihood that a gap of size X gets filled within the session | `lookback_days` (60), `gap_size_bucket` | 5m |
| 120 | **ZigZag** | Connects significant swing points filtering out moves smaller than X% — defines market structure | `min_move_pct` (1.0%) or `min_move_atr` (1.5) | 1h, 1d |
| 121 | **Candlestick: Engulfing** | Bullish/bearish pattern where current body fully engulfs prior body | (pattern detection, no params) | 1m, 5m |
| 122 | **Candlestick: Hammer / Shooting Star** | Single candle with long lower/upper wick — reversal signal | `wick_body_ratio` (2.0) | 1m, 5m |
| 123 | **Candlestick: Doji** | Open ≈ close with wicks — indecision signal | `body_threshold` (0.1% of range) | 1m, 5m |
| 124 | **Candlestick: Morning/Evening Star** | Three-candle reversal pattern — trend candle, small body, reversal candle | (pattern detection) | 5m, 1h |
| 125 | **Candlestick: Harami** | Small body contained within prior large body — potential reversal | (pattern detection) | 5m, 1h |
| 126 | **Candlestick: Three White Soldiers / Three Black Crows** | Three consecutive strong candles in same direction — continuation | (pattern detection) | 5m, 1h |
| 127 | **Candlestick: Marubozu** | Full-body candle with no/minimal wicks — strong directional conviction | `max_wick_pct` (5%) | 1m, 5m |
| 128 | **Candlestick: Inside Bar** | Bar whose range is entirely within previous bar's range — consolidation/breakout setup | (pattern detection) | 5m, 1h |
| 129 | **Candlestick: Pin Bar** | Candle with long rejection wick and small body — reversal at key level | `wick_body_ratio` (2.5), `wick_pct_of_range` (66%) | 1m, 5m |
| 130 | **Market Profile Shape** | Classifies session volume distribution: normal (D-shape), trending (P/b shape), double distribution | `session_period`, `tick_size` | 1h, 1d |
| 131 | **Initial Balance** | First hour's range — framework for whether day will be trending or rotational | `ib_duration` (60 minutes) | 1h |
| 132 | **Price Distance from Key Level** | How far current price is from nearest PDH/PDL/pivot/VWAP — proximity to inflection | `level_types` (list of reference levels) | all |

---

## 1.7 Composite / Multi-Factor Indicators

Combine multiple calculations into a single signal. Often the most powerful but least transparent.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 133 | **MACD** | Difference between fast and slow EMA; signal line is EMA of MACD line | `fast_period` (12), `slow_period` (26), `signal_period` (9) | 5m, 1h |
| 134 | **MACD Histogram** | MACD minus signal line — acceleration/deceleration of momentum | (derived from MACD) | 5m, 1h |
| 135 | **PPO (Percent Price Oscillator)** | MACD expressed as percentage — normalizes across price levels | `fast_period` (12), `slow_period` (26), `signal_period` (9) | 5m, 1h |
| 136 | **Awesome Oscillator (AO)** | Difference between 5-period and 34-period SMA of midpoint (H+L)/2 | `fast_period` (5), `slow_period` (34) | 5m, 1h |
| 137 | **Accelerator Oscillator** | AO minus 5-period SMA of AO — measures acceleration of momentum | (derived from AO) | 5m, 1h |
| 138 | **Balance of Power (BOP)** | (Close - Open) / (High - Low) — measures control of buyers vs sellers | `smoothing_period` (14) | 5m, 1h |
| 139 | **QQE (Quantitative Qualitative Estimation)** | Smoothed RSI with dynamic trailing stop levels — combines momentum with volatility | `rsi_period` (14), `smoothing` (5), `qq_multiplier` (4.236) | 5m, 1h |
| 140 | **Market Facilitation Index (BW MFI)** | Range divided by volume — measures price efficiency per unit of volume | (per bar calculation) | 5m, 1h |
| 141 | **Ehlers Fisher Transform** | Gaussian normalization of price to create sharper turning point signals | `period` (10) | 1m, 5m |
| 142 | **Ehlers Instantaneous Trendline** | Adaptive filter that separates trend from cycle components | `alpha` (0.07) | 1h, 1d |
| 143 | **Ehlers MESA Adaptive MA** | Uses Hilbert Transform to measure dominant cycle and adapt MA length | (auto-adaptive) | 1h, 1d |
| 144 | **Heikin-Ashi Trend** | Modified candle calculation that smooths trend; direction of HA candles as trend signal | (no params — derived candle type) | 5m, 1h |
| 145 | **Squeeze Momentum (LazyBear)** | TTM Squeeze combined with momentum histogram — identifies compression + direction | `bb_length` (20), `bb_mult` (2.0), `kc_length` (20), `kc_mult` (1.5) | 5m, 1h |
| 146 | **RSI + MACD Confluence** | Combined score of RSI and MACD — higher when both agree on direction | `rsi_period`, `macd_fast`, `macd_slow` | 5m, 1h |

---

## 1.8 Breadth / Correlation / Macro Context

External signals that provide regime context. Primarily for longer timescale agents.

| # | Tool | Description | Key Knobs | Best Timescales |
|---|------|-------------|-----------|-----------------|
| 147 | **Sector Relative Strength** | Performance of stock's sector ETF vs SPY over N periods — rotation signal | `period` (20), `sector_etf` | 1d, 1M |
| 148 | **Correlation to Index** | Rolling correlation of stock returns to SPY/QQQ — idiosyncratic vs systematic | `period` (20) | 1d, 1M |
| 149 | **Beta** | Rolling beta coefficient — sensitivity to market moves | `period` (60) | 1d, 1M |
| 150 | **NYSE TICK Index** | Number of NYSE stocks on uptick minus downtick — real-time market breadth | `extreme_threshold` (+/- 1000) | 1m, 5m |
| 151 | **TRIN / Arms Index** | (Advancing issues / Declining issues) / (Advancing volume / Declining volume) | `threshold_oversold` (2.0), `threshold_overbought` (0.5) | 1h, 1d |
| 152 | **Advance/Decline Line** | Cumulative sum of advancing minus declining issues — broad market health | (cumulative) | 1d, 1M |
| 153 | **McClellan Oscillator** | EMA difference of advance-decline data — market breadth momentum | `fast_period` (19), `slow_period` (39) | 1d, 1M |
| 154 | **McClellan Summation Index** | Cumulative sum of McClellan Oscillator — longer-term breadth trend | (cumulative) | 1M |
| 155 | **High-Low Index** | Ratio of new 52-week highs to sum of new highs + new lows — market health | `smoothing_period` (10) | 1d, 1M |
| 156 | **Put/Call Ratio** | Total put volume / call volume — contrarian sentiment indicator | `smoothing_period` (5, 21) | 1d, 1M |
| 157 | **VIX Level as Regime** | Classifies current VIX into regimes: low (<15), normal (15-20), elevated (20-30), extreme (>30) | `regime_thresholds` [15,20,30] | 1d, 1M |
| 158 | **VIX/VIX3M Ratio** | Short-term vs medium-term volatility expectations — fear gauge | `threshold` (1.0 = contango/backwardation boundary) | 1d, 1M |
| 159 | **US Dollar Index (DXY) Trend** | Direction of DXY as macro context — inverse correlation with equities | `ma_period` (50) | 1d, 1M |
| 160 | **10Y Treasury Yield Direction** | Rising/falling yields as risk sentiment proxy | `ma_period` (20) | 1d, 1M |
| 161 | **Intermarket Divergence** | Detects when correlated instruments diverge (e.g., SPY up but QQQ down) | `correlated_pairs`, `divergence_threshold` | 1h, 1d |
| 162 | **Cumulative TICK** | Running sum of NYSE TICK values through the session — intraday breadth momentum | `reset` (per session) | 1m, 5m |
| 163 | **% of Stocks Above MA** | Percentage of index constituents trading above their 50/200 day MA — market regime | `ma_period` (50 or 200) | 1d, 1M |

---

# PART 2: ACTIONS (Doing Tools)

Actions affect position lifecycle. They consume indicator scores and market state to produce trade decisions.

---

## 2.1 Entry Strategies

Each entry type is an Action with phase = `Entry`. It evaluates market conditions and produces an `Enter` signal when criteria are met.

### Breakout Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 1 | **Donchian Channel Breakout** | Enter when price breaks above/below N-period high/low channel | `period` (20), `confirmation_bars` (1), `volume_filter` (bool) |
| 2 | **Bollinger Band Breakout** | Enter when price closes outside Bollinger Bands after squeeze | `bb_period` (20), `bb_std` (2.0), `squeeze_required` (bool) |
| 3 | **Opening Range Breakout (ORB)** | Enter when price breaks above/below high/low of first N minutes | `opening_minutes` (5, 15, 30), `buffer_pct` (0.1%), `volume_confirm` (bool) |
| 4 | **Previous Day High/Low Breakout** | Enter on break of prior session's high or low | `buffer_atr` (0.1), `time_filter` (not in first 5 min) |
| 5 | **Consolidation Breakout** | Enter when price breaks out of narrow range (low ATR / Bollinger squeeze) | `squeeze_bars` (10), `atr_threshold` (percentile), `breakout_atr_multiple` (1.5) |
| 6 | **Range Breakout** | Enter when price exceeds N-bar range by threshold — generic version | `range_period` (10), `breakout_pct` (0.5%) |
| 7 | **VWAP Band Breakout** | Enter when price breaks above/below VWAP + N standard deviations | `std_dev_threshold` (2.0), `volume_confirm` (bool) |
| 8 | **ATR Breakout** | Enter when current bar range exceeds N× ATR — volatility expansion | `atr_period` (14), `multiplier` (2.0) |
| 9 | **Volume-Confirmed Breakout** | Any breakout that requires volume to be N× above average | `volume_multiplier` (1.5), `base_breakout_type` |
| 10 | **Initial Balance Breakout** | Enter when price breaks above/below the first hour's range | `ib_period` (60 min), `extension_target` (1.0× IB range) |

### Pullback / Retracement Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 11 | **Moving Average Pullback** | Enter on pullback to a rising/falling key MA in direction of trend | `ma_period` (9, 21), `ma_type` (EMA), `trend_filter` (ADX > 25) |
| 12 | **VWAP Pullback** | Enter on first retest of VWAP after price has moved away — mean reversion toward institutional average | `min_distance_before_pullback` (0.3%), `confirmation` (bounce candle) |
| 13 | **Fibonacci Retracement Entry** | Enter at key Fibonacci level (38.2%, 50%, 61.8%) with reversal candle | `fib_level` (0.618), `swing_lookback`, `confirmation_pattern` |
| 14 | **Trendline Pullback** | Enter when price retests a dynamic trendline from below/above | `trendline_method` (linear regression, swing points), `touch_count` (min 2) |
| 15 | **Keltner Channel Pullback** | Enter when price pulls back to middle EMA within trending Keltner channel | `kc_period` (20), `kc_mult` (1.5), `trend_filter` |
| 16 | **SuperTrend Pullback** | Enter when price touches SuperTrend line without flipping direction | `atr_period` (10), `multiplier` (3.0) |
| 17 | **EMA Cloud Pullback** | Enter when price pulls back into the space between fast and slow EMA | `fast_ema` (9), `slow_ema` (21) |

### Mean Reversion Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 18 | **Bollinger Band Bounce** | Enter when price touches or exceeds lower/upper band and reverses | `bb_period` (20), `bb_std` (2.0), `reversal_candle_required` (bool) |
| 19 | **RSI Extreme Reversal** | Enter when RSI crosses back from extreme (above 70 → below, or below 30 → above) | `rsi_period` (14), `overbought` (70), `oversold` (30) |
| 20 | **VWAP Mean Reversion** | Enter when price is N std devs from VWAP, betting on return to mean | `std_dev_entry` (2.0), `time_filter` (not in first/last 30 min) |
| 21 | **Z-Score Mean Reversion** | Enter when price z-score (from rolling mean/std) exceeds threshold | `lookback` (20), `entry_zscore` (2.0), `exit_zscore` (0.5) |
| 22 | **Keltner Channel Mean Reversion** | Enter when price touches outer Keltner band and reverses toward middle | `kc_period` (20), `kc_mult` (2.0) |
| 23 | **CCI Extreme Reversal** | Enter when CCI crosses back from extreme (+200/-200) | `cci_period` (20), `extreme_threshold` (200) |
| 24 | **Stochastic Oversold/Overbought** | Enter when %K crosses %D in extreme zone | `k_period` (14), `d_period` (3), `zone_threshold` (20/80) |

### Momentum / Trend Continuation Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 25 | **MACD Crossover** | Enter when MACD line crosses signal line in trending market | `fast` (12), `slow` (26), `signal` (9), `trend_filter` (bool) |
| 26 | **MACD Zero-Line Cross** | Enter when MACD crosses above/below zero — stronger trend signal | `fast` (12), `slow` (26) |
| 27 | **Moving Average Crossover** | Enter when fast MA crosses above slow MA | `fast_period` (9), `slow_period` (21), `ma_type` (EMA) |
| 28 | **ADX Trend Entry** | Enter in direction of +DI/-DI when ADX rises above threshold | `adx_period` (14), `adx_threshold` (25), `di_spread_min` (5) |
| 29 | **SuperTrend Flip** | Enter when SuperTrend indicator changes direction | `atr_period` (10), `multiplier` (3.0) |
| 30 | **Ichimoku Cloud Breakout** | Enter when price crosses above/below the Kumo cloud | `tenkan` (9), `kijun` (26), `senkou_b` (52) |
| 31 | **Aroon Crossover** | Enter when Aroon Up crosses above Aroon Down (or vice versa) | `period` (25) |
| 32 | **RSI Momentum (50-line)** | Enter when RSI crosses above/below 50 — momentum confirmation | `rsi_period` (14), `threshold` (50) |
| 33 | **Parabolic SAR Flip** | Enter when SAR dots flip from above to below price (or vice versa) | `af_start` (0.02), `af_max` (0.20) |
| 34 | **TRIX Crossover** | Enter when TRIX crosses its signal line — slow but reliable trend signal | `period` (15), `signal_period` (9) |
| 35 | **Vortex Crossover** | Enter when VI+ crosses above VI- (bullish) or vice versa | `period` (14) |
| 36 | **Score Threshold Entry** | Enter when composite timescale score exceeds configurable threshold | `entry_threshold` (0.65), `hard_gate_timescales` |

### Order Flow Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 37 | **Absorption Entry** | Enter when aggressive selling is absorbed at support (or buying at resistance) without price movement | `volume_threshold`, `price_stability_bars` (3), `delta_imbalance` |
| 38 | **Exhaustion Entry** | Enter counter-trend when aggressive volume fails to move price further | `volume_spike_mult` (3.0), `price_stall_bars` (5) |
| 39 | **Delta Divergence Entry** | Enter when price makes new high/low but cumulative delta diverges | `divergence_lookback` (20), `delta_threshold` |
| 40 | **Large Print Reversal** | Enter after unusually large print at a key level — institutional activity | `size_multiplier` (5.0), `at_key_level` (bool) |
| 41 | **DOM Sweep Entry** | Enter when one side of the order book gets swept rapidly | `sweep_speed`, `level_depth` (5) |

### Pattern-Based Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 42 | **Engulfing Candle Entry** | Enter on bullish/bearish engulfing at support/resistance | `min_body_ratio` (1.5×), `at_key_level` (bool) |
| 43 | **Pin Bar / Hammer Entry** | Enter after rejection candle with long wick at key level | `wick_body_ratio` (2.5), `at_key_level` (bool) |
| 44 | **Gap-and-Go** | Enter in gap direction when opening volume confirms gap strength | `min_gap_pct` (0.5%), `volume_confirm_period` (5 min) |
| 45 | **Gap Fill Entry** | Enter against gap direction betting on mean reversion to fill gap | `min_gap_pct` (0.5%), `max_gap_pct` (2.0%), `fade_target` (PDC) |
| 46 | **Inside Bar Breakout** | Enter when price breaks out of inside bar range | `min_bars_inside` (1), `breakout_direction_filter` |
| 47 | **Three Bar Play** | Enter after three-bar reversal pattern (strong candle, inside bar, breakout) | (pattern detection) |
| 48 | **Double Bottom/Top Entry** | Enter after W/M pattern confirmation at support/resistance | `tolerance_pct` (0.5%), `confirmation_break` |
| 49 | **Flag/Pennant Breakout** | Enter on breakout from consolidation pattern after strong move | `pole_min_atr` (2.0), `consolidation_bars` (5-15) |

### Time-Based Entries

| # | Entry Type | Description | Key Knobs |
|---|------------|-------------|-----------|
| 50 | **Opening Drive Entry** | Enter in first 15-30 minutes in direction of opening momentum | `open_period` (15 min), `momentum_threshold` |
| 51 | **First Pullback After Open** | Wait for initial move, then enter on first pullback to VWAP/MA | `wait_period` (15 min), `pullback_target` (VWAP) |
| 52 | **Lunch Reversal** | Enter against morning trend during 11:30-1:30 low-volume period | `reversal_time_window`, `morning_trend_threshold` |
| 53 | **Power Hour Momentum** | Enter momentum trades during 3:00-4:00 high-volume close | `start_time` (15:00), `momentum_indicator` |

---

## 2.2 Exit Strategies

### Stop Loss Types

| # | Exit Type | Description | Key Knobs |
|---|-----------|-------------|-----------|
| 54 | **Fixed Dollar Stop** | Exit when loss reaches $X per share/contract | `dollar_amount` |
| 55 | **Fixed Percentage Stop** | Exit when loss reaches X% from entry | `stop_pct` (1.0%) |
| 56 | **ATR-Based Stop** | Exit when price moves N× ATR against entry — adapts to volatility | `atr_period` (14), `multiplier` (2.0) |
| 57 | **Volatility-Adjusted Stop** | Wider stops in high-vol, tighter in low-vol — uses current ATR regime | `base_multiplier` (2.0), `vol_scaling_factor` |
| 58 | **Support/Resistance Stop** | Place stop beyond nearest S/R level — structural stop | `buffer_atr` (0.2), `level_source` (swing low, pivot, VWAP) |
| 59 | **Swing Low/High Stop** | Place stop below recent swing low (longs) or above swing high (shorts) | `swing_lookback` (5 bars), `buffer_atr` (0.1) |
| 60 | **VWAP Stop** | Exit if price closes on wrong side of VWAP — thesis invalidation | `confirmation_bars` (1-2) |
| 61 | **Initial Risk Stop** | Fixed stop set at entry based on predetermined risk budget | `risk_amount`, `derived_from_sizing` |

### Trailing Stop Types

| # | Exit Type | Description | Key Knobs |
|---|-----------|-------------|-----------|
| 62 | **Fixed Trailing Stop** | Trail by fixed dollar/point amount from highest profit | `trail_amount` |
| 63 | **Percentage Trailing** | Trail by fixed percentage from high water mark | `trail_pct` (1.0%) |
| 64 | **ATR Trailing Stop** | Trail by N× ATR from high water mark — volatility-adaptive | `atr_period` (14), `multiplier` (2.5) |
| 65 | **Chandelier Exit** | Trail from highest high (long) or lowest low (short) using ATR multiple | `lookback` (22), `atr_period` (14), `multiplier` (3.0) |
| 66 | **Parabolic SAR Trailing** | Use Parabolic SAR as dynamic trailing stop — accelerates toward price | `af_start` (0.02), `af_increment` (0.02), `af_max` (0.20) |
| 67 | **Moving Average Trailing** | Exit when price closes below/above a fast MA | `ma_period` (9, 20), `ma_type` (EMA) |
| 68 | **Donchian Channel Trailing** | Trail using N-period lowest low (longs) or highest high (shorts) | `period` (10, 20) |
| 69 | **Keltner Channel Trailing** | Trail using lower/upper Keltner channel band | `ema_period` (20), `atr_mult` (1.5) |
| 70 | **Stepping / Ratchet Trailing** | Move stop in fixed increments as profit accumulates (e.g., every $0.50 profit, tighten by $0.25) | `step_size`, `tighten_amount` |
| 71 | **Time-Delayed Trailing** | Only move trail after favorable price persists for N bars — avoids spike fakeouts | `delay_bars` (3), `base_trail_type` |
| 72 | **Yo-Yo Stop** | Alternates between tight and loose stops based on price behavior | `tight_mult`, `loose_mult`, `switch_condition` |
| 73 | **Highest High / Lowest Low Trailing** | Simple trail at N-bar lowest low (longs) or highest high (shorts) | `lookback_bars` (5, 10, 20) |

### Take Profit Types

| # | Exit Type | Description | Key Knobs |
|---|-----------|-------------|-----------|
| 74 | **Fixed Dollar Target** | Exit at fixed dollar profit per share/contract | `target_amount` |
| 75 | **Fixed Percentage Target** | Exit at X% profit from entry | `target_pct` (1.0%) |
| 76 | **ATR-Multiple Target** | Exit at N× ATR from entry — scales with volatility | `atr_period` (14), `multiplier` (3.0) |
| 77 | **Risk-Multiple Target (R-Multiple)** | Exit at N× initial risk — e.g., 2R means profit = 2× stop distance | `r_multiple` (2.0, 3.0) |
| 78 | **Fibonacci Extension Target** | Exit at Fibonacci extension level (127.2%, 161.8%) of the setup swing | `fib_level` (1.618) |
| 79 | **Resistance/Support Target** | Exit at next identified S/R level in direction of trade | `level_source` (pivot, PDH/PDL, swing point) |
| 80 | **VWAP Band Target** | Exit at N std dev VWAP band on opposite side | `std_dev_target` (2.0) |
| 81 | **Bollinger Band Target** | Exit when price reaches opposite Bollinger Band | `bb_period` (20), `bb_std` (2.0) |
| 82 | **Previous High/Low Target** | Exit at prior session's high (longs) or low (shorts) | (no params) |
| 83 | **Partial Target Ladder** | Take partial profits at multiple levels (e.g., 33% at 1R, 33% at 2R, rest trailing) | `levels` [{pct: 0.33, target: 1R}, ...] |

### Time-Based Exits

| # | Exit Type | Description | Key Knobs |
|---|-----------|-------------|-----------|
| 84 | **Maximum Hold Time** | Exit if position open longer than N minutes regardless of P&L | `max_minutes` (30, 60, 120) |
| 85 | **Session Close Exit** | Force exit before market close — eliminates overnight risk | `exit_time` (15:55), `warning_time` (15:45) |
| 86 | **No New Entries Cutoff** | Prevent new entries after specified time but let existing trades run | `cutoff_time` (15:30) |
| 87 | **Avoid First N Minutes** | No entries during opening volatility period | `skip_minutes` (5, 15) |
| 88 | **Event-Based Time Exit** | Force exit before known events (FOMC, earnings, etc.) | `event_calendar`, `exit_minutes_before` (30) |
| 89 | **Extended Hold Override** | Allow max hold to extend if position is profitable and momentum continues | `profit_threshold_to_extend`, `extended_max_minutes` |

### Signal-Based Exits

| # | Exit Type | Description | Key Knobs |
|---|-----------|-------------|-----------|
| 90 | **Indicator Reversal Exit** | Exit when a specific indicator gives an opposite signal | `indicator_name`, `reversal_condition` |
| 91 | **Composite Score Degradation** | Exit when composite timescale score drops below threshold while in position | `exit_threshold` (-0.3) |
| 92 | **Moving Average Crossover Exit** | Exit when fast MA crosses back through slow MA against position | `fast_ma`, `slow_ma` |
| 93 | **Trend Exhaustion Exit** | Exit when ADX starts declining from peak while in trend trade | `adx_period` (14), `decline_bars` (3) |
| 94 | **Volume Dry-Up Exit** | Exit when volume drops significantly during a trade — conviction fading | `volume_drop_threshold` (50% of entry volume), `lookback` (5 bars) |
| 95 | **Momentum Divergence Exit** | Exit when price makes new high/low but momentum indicator diverges | `indicator` (RSI, MACD), `divergence_bars` (5) |
| 96 | **Stop-and-Reverse** | Exit current position and immediately enter opposite direction | `reversal_signal_source`, `size_for_new_position` |

### Breakeven Mechanisms

| # | Exit Type | Description | Key Knobs |
|---|-----------|-------------|-----------|
| 97 | **Breakeven Stop** | Move stop to entry price after position profits by X amount | `profit_trigger` (1R or specific $), `include_commission` (bool) |
| 98 | **Breakeven Plus** | Move stop to entry + small profit after hitting trigger — guarantees small win | `profit_trigger`, `plus_amount` (0.1%) |
| 99 | **Stepped Breakeven** | Move to breakeven, then to +1R, then to +2R as price advances | `steps` [{trigger: 1R, stop: 0R}, {trigger: 2R, stop: 1R}] |

---

## 2.3 Position Monitoring Actions

These run continuously while a position is open.

| # | Action | Description | Key Knobs |
|---|--------|-------------|-----------|
| 100 | **Profit-Based Stop Tightening** | Reduce trailing distance as unrealized profit grows | `tighten_schedule` [{profit: 1R, trail: 2ATR}, {profit: 2R, trail: 1.5ATR}] |
| 101 | **Time-Based Stop Tightening** | Tighten stops as hold duration increases — pressure to reach thesis | `tighten_after_minutes` (15), `tighten_rate` |
| 102 | **Volatility Regime Adjustment** | Widen stops if intraday volatility spikes, tighten if it compresses | `atr_lookback` (14), `adjustment_factor` |
| 103 | **Score-Based Stop Adjustment** | Tighten stop if composite score deteriorates while in position | `score_thresholds`, `tightening_schedule` |
| 104 | **Partial Profit Taking** | Close portion of position at predefined profit levels | `levels` [{pct_of_position: 50, at_profit: 1R}, ...] |
| 105 | **Pyramiding / Add to Winner** | Add to position after pullback within trending trade — increases size | `max_adds` (2), `add_trigger` (pullback to MA), `add_size_fraction` (0.5) |
| 106 | **Scale-In on Pullback** | Enter partial position initially, add rest on pullback confirmation | `initial_fraction` (0.5), `pullback_trigger` |
| 107 | **Scale-Out Rules** | Systematic reduction: e.g., 50% at 1R, 25% at 2R, 25% trailing | `scale_schedule` [{pct: 50, trigger: 1R}, ...] |
| 108 | **Thesis Invalidation Flag** | Flag the trade for review when conditions change but stop hasn't hit — early warning | `invalidation_conditions` (score drops, volume dies, trend changes) |
| 109 | **Trailing Stop Upgrade** | Switch from wider to tighter trailing stop type after profit milestone | `upgrade_at_profit` (2R), `initial_trail_type`, `upgraded_trail_type` |

---

## 2.4 Position Sizing Methods

These determine how large each position should be at entry.

| # | Method | Description | Key Knobs |
|---|--------|-------------|-----------|
| 110 | **Fixed Fractional** | Risk X% of total capital per trade | `risk_pct` (1.0%, 2.0%) |
| 111 | **Fixed Dollar** | Risk fixed dollar amount per trade regardless of account size | `risk_dollars` (200) |
| 112 | **ATR-Based Sizing** | Position size = risk budget / (ATR × multiplier) — volatility-adaptive | `atr_period` (14), `atr_multiplier` (2.0), `risk_budget` |
| 113 | **Volatility-Scaled Sizing** | Inverse volatility weighting — smaller positions in volatile markets | `vol_lookback` (20), `target_vol` (annualized %) |
| 114 | **Kelly Criterion** | Optimal fraction based on win rate and win/loss ratio — maximizes geometric growth | `win_rate`, `avg_win_loss_ratio`, `lookback_trades` (100) |
| 115 | **Fractional Kelly (Half/Quarter)** | Kelly fraction × 0.25 or 0.5 — reduces volatility at cost of some growth | `kelly_fraction` (0.25, 0.50) |
| 116 | **Optimal F** | Tests all fractions against historical trade series to find maximum growth | `trade_history_lookback`, `min_f` (0.01), `max_f` (0.30) |
| 117 | **Score-Based Sizing** | Higher composite confidence score = larger position within risk limits | `min_score`, `max_score`, `min_size_fraction`, `max_size_fraction` |
| 118 | **Drawdown-Adjusted Sizing** | Reduce size during losing streaks, optionally increase during winning streaks | `consecutive_loss_threshold` (3), `reduction_factor` (0.5), `recovery_trades` (5) |
| 119 | **Risk Parity** | Equalize risk contribution across concurrent positions | `target_risk_per_position`, `max_positions` |
| 120 | **Max Exposure Limits** | Cap total deployed capital and/or number of concurrent positions | `max_capital_pct` (10%), `max_positions` (3) |
| 121 | **Regime-Adjusted Sizing** | Scale position size based on VIX regime or volatility environment | `regime_multipliers` {low: 1.2, normal: 1.0, high: 0.6, extreme: 0.3} |
| 122 | **CPPI (Constant Proportion Portfolio Insurance)** | Size based on cushion above floor value — protects capital with growth exposure | `floor_pct` (90%), `multiplier` (3.0) |
| 123 | **Fixed Ratio** | Add contract/unit after every N dollars of profit (Ryan Jones method) | `delta` ($5000 per additional unit) |
| 124 | **Percent of Equity** | Size as fixed percentage of current equity — scales with account | `equity_pct` (5%) |

---

# Summary Statistics

| Category | Count |
|----------|-------|
| **Indicators — Momentum Oscillators** | 20 |
| **Indicators — Trend** | 30 |
| **Indicators — Volatility** | 20 |
| **Indicators — Volume-Based** | 24 |
| **Indicators — Order Flow / Microstructure** | 14 |
| **Indicators — Price Pattern / Structure** | 24 |
| **Indicators — Composite / Multi-Factor** | 14 |
| **Indicators — Breadth / Correlation / Macro** | 17 |
| **TOTAL INDICATORS** | **163** |
| | |
| **Actions — Entry Strategies** | 53 |
| **Actions — Exit: Stop Loss** | 8 |
| **Actions — Exit: Trailing Stops** | 12 |
| **Actions — Exit: Take Profits** | 10 |
| **Actions — Exit: Time-Based** | 6 |
| **Actions — Exit: Signal-Based** | 7 |
| **Actions — Exit: Breakeven** | 3 |
| **Actions — Position Monitoring** | 10 |
| **Actions — Position Sizing** | 15 |
| **TOTAL ACTIONS** | **124** |
| | |
| **GRAND TOTAL TOOLS** | **287** |
