# research synthesis: applicable learnings from 44 papers

**date:** 2026-03-02
**scope:** 10 multi-agent LLM trading papers + 34 intraday ML/microstructure papers
**lens:** what can be applied to the galactic trading firm's rust execution engine and typescript agentic loop

---

## part 1: execution engine improvements

### 1.1 new indicators from microstructure research

these are implementable with the current OHLCV data pipeline — no new data sources required.

#### OFI proxy (order flow imbalance) — priority: high

from cont, kukanov, stoikov (2014). the single most validated microstructure predictor of short-term price movement. with OHLCV data, approximate via close-location value:

```
CLV = ((close - low) - (high - close)) / (high - low)
OFI_proxy = CLV * volume
```

normalize to [-1, +1] by dividing by average volume. positive = net buying pressure, negative = net selling pressure. the linear relationship `delta_P = lambda * OFI` is robust across stocks and timescales.

**implementation:** new indicator in the registry, computed at 1-min and 5-min timescales. the 5-min OFI is more stable; the 1-min OFI captures faster signals.

#### VPIN (volume-synchronized probability of informed trading) — priority: high

from easley, lopez de prado, o'hara (2012). measures order flow toxicity — probability that informed traders are adversely selecting. famously spiked before the 2010 flash crash.

algorithm:
1. bulk volume classification: `V_buy = V * Phi((close - open) / sigma)` where `Phi` is the normal CDF and `sigma` is rolling std of price changes
2. accumulate bars into fixed-volume buckets (`V_bucket = avg_daily_volume / 50`)
3. `VPIN = (1/n) * sum |V_buy - V_sell| / V_bucket` over rolling window of 50 buckets

**key parameters:** 50 buckets/day, 50-bucket lookback window, 20-bar sigma window. output range [0, 1]. normal: 0.2-0.5, elevated: >0.7, extreme: >0.9.

**implementation:** this is a **stateful** indicator (maintains bucket accumulation across ticks), which is a departure from the current stateless-replay pattern. requires a `VpinState` struct with a ring buffer. could serve as a hard gate: if VPIN > 0.8, suppress new entries.

#### cross-asset OFI — priority: medium

from cont, cucuringu, zhang (2023). lagged OFI of one asset predicts returns of correlated assets. specifically: SPY OFI at time t-1 predicts constituent stock returns at time t, and vice versa.

```
signal_i = alpha * self_OFI_i(t) + beta * sum(lagged_OFI_j(t-1) * weight_ij)
```

**implementation:** compute OFI for each instrument. add a cross-asset indicator that uses SPY's lagged OFI as an input for QQQ and mega-cap scoring. weights can be PM-tunable knobs.

#### candlestick pattern classifier — priority: medium

from lin et al. (2021, PRML). classifying candles into 13 one-day pattern types based on body/wick ratios, then tracking 2-day pattern combinations (169 possibilities). ML-filtered top-10 patterns achieved 36.73% annual return with 0.81 sharpe.

```rust
fn classify_candle(open: f64, high: f64, low: f64, close: f64) -> u8 {
    // compute body_ratio = |close - open| / (high - low)
    // compute upper_shadow_ratio = (high - max(open, close)) / (high - low)
    // compute lower_shadow_ratio = (min(open, close) - low) / (high - low)
    // map to one of 13 pattern types
}
```

**implementation:** new indicator returning a pattern-match score. track 2-bar pattern pairs and score based on historical pattern returns (pre-computed lookup table, PM-tunable).

### 1.2 positional context features

from goluza et al. (2024). enriching the state space with the agent's current trading context significantly improves RL-based intraday trading. the `PositionManager` already tracks all of these — they just need to be exposed as indicator inputs.

| feature | source | normalization |
|---------|--------|---------------|
| position direction | `PositionManager` | -1.0 / 0.0 / +1.0 |
| unrealized PnL | (current - entry) / entry | raw float |
| holding bars | bars since entry / max_hold | 0.0 to 1.0 |
| session remaining | bars left / total session bars | 1.0 to 0.0 |

**implementation:** new "meta-indicator" category in the registry that reads from position state rather than market data. these feed into the scoring pipeline as additional signals — for example, weighting exit signals higher as `holding_bars` approaches max_hold.

