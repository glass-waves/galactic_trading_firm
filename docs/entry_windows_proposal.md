# entry windows: condition-based entry architecture proposal

## 1. problem: weighted-sum dilution

analysis of 1,847 trades across 4 years (2022–2025) reveals the current weighted-sum scoring architecture systematically dilutes indicator signals.

### the numbers

| composite range | trades | share | avg P&L/trade |
|----------------|--------|-------|---------------|
| [0.40, 0.45) | 1,445 | 78.2% | $0.60 |
| [0.45, 0.50) | 234 | 12.7% | $1.42 |
| [0.50, 0.55) | 87 | 4.7% | $1.58 |
| [0.55, 0.60) | 41 | 2.2% | $2.31 |
| [0.60+) | 40 | 2.2% | $0.28 |

- **median entry composite: 0.415** — barely above the 0.40 threshold
- **78% of entries** cluster in the [0.40, 0.45) band
- higher conviction entries are monotonically more profitable ($0.60 → $2.31/trade)
- the pre-bug-fix claim that "composites are typically 0.55+" was empirically false

### the 1-minute noise problem

| entry type | trades | share | avg P&L | win rate |
|-----------|--------|-------|---------|----------|
| 1m-led, 5m weak (<0.40) | 946 | 51% | $0.43 | 52.9% |
| 5m-led (5m >= 0.50) | 201 | 11% | $1.41 | 59.2% |
| aligned (all within 0.15) | 314 | 17% | $1.05 | 65.0% |

**51% of all trades** are entries where the 1-minute timescale (noisy, 10% weight) is the strongest signal while the 5-minute core signal is weak. these average $0.43/trade — **3.3x worse** than when the 5m genuinely leads ($1.41).

### per-ticker impact

| ticker | 1m-led-weak share | avg P&L (1m-led) | overall avg P&L |
|--------|-------------------|------------------|-----------------|
| NVDA | 7% | $0.89 | $1.41 |
| MSFT | 15% | $0.15 | $0.59 |
| QQQ | 22% | $0.27 | $0.37 |
| SPY | 25% | $0.15 | $0.32 |
| AAPL | 35% | $0.42 | $0.72 |

NVDA dominates P&L (51% of total) partly because it naturally has the fewest noise-led entries.

### root cause

the weighted-sum makes every indicator contribute to every entry. four dilution mechanisms:

1. **threshold clustering**: 78% of entries at [0.40, 0.45) — decisions made on weak signals
2. **noise dominance**: 1m variance pushes borderline 5m signals over the edge
3. **invisible agreement**: two market states with identical composites (0.46) can have 65% vs 53% win rates depending on timescale alignment
4. **no entry regime concept**: breakouts, trend continuations, and bounces all produce the same blended number

---

## 2. entry windows concept

replace the single composite threshold with multiple named **entry windows** — each a set of conditions that must ALL be true for entry.

```
entry = OR(
  window_1: condition_A AND condition_B AND condition_C,
  window_2: condition_D AND condition_E,
  window_3: condition_F AND condition_G AND condition_H,
  ...
)
```

each window defines:
- **name**: human-readable identifier (e.g., "5m_thrust", "aligned_bias")
- **conditions**: AND-logic requirements on timescale scores, individual indicator scores, or market state
- **direction**: long, short, or either
- **priority**: when multiple windows fire, higher priority wins
- **enabled**: toggle for A/B testing

entry fires when **any** enabled window has **all** its conditions met. the tick loop already iterates entry actions — first match wins.

### what changes vs current system

| aspect | current | entry windows |
|--------|---------|---------------|
| entry decision | composite >= 0.40 | any window's conditions met |
| indicator role | all blend into one number | specific indicators matter per window |
| regime awareness | none | each window IS a regime |
| testability | change weights, observe composite | enable/disable individual windows |
| agent evolution | tune weights continuously | add/remove/modify discrete windows |

### what stays the same

indicators still compute scores. exits still use composite (ScoreExit). sizing still chains after entry. position management unchanged. backtest infrastructure unchanged. paper trader hot-reload unchanged.

---

## 3. architecture: zero breaking changes

the existing system is perfectly modular. the entry mechanism is abstracted behind the `Action` trait:

