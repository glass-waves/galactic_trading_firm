# action items from research synthesis

**date:** 2026-03-02
**source:** `docs/research_synthesis.md`, 44 papers across multi-agent systems, intraday ML, candlestick analysis, multi-timeframe modeling, and market microstructure

---

## 1. agentic architecture changes

### 1.1 consolidate to 2-agent cycles

**status:** decided
**what:** replace 7-agent cycles (3 check-in + 3 recommendation + 1 PM) with 2-agent cycles (1 analysis + 1 PM)
**rationale:** miyazaki (task granularity > agent count), MAST (eliminates ~28% failure surface), evolving orchestration (RL learns compaction), frontier model capability reduces multi-agent need
**details:**
- analysis agent: opus 4.6, exploratory, all timescales, fine-grained atomic sub-tasks, structured JSON output, zero config authority
- PM agent: opus 4.6, conservative, evaluative, receives structured analysis + beliefs, full config mutation authority
- check-in cycles: single sonnet 4.6 observation agent with structured sub-tasks

### 1.2 structured suggestion format

**status:** decided
**what:** replace free-text recommendations with JSON `{target_tool_id, param, current_value, proposed_value, confidence, evidence_summary}`
**rationale:** eliminates PM parsing errors, enables programmatic quorum detection
**blocked by:** new memo schema (add `suggestions` JSONB column to `agent_memos`)

### 1.3 fine-grained prompt engineering

**status:** todo (deep-dive session needed)
**what:** rewrite analysis agent prompt with numbered atomic sub-tasks, explicit output schemas, completion criteria
**research basis:** miyazaki (fine-grained decomposition), MAST FM-1.1 and FM-3.1 guards
**notes:** defer to dedicated prompt engineering session. key principles:
- each sub-task has defined input, output format, and scoring rubric
- completion checklist prevents premature termination
- explicit "if uncertain, state uncertainty with confidence interval" directive
- PM prompt requires explicit per-suggestion agree/disagree with reasoning

### 1.4 beliefs table (CVRF)

**status:** todo
**what:** add `beliefs` postgres table storing conceptualized investment beliefs from cross-episode comparison. inject into agent prompts as accumulated wisdom.
**research basis:** FinCon (NeurIPS 2024) — verbal reinforcement without retraining
**implementation:** see `docs/research_synthesis.md` section 2.1

### 1.5 emergency market trigger

**status:** todo
**what:** trigger unscheduled PM cycle on extreme market moves (>3% daily amplitude on SPY/QQQ, >7% 3-day cumulative)
**research basis:** HedgeAgents (WWW 2025) — emergency conference was highest-impact module

### 1.6 future experiment: parallel analysis agents

**status:** deferred
**what:** run 2-3 analysis agents with different analytical lenses (recent performance / regime detection / parameter calibration), aggregate via confidence-weighted voting before PM
**research basis:** debate vs vote (NeurIPS 2025) — independent ensembling beats debate
**prerequisite:** establish single-agent baseline first, then A/B test

---

## 2. execution engine: new indicators

### 2.1 OFI proxy (order flow imbalance) — priority: high

**what:** new indicator computing net buying/selling pressure from OHLCV candles
**formula:**
```
CLV = ((close - low) - (high - close)) / (high - low)
OFI = CLV * volume
score = (OFI / avg_volume_20).clamp(-1.0, 1.0)
```
**research basis:** cont, kukanov, stoikov (2014) — most validated microstructure predictor of short-term price moves. linear `delta_P = lambda * OFI` relationship robust across stocks and timescales. kolm et al. (2023) — OFI features significantly outperform raw price features regardless of model choice.
**timescale assignment:** 1-min (fast signal) and 5-min (more stable)
**why it matters:** your current 13 indicators are all price-based oscillators. zero volume-flow indicators. the microstructure literature consistently shows flow > price for short-horizon prediction. OFI fills the biggest gap in your indicator set.
**effort:** low — single indicator, ~100 lines rust, stateless (fits current replay pattern)

### 2.2 VPIN (volume-synchronized probability of informed trading) — priority: high