### 1.3 dynamic timescale fusion

from CMLF (hou et al., CIKM 2021). the core finding: static timescale weights (your current PM-tunable weights) are suboptimal. an adaptive gate based on market regime should blend timescale scores dynamically:

```rust
let gate = sigmoid(volatility_score * w1 + trend_score * w2 + bias);
let composite = gate * fast_score + (1.0 - gate) * slow_score;
```

when volatility is high, the fast (1-min) timescale should dominate. when trends are strong and clear, the slow (hourly) timescale should dominate. this is implementable as a modification to `compute_composite()` in the scoring pipeline.

**PM-tunable knobs:** `w1`, `w2`, `bias` — the gate parameters. additionally, a boolean `use_dynamic_fusion` to toggle between static and dynamic modes.

### 1.4 volatility-scaled position sizing

from zhang, zohren, roberts (2020, oxford-man). scale position sizes inversely with realized volatility:

```
position_size = base_size / (sigma_t / sigma_baseline)
```

where `sigma_t` is current rolling ATR or return std dev, and `sigma_baseline` is a long-run average. this naturally reduces exposure during volatile periods and increases it during calm markets.

**implementation:** modify the `FixedFractionalSizing` action or add a new `VolatilityScaledSizing` action. the ATR indicator is already in the registry.

### 1.5 frequency decomposition as indicator

from SFM (zhang, aggarwal, qi, KDD 2017). compute the DFT of recent price returns and extract amplitude at K frequency components:

```
returns = [r_1, r_2, ..., r_N]  // last N bar returns
fft_result = DFT(returns)
high_freq_amplitude = sum(|fft[k]| for k in top_quartile)
low_freq_amplitude = sum(|fft[k]| for k in bottom_quartile)
freq_ratio = high_freq_amplitude / low_freq_amplitude
```

high `freq_ratio` = choppy/noisy market (short-term patterns dominate). low `freq_ratio` = trending market (long-term patterns dominate). useful as a regime indicator feeding into the dynamic fusion gate.

### 1.6 information-driven bars (future consideration)

from lopez de prado (2018). replace time-based candles with volume bars, dollar bars, or tick imbalance bars. these have more uniform information content per bar, which improves all downstream indicator computations.

**volume bars:** sample when cumulative volume reaches `V_bar = avg_daily_volume / expected_bars_per_day`.

**tick imbalance bars:** sample when cumulative signed tick imbalance exceeds a dynamic threshold based on EWMA of recent bar sizes.

**implementation complexity:** medium-high. requires changes to `CandleAggregator` in the data_feed crate. the benefit is more stationary indicator inputs and better-calibrated thresholds. consider as a phase 8+ addition.

### 1.7 triple-barrier labeling for backtest evaluation

from lopez de prado (2018). instead of binary win/loss, label each trade by which of three barriers was hit first: profit target, stop loss, or time expiry.

```
upper_barrier = entry * (1 + pt)      // profit-taking
lower_barrier = entry * (1 - sl)      // stop-loss
vertical_barrier = entry_time + max_hold
```

barrier widths should be dynamic: `pt = multiplier * per_bar_volatility`. this labeling is more informative than simple P&L for the PM agent's analysis — it distinguishes between trades that hit targets vs those that timed out.

**implementation:** the engine already tracks exit reasons (trailing stop, session close, max hold timeout, etc.). this is more about enriching the backtest report with barrier-relative metrics that the PM agent can reason about.

### 1.8 meta-labeling (future consideration)

from lopez de prado (2018). a two-model architecture: the primary model (your scoring pipeline) generates directional signals with high recall, and a secondary "meta-model" filters those signals for precision:

```
primary_signal = scoring_pipeline(indicators)   // direction
meta_probability = meta_model(signal, features) // confidence [0, 1]
position_size = meta_probability * max_size
```

this cleanly separates "which direction?" from "should we actually trade?" and maps naturally to the scoring pipeline (direction) + action system (sizing). the meta-model could be a simple logistic regression trained on historical trade features.

---

## part 2: agentic loop improvements

### 2.1 conceptual verbal reinforcement (CVRF)