```rust
// crates/types/src/action.rs
trait Action {
    fn evaluate(
        &self,
        position: Option<&Position>,
        market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal;
}
```

a new `EntryWindowAction` implements this trait. the tick loop (`crates/engine/src/tick_loop.rs` lines 200-251) iterates entry actions generically — it doesn't know or care what type of entry action it's calling.

### component impact matrix

| component | file | change needed |
|-----------|------|---------------|
| tick loop | `crates/engine/src/tick_loop.rs` | none |
| Action trait | `crates/types/src/action.rs` | see "indicator access" below |
| indicator computation | `crates/indicators/` | none |
| scoring pipeline | `crates/engine/src/scoring.rs` | none (still runs for exits) |
| exit logic | `crates/actions/src/exit/` | none |
| sizing actions | `crates/actions/src/sizing/` | none |
| position manager | `crates/engine/src/position.rs` | none |
| backtest replay | `crates/backtest/src/replay.rs` | none |
| backtest CLI | `crates/backtest/src/main.rs` | add CLI flags for window testing |
| paper trader | `crates/data_feed/src/` | none (hot-reload automatic) |
| agent layer | `agents-ts/` | none (config is JSONB) |
| config system | `crates/types/src/config.rs` | none (uses ActionConfig params) |

### implementation surface

| file | change | estimated lines |
|------|--------|-----------------|
| `crates/actions/src/entry/entry_window.rs` | **new** — window evaluation logic | ~300-400 |
| `crates/actions/src/entry/mod.rs` | add `pub mod entry_window;` | 1 |
| `crates/actions/src/lib.rs` | register factory | 1 |
| `crates/actions/tests/` | window action tests | ~200 |
| **total** | | **~500-600** |

### indicator access: recommended trait extension

entry windows need to check individual indicator scores (e.g., "MACD_5min > 0.3"), not just aggregated timescale scores. two options:

**option A — no trait change (limited):** windows can only check `TimescaleScores` fields (one_minute, five_minute, one_hour) and `MarketState` (raw candles, VWAP). cannot reference individual indicators by name.

**option B — expose indicator outputs (recommended):** add `indicator_outputs` to the evaluate signature:

```rust
fn evaluate(
    &self,
    position: Option<&Position>,
    market: &MarketState,
    scores: &TimescaleScores,
    indicator_outputs: Option<&HashMap<String, Option<f64>>>,  // NEW
) -> ActionSignal;
```

all existing actions add `_indicator_outputs: Option<&...>` and ignore it. entry windows use it to check specific indicators. this enables conditions like:

```
window "macd_breakout":
  macd_5min >= 0.30
  AND ema_5min >= 0.20
  AND supertrend_1hr >= 0.0
  AND adx_1hr >= 0.15
```

**recommendation:** option B. the whole point of entry windows is indicator-level interplay. the trait change is trivial — one new parameter on ~8 existing implementations.

the tick loop already has `indicator_outputs` available (computed at line 110-126 in `tick_loop.rs`) — it just needs to pass them through to actions.

---

## 4. data-driven initial windows

### natural entry regimes (from 4-year backtest analysis)

clustering trades by timescale score patterns reveals 5 natural regimes:

| regime | condition | trades | win% | avg P&L | quality |
|--------|-----------|--------|------|---------|---------|
| **5m thrust** | 5m leads all by 0.15+, 5m >= 0.50 | 76 | 78.9% | $2.62 | excellent |
| **aligned** | all timescales within 0.15 spread | 314 | 65.0% | $1.05 | good |
| **1h trend** | 1h leads all by 0.15+, 1h >= 0.40 | 215 | 62.3% | $1.25 | good |
| **mixed** | no clear dominance | 1,047 | 52.6% | $0.55 | mediocre |
| **1m noise** | 1m leads all by 0.15+, 5m < 0.35 | 195 | 51.3% | $0.36 | poor |

### score combination patterns (best trades)

the top quartile by P&L (461 trades, avg $4.74/trade) shows:

| pattern (1m / 5m / 1h) | trades | win% | avg P&L | interpretation |
|------------------------|--------|------|---------|----------------|
| weak / strong / moderate | 33 | 69.7% | $2.78 | **mean-reversion within trend** |
| weak / weak / strong | 321 | 64.5% | $1.12 | hourly trend carrying |
| strong / strong / strong | 38 | 65.8% | $1.86 | full alignment (rare) |
| weak / strong / weak | 48 | 60.4% | $1.09 | gap-fill revert |
| strong / weak / strong | 464 | 55.6% | $0.69 | noisy, mediocre |
| strong / weak / weak | 174 | 44.8% | $0.24 | **1m exhaustion — avoid** |

### exit reason distribution by regime

| regime | ScoreExit win% | SessionClose win% | Timeout win% |
|--------|----------------|--------------------| -------------|
| aligned | 45.3% | 81.0% | 86.7% |
| 5m thrust | 74.4% | 91.3% | 77.8% |
| 1m noise | 37.1% | 76.5% | 75.0% |

1m-noise entries **fail badly on ScoreExit** (37% win rate) but are saved by session close / timeout. this is dead-money behavior — the entry was bad but the hold period bailed it out.

### per-ticker windows

NVDA-specific patterns (523 trades, $1.41 avg):

| NVDA pattern | trades | win% | avg P&L |
|-------------|--------|------|---------|
| 5m-led | 45 | 82.2% | $3.46 |
| 1h-led | 112 | 67.0% | $1.72 |
| aligned | 73 | 69.9% | $1.70 |
| 1m-led (weak 1h) | 7 | 57.1% | $0.89 |

NVDA's 5m-led pattern: **82.2% win rate, $3.46/trade**. this suggests per-ticker entry windows could be valuable.

---

## 5. proposed initial window set

### window 1: "5m thrust"

the strongest pattern. 5-minute timescale dominates with clear lead over other timescales.

```
conditions:
  5m_score > 1m_score + 0.15
  5m_score > 1h_score + 0.15
  5m_score >= 0.50
  1h_score > 0.0          # hourly not bearish (basic safety)
```

expected: ~76 trades/4yr, 79% win rate, $2.62 avg P&L

### window 2: "aligned bias"

all timescales agree AND momentum is accelerating. the momentum gate filters out "aligned but flat" entries — without it, ScoreExit on aligned entries has only 45.3% win rate ($0.13/trade) vs SessionClose at 81% ($1.64/trade). the flat entries revert quickly and get caught by score-exit.

```
conditions:
  max(1m, 5m, 1h) - min(1m, 5m, 1h) < 0.15
  min(1m, 5m, 1h) > 0.10
  momentum_persistence_5min > 0.0    # acceleration, not equilibrium
```

expected: subset of ~314 trades/4yr with improved ScoreExit win rate

### window 3: "hourly trend"