**what:** order flow toxicity measure predicting volatility events. operates on a volume clock (fixed-volume buckets rather than time bars).
**algorithm:**
1. bulk volume classification: `V_buy = V * Phi((close - open) / sigma)` where sigma = rolling 20-bar price change std dev
2. accumulate bars into volume buckets: `V_bucket = avg_daily_volume / 50`
3. `VPIN = (1/n) * sum(|V_buy - V_sell| / V_bucket)` over 50-bucket window
**output:** [0, 1]. normal: 0.2-0.5, elevated: >0.7, extreme: >0.9
**research basis:** easley, lopez de prado, o'hara (2012) — VPIN spiked before 2010 flash crash. predictive of liquidity-driven volatility events.
**timescale assignment:** operates on volume-time, not a traditional timescale. best used as a **hard gate** or risk modifier: suppress new entries when VPIN > 0.85.
**why it matters:** this is a fundamentally different signal from anything in your current set. your indicators measure momentum/trend; VPIN measures whether the market is safe to trade in. it catches "the market looks normal but informed traders are positioning" — invisible to RSI/MACD.
**effort:** medium — stateful indicator (needs ring buffer for buckets), ~200 lines rust. departs from stateless replay pattern — needs `VpinState` struct maintained across ticks.
**design consideration:** could be implemented as an action (hard gate check) rather than an indicator, since its role is binary gating rather than directional scoring.

### 2.3 positional context meta-indicators — priority: medium

**what:** expose current position state as indicator inputs to the scoring pipeline
**features:**

| feature | source | score mapping |
|---------|--------|---------------|
| position direction | `Position.direction` | -1.0 (short) / 0.0 (flat) / +1.0 (long) |
| unrealized PnL | `Position.unrealized_pnl_pct` | raw float, clamped |
| holding duration | `Position.hold_duration_ms / max_hold_ms` | 0.0 to 1.0 |
| session remaining | bars_left / total_session_bars | 1.0 (open) to 0.0 (close) |

**research basis:** goluza et al. (2024) — adding positional context to RL state space significantly improved intraday trading. each feature individually contributed via feature importance analysis.
**why it matters:** your exit actions currently evaluate position state, but your scoring pipeline doesn't see it. a trade that's +0.8% with 40 minutes held should score differently than a fresh entry at the same composite score. session_remaining is especially important — the scoring pipeline should naturally become more conservative as the session winds down.
**effort:** low — new "meta-indicator" type that reads from `Position` rather than `MarketState`. ~150 lines rust.
**assignment:** these don't belong to a timescale. they feed directly into the composite score or as modifiers to the entry/exit thresholds.

### 2.4 candlestick pattern classifier — priority: medium

**what:** classify each candle into one of 13 pattern types (based on body/wick ratios), then score 2-candle pattern pairs against a historical returns lookup table.
**research basis:** lin et al. (PRML, 2021) — ML-filtered 2-day patterns achieved 36.73% annual return, sharpe 0.81. but: marshall et al. (2006) showed raw patterns have no statistical significance in efficient markets. the edge comes from ML filtering, not the patterns themselves.
**implementation approach:**
```rust
fn classify_candle(o: f64, h: f64, l: f64, c: f64) -> u8 {
    let range = h - l;
    let body = (c - o).abs() / range;
    let upper_shadow = (h - c.max(o)) / range;
    let lower_shadow = (c.min(o) - l) / range;
    // map to 1 of 13 types based on body/shadow ratios
}
```
- track previous candle's type, form 2-candle pair ID (0-168)
- score from pre-computed lookup table (can be PM-tunable or learned from backtest data)
**timescale assignment:** 5-min (sweet spot between noise and signal per the research)
**effort:** medium — classifier is simple, but the lookup table needs historical calibration
**risk:** without the ML filtering step, this may add noise. consider implementing the classifier first and letting the PM agent observe correlation with trade outcomes before adding it to the scoring pipeline.

### 2.5 cross-asset OFI — priority: medium-low

**what:** use lagged OFI from SPY to predict constituent stock returns (and vice versa)
**formula:** `signal_i = alpha * self_OFI(t) + beta * SPY_OFI(t-1)`
**research basis:** cont, cucuringu, zhang (2023) — lagged cross-asset OFI significantly improves return forecasting at short horizons. strongest within ETF-constituent pairs.
**why it matters:** your universe is SPY, QQQ, AAPL, NVDA, MSFT — exactly the ETF+constituent structure where cross-impact is strongest. SPY flow leading AAPL by 1-5 bars is a documented signal.
**prerequisite:** OFI proxy (2.1) must be implemented first
**effort:** medium — requires cross-instrument state sharing in the engine