from FinCon (yu et al., NeurIPS 2024). the most sophisticated learning-without-retraining mechanism in the literature. the algorithm:

1. compare PnL of two adjacent config versions (episodes)
2. identify which indicator/action changes contributed to better/worse outcomes
3. generate conceptualized belief updates attributing gains/losses to specific parameters
4. store these beliefs and inject them into future agent prompts

**implementation:**

add a `beliefs` table to postgres:
```sql
CREATE TABLE beliefs (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ DEFAULT now(),
    source_config_old INTEGER REFERENCES config_versions(id),
    source_config_new INTEGER REFERENCES config_versions(id),
    belief_text TEXT NOT NULL,          -- natural language belief
    confidence FLOAT NOT NULL,          -- 0.0 to 1.0
    category TEXT NOT NULL,             -- 'indicator', 'action', 'threshold', 'regime'
    still_valid BOOLEAN DEFAULT true    -- PM can invalidate
);
```

after each config promotion cycle, the PM agent compares old vs new performance and writes belief entries. these get injected into recommendation agent prompts as context:

```
## accumulated beliefs (from past config experiments)
- "raising entry_threshold from 0.45 to 0.50 improved win rate by 8% but reduced trade count by 30%" (confidence: 0.8)
- "trailing stop ATR multiplier below 2.0 causes excessive whipsaws in choppy SPY sessions" (confidence: 0.7)
```

this directly addresses the "no feedback loop on past config changes" issue from `agentic_system_recommendations.md`.

### 2.2 emergency market conference trigger

from HedgeAgents (li et al., WWW 2025). define extreme market thresholds that trigger an unscheduled PM cycle:

| trigger | threshold | action |
|---------|-----------|--------|
| single-day amplitude | > 3% (SPY/QQQ) | emergency PM cycle |
| 3-day cumulative amplitude | > 7% | emergency PM cycle |
| VPIN spike | > 0.85 | tighten entry threshold |
| consecutive losing trades | > 5 in a row | emergency check-in |

**implementation:** add a `MarketEventMonitor` to the orchestrator that polls the data_feed for extreme conditions. when triggered, invoke `runFullPmCycle()` regardless of schedule. the existing budget enforcement ensures this stays within the $5/day cap.

note: HedgeAgents used 5%/10% thresholds for multi-asset (crypto+stocks+forex). for pure equity intraday, lower thresholds (3%/7%) are more appropriate since SPY rarely moves 5% in a day.

### 2.3 fine-grained task decomposition for agent prompts

from miyazaki et al. (2026). the key finding: **specificity of task instructions matters more than number of agents.** change recommendation agent prompts from vague mandates to atomic sub-tasks:

**before (current approach):**
```
analyze the current config and suggest parameter changes for the 5-minute timescale
```

**after (fine-grained decomposition):**
```
complete these analysis tasks in order:

1. compute 5-day rolling win rate per ticker → report as float per ticker
2. for each active 5-min indicator, compute its contribution to winning vs losing trades:
   - avg score on winners, avg score on losers, delta → report as table
3. identify the single indicator with the highest false-signal rate
   (positive score on losing trades) → report indicator name + false rate
4. evaluate whether current entry_threshold produces balanced win/loss ratios
   → report current WR and suggest specific threshold adjustment if WR < 45% or > 70%
5. propose exactly ONE parameter change with:
   - target_tool_id, param_name, current_value, proposed_value
   - expected impact estimate (e.g., "+3% win rate, -15% trade count")
   - supporting evidence from tasks 1-4
```

this directly mitigates MAST failure modes FM-1.1 (disobey task specification) and FM-3.1 (premature termination) by providing explicit completion criteria.

### 2.4 voting over debate for parameter recommendations

from choi, zhu, li (NeurIPS 2025). the theoretical result: multi-agent debate is a **martingale** — it neither helps nor hurts in expectation. majority voting alone accounts for most performance gains attributed to debate.

**implications for the current 3-recommendation-agent design:**

1. **run agents independently, not sequentially.** recommendation agents should not see each other's memos before generating. this prevents herding and maximizes diversity.