hourly timescale provides strong directional bias. ideally captures pullback-into-trend entries (Elder's Triple Screen concept). the 5m condition should eventually become a range filter (`timescale_range`) to capture "5m weak but not reversing" — exact thresholds need empirical validation post-implementation since current data is biased by the composite threshold gate.

```
conditions:
  1h_score > 1m_score + 0.15
  1h_score > 5m_score + 0.15
  1h_score >= 0.40
  5m_score > 0.0          # 5m not bearish (upgrade to range filter after validation)
```

expected: ~215 trades/4yr, 62% win rate, $1.25 avg P&L

### window 4: "strong core"

high conviction regardless of pattern — both core timescales strong. 49% overlap with windows 1-3, but captures 64 unique trades at $1.23/trade (53.1% win rate). candidate for removal if empirically redundant after initial validation.

```
conditions:
  5m_score >= 0.50
  1h_score >= 0.30
```

expected: ~126 trades/4yr (64 unique beyond W1-W3)

### window 5: "engulfing reversal" (candle pattern integration)

integrates the candle pattern indicator as a first-class entry signal rather than a 0.05-weight composite contributor. gated on low trend strength — mean-reversion patterns in trending conditions get run over. this makes the window explicitly a "mean-reversion in range-bound conditions" setup.

```
conditions:
  candle_5min_score > 0.30
  5m_score > 0.20
  1h_score > 0.0
  adx_1hr < 0.25            # range-bound regime (trend not dominant)
```

the candle pattern indicator already computes confluence (volume, VWAP location, trend) internally, so the score threshold captures pattern quality. the ADX gate ensures the pattern fires in conditions where mean-reversion is viable.

### window 6: "OFI surge" (research — disabled initially)

OFI (order flow imbalance) at 0.05 weight in the composite is effectively invisible. as its own window, OFI becomes the primary signal — it's orthogonal to price-based indicators and explains ~65% of contemporaneous price variation at short horizons (Cont et al. 2014). start disabled, validate independently.

```
conditions:
  ofi_5min >= 0.50           # strong order flow imbalance
  macd_5min >= 0.20          # trend direction confirms
  1h_score >= 0.0            # hourly not bearish
enabled: false               # research window — validate before enabling
```

### window 7: "VWAP reclaim" (research — disabled initially)

price reclaiming session VWAP signals institutional-driven directional moves — a distinct regime from momentum or alignment windows. uses the existing `vwap_distance` 1h indicator. start disabled, validate independently.

```
conditions:
  vwap_distance_1hr >= 0.30  # price meaningfully above VWAP
  5m_score >= 0.30           # 5m confirms direction
  adx_1hr >= 0.15            # some trend strength present
enabled: false               # research window — validate before enabling
```

### reject gate: "1m noise filter"

block entry when 1m noise dominates.

```
reject if:
  1m_score > 5m_score + 0.15
  1m_score > 1h_score + 0.15
  5m_score < 0.35
```

this removes ~195 trades/4yr averaging $0.36/trade — $70 total P&L sacrificed, ~$50 in transaction costs saved.

---

## 6. generating new windows

### from existing data

the 4-year trade CSVs (`data/backtest_202{2,3,4,5}_trades.csv`) contain per-trade entry scores:

```
columns: entry_composite, exit_composite, entry_1m, entry_5m, entry_1h, exit_1m, exit_5m, exit_1h
```

to generate new window candidates:
1. filter trades by P&L quartile (top 25%)
2. cluster by timescale score patterns
3. identify discriminating score ranges (what separates winners from losers in each cluster)
4. express as entry window conditions
5. backtest the window in isolation to measure PF, win rate, trade count

### from individual indicators

run backtest with `--verbose` to capture per-indicator scores, then:
1. for each indicator pair (e.g., MACD_5m × StochRSI_5m), compute P&L heatmap
2. identify high-win-rate regions in the 2D space
3. express as window conditions (e.g., "MACD > 0.3 AND StochRSI > 0.2")

this requires the option B trait change (indicator access in actions).

### from market microstructure

windows based on `MarketState` fields (not just scores):
- **volume breakout**: relative_volume > 2.0 AND 5m_score > 0.30
- **VWAP reclaim**: price crosses above session VWAP AND 1h_score > 0.20
- **narrow spread**: bid_ask_spread < threshold AND 5m_score > 0.40

these access raw market data through `MarketState`, which is already passed to `Action::evaluate()`.

---

## 7. testing strategy

### phase 1: all windows together

configure all 5 windows + reject gate. run 4-year backtest. measure:
- total trades, total P&L, PF, win rate
- per-window breakdown: which window triggered each trade
- window overlap: how often do multiple windows fire on the same tick

this tests the portfolio effect. compare against current weighted-sum baseline ($1,446 total, $0.78/trade, 56.8% win rate, PF ~3.09).

### phase 2: isolate each window

backtest with one window enabled at a time. for each:
- trade count, P&L, PF, win rate, worst loss
- per-ticker breakdown
- exit reason distribution

this identifies which windows carry the alpha and which are noise.

### phase 3: sensitivity analysis

for each window, sweep condition thresholds:
- e.g., "5m thrust" lead_by: 0.10, 0.15, 0.20, 0.25
- e.g., "aligned" spread: 0.10, 0.15, 0.20, 0.25

this finds the optimal tightness for each window.

### phase 4: per-ticker windows

test ticker-specific variants:
- NVDA "5m thrust" at 5m >= 0.45 (vs 0.50 base)
- AAPL "aligned" only (its 1m-noise share is 35%)

### implementation

use existing `ConfigOverrides` pattern in `crates/backtest/src/main.rs`:

```bash
# test all windows
./target/release/backtest --date 2024-06-15 --entry-action entry_window --windows all

# test single window
./target/release/backtest --date 2024-06-15 --entry-action entry_window --window 5m_thrust

# compare against baseline
./target/release/backtest --date 2024-06-15 --entry-action score_threshold
```

or use the shell script pattern from `scripts/test_candle_pattern.sh` with a `run_sweep()` function.

---

## 8. config schema

entry windows fit naturally into the existing `ActionConfig` structure:

```json
{
  "actions": [
    {
      "action_type": "entry_window",
      "instance_id": "window_5m_thrust",
      "phase": "Entry",
      "enabled": true,
      "priority": 10,
      "params": {
        "name": "5m thrust",
        "direction": "long",
        "conditions": [
          {"type": "timescale_lead", "timescale": "FiveMinute", "lead_by": 0.15},
          {"type": "timescale_min", "timescale": "FiveMinute", "min_score": 0.50},
          {"type": "timescale_min", "timescale": "OneHour", "min_score": 0.0}
        ]
      }
    },
    {
      "action_type": "entry_window",
      "instance_id": "window_aligned",
      "phase": "Entry",
      "enabled": true,
      "priority": 5,
      "params": {
        "name": "aligned bias",
        "direction": "long",
        "conditions": [
          {"type": "timescale_spread_max", "max_spread": 0.15},
          {"type": "timescale_all_min", "min_score": 0.10}
        ]
      }
    },
    {
      "action_type": "entry_window",
      "instance_id": "reject_1m_noise",
      "phase": "Entry",
      "enabled": true,
      "priority": 100,
      "params": {
        "name": "1m noise reject",
        "is_reject_gate": true,
        "conditions": [
          {"type": "timescale_lead", "timescale": "OneMinute", "lead_by": 0.15},
          {"type": "timescale_max", "timescale": "FiveMinute", "max_score": 0.35}
        ]
      }
    }
  ]
}
```

### condition types

| type | params | meaning |
|------|--------|---------|
| `timescale_min` | timescale, min_score | timescale score >= min_score |
| `timescale_max` | timescale, max_score | timescale score <= max_score |
| `timescale_lead` | timescale, lead_by | timescale leads all others by >= lead_by |
| `timescale_spread_max` | max_spread | max(all) - min(all) <= max_spread |
| `timescale_all_min` | min_score | all timescale scores >= min_score |
| `indicator_min` | instance_id, min_score | specific indicator score >= min_score |
| `indicator_max` | instance_id, max_score | specific indicator score <= max_score |
| `indicator_range` | instance_id, min, max | score within [min, max] |
| `timescale_range` | timescale, min, max | timescale score within [min, max] |

this is extensible — new condition types can be added without config schema changes (they're just param objects in the conditions array).

### multiple windows in tick loop

the tick loop (`tick_loop.rs` lines 200-251) already iterates through `entry_actions`:

```rust
for action in &self.entry_actions {
    let signal = action.evaluate(None, market, &scores);
    if let ActionSignal::Enter { .. } = signal {
        // ... sizing and position opening
        break;  // first match wins
    }
}
```

reject gates have highest priority and return `ActionSignal::Hold` to block entry. positive windows are checked in priority order — first match wins.

---

## 9. future extensions

### per-window exit strategy (phase 2 — after initial window validation)

different entry regimes should exit differently. our own data proves this — ScoreExit has 74.4% win rate on 5m thrust but only 45.3% on aligned entries. the practical problem: ScoreExit uses the composite score, which is now partially decoupled from entry decisions. a 5m thrust entry may see the composite drop below -0.05 as 1m noise settles — exactly when you should be holding, not exiting.

**important:** validate window quality with current exits first. don't change exit behavior until windows are proven.

| window | suggested exit bias | rationale |
|--------|-------------------|-----------|
| 5m thrust | score_exit_threshold: -0.20, atr_multiplier: 5.0, max_hold: 45min | fast momentum play, exit on reversal |
| aligned | disable score_exit, use session_close, max_hold: 90min | broad trend, don't let score noise shake you out |
| 1h trend | wider stop, max_hold: 90min | hourly moves are slower, need time |
| engulfing reversal | tight stop at pattern low, max_hold: 45min | clear invalidation level |

implementation: `ActionSignal::Enter.reason` already carries a string — encode window name there. exit actions can check the reason to select strategy. add `exit_overrides` to window config params:

```json
{
  "exit_overrides": {
    "score_exit_threshold": -0.20,
    "atr_multiplier": 5.0,
    "max_hold_ms": 2700000
  }
}
```

### agent evolution of windows

agents can:
- **add new windows**: propose configs with new `entry_window` actions
- **tune windows**: adjust condition thresholds (min_score, lead_by, etc.)
- **disable windows**: set `enabled: false` on underperforming windows
- **create ticker-specific windows**: per-ticker overrides with different conditions

the config versioning system (append-only, proposed → backtested → validated → promoted) provides safety.

### composite score as a window

the current weighted-sum can be expressed as a window:

```json
{
  "action_type": "entry_window",
  "instance_id": "window_composite_fallback",
  "params": {
    "name": "composite fallback",
    "conditions": [
      {"type": "composite_min", "min_score": 0.50}
    ]
  }
}
```

this preserves backward compatibility during transition — raise the composite threshold to 0.50 (only high-conviction weighted-sum entries) and let windows handle the rest.

---

## 10. expected outcomes

### conservative estimate (windows 1-4 + reject gate)

| metric | current | projected | change |
|--------|---------|-----------|--------|
| trades/year | ~462 | ~300-350 | -25% |
| P&L/trade | $0.78 | $1.10-1.30 | +40-65% |
| win rate | 56.8% | 62-65% | +5-8pp |
| profit factor | ~3.09 | ~4.0-5.0 | +30-60% |
| total P&L | $1,446 | $1,200-1,400 | -3% to flat |
| transaction costs | ~$370 | ~$280 | -25% |

the trade count drops but per-trade quality improves. total P&L may be slightly lower initially but with better risk-adjusted returns (higher PF, higher win rate, lower transaction costs).

### key risk

the initial window definitions are based on historical clustering. they need validation on out-of-sample data to confirm they're not overfit. the testing strategy (section 7) addresses this.

---

## appendix: current knobs reference

### scoring pipeline
- `entry_threshold`: 0.40, `exit_threshold`: -0.05
- `timescale_weights`: {1m: 0.10, 5m: 0.60, 1h: 0.30}
- `aggregation`: WeightedSumWithGates
- `hard_gate_timescales`: [OneHour]
- `agreement`: disabled (previously tested as "catastrophic" — should be revisited post-bug-fix)

### 5-minute indicators (8)
- MACD(12/26/9) w=0.40, EMA(20) w=0.30, StochRSI(14) w=0.15
- RSI(14) w=0.10, Bollinger(20,2.0) w=0.05
- OFI w=0.05, momentum_persistence w=0.05, candle_pattern w=0.05

### 1-hour indicators (5)
- SuperTrend(10,3.0) w=0.30, EMA(20) w=0.25, ADX(14) w=0.20
- VWAP_distance w=0.15, Bollinger_bandwidth(20,2.0) w=0.10

### 1-minute indicators (4)
- RSI(7), Stochastic(14), ROC(12), MACD(6/13/5)

### exit actions
- ATR trailing stop (7x multiplier)
- fixed stop (2.5%)
- max hold timeout (90min, adaptive +30/-15)
- session close
- score exit (composite <= -0.05)
- breakeven stop (1.5% trigger, documented no-op)

### sizing
- volatility-scaled: base 5%, baseline_atr 1.0, max 3%, lookback 20

### session rules
- avoid_first_minutes: 30
- no_new_entries_after: 11:00 ET *(exhaustively validated: tested 11:00, 12:00, 13:00, 14:00, 15:30 — morning-only won on both IS and OOS. afternoon entries were consistently negative across all configs. this is a timing finding, scale-independent. a regime-aware late-session gate was considered but rejected — the 11am cutoff is one of our most robust findings.)*
- entry_cooldown_ms: 30000
- max_daily_loss_pct: 0.10

### per-ticker overrides
- AAPL: entry_threshold 0.35

### data sources
- 4-year trade CSVs: `data/backtest_202{2,3,4,5}_trades.csv`
- CSV columns: `$14=entry_composite, $16=entry_1m, $17=entry_5m, $18=entry_1h`
- tuning log: `docs/backtest_tuning_log.md`
- pre-bug-fix archive: `docs/pre-bug-fix.md`