---

## 3. execution engine: scoring pipeline changes

### 3.1 dynamic timescale fusion gate — priority: high

**what:** replace static timescale weights with an adaptive gate driven by market regime
**current:** `composite = 0.20 * score_1m + 0.50 * score_5m + 0.30 * score_1h` (fixed weights)
**proposed:**
```rust
let volatility_score = atr_indicator.score;  // or rolling std dev
let trend_score = adx_indicator.score;       // or supertrend direction
let gate = sigmoid(volatility_score * w1 + trend_score * w2 + bias);
// gate ≈ 1.0 → trust fast timescales (volatile/choppy market)
// gate ≈ 0.0 → trust slow timescales (trending market)
let fast_weight = base_fast_weight + gate * adjustment;
let slow_weight = base_slow_weight - gate * adjustment;
```
**research basis:** CMLF (hou et al., CIKM 2021) — adaptive gate mechanism using market-aware technical indicators to blend timescale features outperforms static weights. the gate learns when to trust fast vs slow signals.
**PM-tunable knobs:** `w1`, `w2`, `bias`, `adjustment`, `use_dynamic_fusion` (boolean toggle)
**why it matters:** in trending markets, the hourly signal is the most valuable — fast noise is just noise. in choppy/volatile markets, fast signals matter more because the trend is unreliable. static 50% weight on 5-min is a compromise that's wrong in both regimes.
**effort:** medium — modification to `compute_composite()` in scoring.rs. the gate parameters become PM-tunable knobs.

### 3.2 cross-timescale agreement signal — priority: medium

**what:** add a meta-signal measuring whether fast and slow timescales agree
**formula:**
```rust
let agreement = 1.0 - (score_1min - score_1hr).abs();
// agreement ≈ 1.0 → timescales aligned, high confidence
// agreement ≈ 0.0 → timescales divergent, low confidence
```
**use as:**
- confidence multiplier on composite score: `adjusted_composite = composite * agreement.powf(0.5)`
- or additional hard gate: if `agreement < 0.3` then suppress entry
**research basis:** CMLF (contrastive learning maximizes mutual information between granularities), MSTNN (multi-scale ablation)
**why it matters:** your current hard gate only checks hourly > 0. it doesn't check whether 1-min and hourly are telling the same story. a trade where all timescales agree at +0.5 is much higher quality than one where 1-min = +0.9 and hourly = +0.1 (both above threshold, but divergent).
**effort:** low — ~20 lines in scoring.rs

### 3.3 volatility-scaled position sizing — priority: medium

**what:** new sizing action that scales position inversely with realized volatility
**formula:**
```rust
let sigma_ratio = current_atr / baseline_atr;  // baseline from 20-day average
let adjusted_fraction = base_fraction / sigma_ratio;
// high vol → smaller positions, low vol → larger positions
```
**research basis:** zhang, zohren, roberts (2020, oxford-man) — volatility-scaled rewards produced consistently better risk-adjusted returns across 50 futures contracts
**effort:** low — new action type (~80 lines), ATR already in indicator registry

---

## 4. execution engine: indicator configuration improvements

### 4.1 reduce redundancy in current config

**observation:** RSI appears on both 1-min (weight=0.30) and 5-min (weight=0.25) with identical period=14. MACD appears on both 1-min (weight=0.25) and 5-min (weight=0.25) with identical parameters. having the same indicator at the same period on adjacent timescales adds correlation without adding information.

**options:**
- **differentiate parameters:** RSI-7 on 1-min (faster, more responsive) and RSI-14 on 5-min (standard). MACD(6,13,5) on 1-min (faster) and MACD(12,26,9) on 5-min (standard).
- **replace duplicates:** swap one of the duplicate RSIs for OFI proxy. swap one of the duplicate MACDs for a regime indicator (ADX or frequency ratio).