2. **diversify analytical lenses.** make each agent genuinely different:
   - agent_1min: focus on last-24h trade performance, short-term score calibration
   - agent_5min: focus on regime/volatility changes over last 5 days
   - agent_hourly: focus on parameter drift and long-term indicator calibration

3. **aggregate via confidence-weighted median.** each agent outputs a parameter change + confidence score. the PM uses the confidence-weighted median, not a synthesized debate.

4. **invest in agent diversity, not agent count.** three diverse agents voting beats five homogeneous agents debating.

### 2.5 quorum-based early termination

from Aegean (ruan et al., 2025). run recommendation agents concurrently and define a quorum:

- if 2 of 3 agents recommend the same parameter change (same parameter, same direction, magnitude within 15%), **commit immediately** and cancel the third agent
- if no quorum after all 3, the PM agent adjudicates
- use **stability horizon = 1** for time-sensitive daily decisions

**estimated savings:** 20-30% average token cost per cycle, from canceling the third agent when quorum is reached early.

**implementation:** modify `runFullPmCycle()` to launch recommendation agents concurrently (Promise.all with AbortController), check for quorum after each completion, and abort remaining agents on consensus.

### 2.6 adaptive orchestration

from evolving orchestration (dang et al., NeurIPS 2025). the key finding: a trained orchestrator learns to **use fewer agents when possible** and **cycle back for verification when uncertain.** two emergent patterns:

1. **compaction:** if yesterday's sharpe > 2.0 and no regime change detected, skip recommendation agents entirely — only run check-ins. saves tokens on stable days.

2. **escalation:** if check-in memos flag degrading signal quality or anomalous market conditions, escalate to a full PM cycle immediately rather than waiting for the scheduled time.

3. **cyclic verification:** after the PM proposes a config change, have one recommendation agent review the proposal before promotion. this adds a verification cycle that catches errors.

**implementation:** add orchestration logic to the scheduler:
```typescript
const shouldRunFullCycle = await evaluateCycleNeed(pool, tradingDate);
// returns: 'skip' | 'standard' | 'urgent'
// based on: recent sharpe, check-in memo flags, market conditions
```

### 2.7 MAST failure mode guards

from cemri et al. (NeurIPS 2025). the top failure modes and their mitigations:

| failure mode | frequency | guard |
|---|---|---|
| disobey task specification (15.7%) | enforce JSON output schema with zod validation; reject and re-prompt on schema violation |
| step repetition (13.2%) | hash each memo's key recommendations; reject if >80% overlap with previous cycle's memo for same agent |
| conversation reset (12.4%) | persist agent state in postgres between invocations, inject previous memo summary into prompt |
| ignored other agent's input (9.1%) | PM prompt must explicitly reference each recommendation agent's memo by name and state agree/disagree |
| information withholding (6.2%) | require structured output with all computed metrics, not just conclusions |
| premature termination (11.8%) | define explicit completion checklist in each prompt; validate all tasks completed before accepting output |

**most critical for your system:** the "ignored other agent's input" failure (9.1%) maps directly to the agentic_system_recommendations finding that the PM sometimes ignores recommendation agent disagreements. the fix: force the PM prompt to enumerate each memo and explicitly address it.

### 2.8 structured suggestions in memos

from the agentic_system_recommendations doc + QuantAgent's structured output. replace free-text suggestions with structured JSON:

```json
{
  "analysis_tasks_completed": ["win_rate_by_ticker", "indicator_contribution", "threshold_evaluation"],
  "suggestions": [
    {
      "target_tool_id": "trailing_stop_atr",
      "param": "multiplier",
      "current_value": 2.0,
      "proposed_value": 2.5,
      "direction": "increase",
      "confidence": 0.78,
      "expected_impact": "+8% win rate on trailing stop exits",
      "evidence_summary": "21% WR on 24 trailing stop exits vs 64% for other exits; ATR multiplier too tight for current SPY volatility regime"
    }
  ]
}
```

this eliminates the PM's need to parse prose for parameter changes and enables programmatic quorum detection (section 2.5).

### 2.9 three-tier memory architecture

from FinCon and HedgeAgents. both papers use explicit multi-tier memory. mapping to your system:

| tier | purpose | implementation |
|------|---------|---------------|
| **working** | current market state, recent scores | already in engine (MarketState, TickResult) |
| **procedural** | trade history, config changelog | already in postgres (trades, config_versions, config_changelog) |
| **episodic** | investment beliefs, regime observations | **new**: beliefs table (section 2.1) + enriched agent_memos |
| **experiential** | cross-agent shared knowledge pool | **new**: get_shared_insights tool that surfaces key findings from all agents |

the gap is in the episodic layer. the current system writes memos but doesn't systematically accumulate beliefs across cycles. CVRF (section 2.1) fills this gap.

### 2.10 budget reallocation across instruments

from HedgeAgents' budget allocation conference. the 30-day BAC cycle was the single most impactful module — removing it dropped annualized returns by 37-44%.

**implementation:** add a monthly PM cycle (separate from daily) that reviews per-instrument performance and adjusts:
- position sizing weights across SPY, QQQ, and mega-caps
- which instruments are active in the config
- per-instrument entry/exit thresholds

this maps to a new config section: `instrument_weights: { "SPY": 0.4, "QQQ": 0.3, "AAPL": 0.1, ... }` that the PM can mutate.

---

## part 3: engine configuration insights

### 3.1 concrete parameters from the literature

| parameter | recommended value | source | notes |
|-----------|------------------|--------|-------|
| default lookback window | 60 bars | zhang et al. (oxford-man) | for composable indicators on 1-min data |
| entry signal threshold | ~20 bps (0.002) | DeepLOB | minimum profitable signal after costs |
| transaction cost assumption | 0.10-0.25% roundtrip | DeepScalper, multiple | for backtest realism |
| ATR trailing stop multiplier | 2.0-3.0 | multiple | 2.5 already validated by your post-analysis |
| VPIN toxicity threshold | 0.85 | easley et al. | suppress entries above this |
| OFI normalization | divide by avg volume | cont et al. | for cross-asset comparability |
| candlestick pattern window | 2 bars | lin et al. (PRML) | 2-day patterns strongest signal |
| dynamic fusion gate bias | -0.5 to 0.5 | CMLF | start at 0 (balanced), let PM tune |

### 3.2 multi-timescale scoring improvements

the literature strongly validates your multi-timescale approach (MSTNN ablation: removing multi-scale drops F1 by 27-34%). specific refinements:

1. **inception-style multi-resolution:** compute the same indicator at 1-bar, 3-bar, and 5-bar resolutions within each timescale, then fuse (from DeepLOB's inception module). for example, RSI-14 on 1-min candles alongside RSI-14 on 3-min and 5-min candles, even within the "1-min timescale."

2. **cross-granularity agreement as signal quality:** from CMLF's contrastive learning. when 1-min and hourly indicators agree, signal quality is high — increase confidence. when they disagree, reduce entry sizing or raise the threshold. this could be an additional hard gate: `if abs(score_1min - score_hourly) > 0.5 then suppress_entry`.

3. **frequency ratio as regime detector:** from SFM. compute DFT of 60-bar returns. high-frequency dominance = choppy market (favor mean-reversion), low-frequency dominance = trending market (favor trend-following). feed this ratio into the dynamic fusion gate.

### 3.3 the universality finding

from sirignano, cont (2019). a single model trained on multiple stocks outperforms stock-specific models. this validates your approach of using **the same config across all instruments** rather than per-instrument configs. the PM agent should be cautious about per-instrument overrides — they're more likely to overfit than to capture genuine cross-sectional differences.

exception: SPY/QQQ (ETFs) vs individual mega-caps may have genuinely different microstructure. the cross-asset OFI indicator (section 1.3) can capture this difference without separate configs.

### 3.4 the market efficiency warning

from byrd, balch (2019). simple ML predictive models on standard features lost profitability after ~2009 as markets became more efficient. implications:

1. **monitor predictive decay.** track rolling sharpe ratio and the correlation between composite scores and subsequent trade outcomes. if the correlation drops below a threshold, the PM should flag a regime change.

2. **feature innovation matters more than model complexity.** OFI, VPIN, and cross-asset signals are more likely to provide durable alpha than more sophisticated use of the same OHLCV indicators everyone else uses.

3. **the agentic loop is the moat.** the ability to continuously adapt config parameters via the PM agent is this system's key advantage over static strategies. invest in making the evolution loop faster and more reliable.

---

## part 4: prioritized implementation roadmap

### tier 1 — high impact, low-medium effort

| item | category | effort | impact | section |
|------|----------|--------|--------|---------|
| OFI proxy indicator | engine | low | high | 1.1 |
| fine-grained prompt decomposition | agents | low | high | 2.3 |
| structured suggestion format | agents | low | high | 2.8 |
| MAST failure mode guards | agents | low | high | 2.7 |
| run recommendation agents concurrently | agents | medium | medium | 2.5 |
| positional context meta-indicators | engine | low | medium | 1.2 |

### tier 2 — high impact, medium effort

| item | category | effort | impact | section |
|------|----------|--------|--------|---------|
| VPIN indicator | engine | medium | high | 1.1 |
| beliefs table + CVRF | agents | medium | high | 2.1 |
| dynamic timescale fusion gate | engine | medium | high | 1.3 |
| emergency market conference trigger | agents | medium | medium | 2.2 |
| volatility-scaled position sizing | engine | low | medium | 1.4 |
| candlestick pattern classifier | engine | medium | medium | 1.1 |

### tier 3 — medium impact, medium-high effort

| item | category | effort | impact | section |
|------|----------|--------|--------|---------|
| cross-asset OFI | engine | medium | medium | 1.1 |
| adaptive orchestration | agents | high | medium | 2.6 |
| monthly instrument reallocation cycle | agents | medium | medium | 2.10 |
| frequency decomposition indicator | engine | medium | low-medium | 1.5 |
| inception multi-resolution indicators | engine | medium | medium | 3.2 |

### tier 4 — future consideration

| item | category | effort | impact | section |
|------|----------|--------|--------|---------|
| information-driven bars | engine | high | medium | 1.6 |
| meta-labeling | engine | high | high | 1.8 |
| quorum early termination | agents | medium | low | 2.5 |
| triple-barrier labeling for backtest | engine | medium | medium | 1.7 |

---

## appendix: paper reference index

### multi-agent papers
1. QuantAgent (xiong et al., 2025) — price-only multi-agent HFT
2. TradingAgents (xiao et al., 2024) — bull-bear debate, 7-role architecture
3. FinCon (yu et al., NeurIPS 2024) — verbal reinforcement learning
4. HedgeAgents (li et al., WWW 2025) — 3 conference types, emergency response
5. ElliottAgents (wawer, chudziak, 2025) — wave patterns + DRL validation
6. fine-grained decomposition (miyazaki et al., 2026) — task granularity > agent count
7. MAST failure taxonomy (cemri et al., NeurIPS 2025) — 14 failure modes, 1600+ traces
8. evolving orchestration (dang et al., NeurIPS 2025) — RL-trained puppeteer
9. Aegean (ruan et al., 2025) — distributed consensus for LLM agents
10. debate vs vote (choi et al., NeurIPS 2025) — martingale theorem, voting wins

### intraday ML papers
1-8. gu et al. (2020), lopez de prado (2018), huddleston et al. (2023), liu & stentoft (2023), DeepScalper (sun et al., 2022), zhang et al. (2020), byrd & balch (2019), goluza et al. (2024)

### candlestick papers
9-16. marshall et al. (2006), chen et al. (2016), tharavanij et al. (2017), cohen et al. (2019), chen & tsai (2020), lin et al. (2021), brim & flann (2022), chen & tsai (2022)

### multi-timeframe papers
17-24. gencay et al. (2001), SFM (zhang et al., KDD 2017), ding et al. (IJCAI 2020), MTDNN (liu et al., IJCAI 2020), CMLF (hou et al., CIKM 2021), HATR (wang et al., IJCAI 2021), chen et al. (2024), MSTNN (sun et al., IJCAI 2025)

### microstructure papers
25-34. kyle (1985), glosten & milgrom (1985), almgren & chriss (2001), VPIN (easley et al., 2012), OFI (cont et al., 2014), hawkes (bacry et al., 2015), DeepLOB (zhang et al., 2019), sirignano & cont (2019), deep OFI (kolm et al., 2023), cross-impact OFI (cont et al., 2023)