**research basis:** miyazaki — fine-grained task decomposition matters. by analogy, fine-grained indicator differentiation (each indicator serving a distinct analytical purpose) matters more than adding more indicators of the same type.

### 4.2 add regime-detection indicator to hourly timescale

**observation:** the hourly timescale currently has VWAP distance, supertrend, EMA, and bollinger bandwidth. these are all trend/position indicators. none of them explicitly measure **whether the market is trending or mean-reverting** — they measure where price is relative to a trend, not trend quality.

**proposed:** add ADX to the hourly timescale. ADX is already implemented (composable/adx.rs) but not in the seed config. it explicitly measures trend strength (0-100), which is the missing dimension.

**why it matters:** your scoring pipeline applies the same indicator weights whether the market is trending or choppy. an ADX reading could:
- feed into the dynamic fusion gate (section 3.1)
- serve as an additional hard gate: if `ADX < 20` (no trend), suppress trend-following entries
- let the PM agent distinguish "the strategy works because the market is trending" from "the strategy works because these parameters are good"

### 4.3 add volume-flow representation to each timescale

**observation:** your 13 active indicators are 100% price-derived. zero use volume as a primary signal (MFI and OBV exist in the registry but aren't in the seed config). the microstructure papers consistently show volume-flow signals outperform price signals for short-horizon prediction.

**proposed timescale-aware volume indicators:**

| timescale | indicator | purpose |
|-----------|-----------|---------|
| 1-min | OFI proxy | fast directional flow pressure |
| 5-min | OFI proxy (more stable) | intermediate flow signal |
| 1-hr | VPIN (or simplified volume imbalance) | toxicity/regime gating |

this gives each timescale a volume-flow signal alongside its price-based signals, directly implementing the "heterogeneous features per timescale" pattern from the multi-granularity literature.

---

## 5. execution engine: future research items

these require more investigation or data pipeline changes before implementation.

### 5.1 information-driven bars

**what:** replace time-based candles with volume bars or tick imbalance bars for more uniform information content per bar
**from:** lopez de prado (2018)
**impact:** improves all downstream indicator computations by making bars more stationary
**prerequisite:** changes to `CandleAggregator` in data_feed crate; significant architectural change
**status:** defer to later phase

### 5.2 frequency decomposition indicator

**what:** compute DFT of recent returns, extract high-frequency vs low-frequency amplitude ratio as regime signal
**from:** SFM (KDD 2017)
**use:** feed into dynamic fusion gate as a regime indicator
**status:** interesting but low priority — ADX serves a similar purpose with less complexity

### 5.3 meta-labeling

**what:** two-model architecture where primary model (scoring pipeline) generates direction with high recall, secondary model filters for precision
**from:** lopez de prado (2018)
**impact:** cleanly separates "which direction?" from "should we trade?" — could dramatically improve win rate
**prerequisite:** requires an ML inference step in the tick loop (logistic regression or similar)
**status:** conceptually compelling but requires ML infrastructure in the rust engine

### 5.4 triple-barrier trade labeling for backtest

**what:** label each trade by which barrier was hit first (profit target, stop loss, time expiry) with ATR-scaled dynamic barriers
**from:** lopez de prado (2018)
**impact:** more informative backtest metrics for PM agent analysis
**note:** the engine already tracks exit reasons — this is more about enriching the backtest report with barrier-relative metrics

---

## 6. summary: what to do first

### immediate (before next walk-forward run)
1. implement OFI proxy indicator (2.1) — fills biggest gap in indicator set
2. restructure agent architecture to 2-agent cycles (1.1) — already decided
3. add structured suggestion format to memos (1.2)
4. differentiate duplicate indicator parameters (4.1)
5. add ADX to hourly timescale config (4.2)

### next iteration
6. implement VPIN indicator (2.2) — unique toxicity signal
7. implement dynamic timescale fusion gate (3.1)
8. add cross-timescale agreement signal (3.2)
9. add positional context meta-indicators (2.3)
10. implement beliefs table / CVRF (1.4)
11. add volatility-scaled sizing action (3.3)

### later
12. deep-dive prompt engineering session (1.3)
13. candlestick pattern classifier (2.4)
14. cross-asset OFI (2.5)
15. emergency market trigger (1.5)
16. parallel analysis agents experiment (1.6)
