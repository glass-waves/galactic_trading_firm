# proposed additions to the trading system

gaps and missing capabilities discovered during 100-day config tuning (march 2025 – march 2026).

**status (2026-03-05):** all 13 features and 5 bug fixes implemented. see `docs/backtest_tuning_log.md` phase 3 for A/B test results.

---

## missing actions / knobs

### 1. daily loss circuit breaker ✅ implemented
**priority: high** — **status: done** (engine-level, `SessionConfig.max_daily_loss_pct`)

no mechanism to stop trading after accumulating losses within a day. on nov 10, 2025, the system entered 3 separate positions (AAPL, NVDA, MSFT) that all lost simultaneously, resulting in a -$1,973 day (19.7% of capital).

**proposed implementation**: new action type `daily_loss_limit` in the Monitor phase.
- params: `max_daily_loss_pct` (e.g., 0.10 = 10% of capital)
- tracks cumulative realized P&L for the session
- once threshold is breached, blocks all new entries for the remainder of the day
- does NOT force-close existing positions (let existing stops manage those)

### 2. re-entry cooldown per ticker ✅ implemented
**priority: high** — **status: done** (engine-level, `SessionConfig.entry_cooldown_ms`)

after a stop-out, the system can immediately re-enter the same ticker on the next tick. on high-volatility days (apr 7, jan 21), this causes rapid stop-out → re-entry → stop-out churn (71-72 trades).

**proposed implementation**: new param on `score_threshold` entry action.
- params: `cooldown_ms` (e.g., 300000 = 5 minutes)
- after a position closes on a ticker, block re-entry for that ticker for `cooldown_ms`
- requires tracking per-ticker last-exit timestamps in the engine

### 3. score-proportional position sizing ✅ implemented
**priority: medium** — **status: done** (action type `score_scaled` in sizing phase). A/B testing showed it underperforms vol-scaled sizing — not included in winning config but available for agents to enable.

the composite score only gates entry (>=0.58) but a barely-passing 0.58 entry gets the same position size as a strong 0.95 entry. higher-confidence entries should carry larger positions.

**proposed implementation**: new sizing action type `score_scaled` (or add params to `volatility_scaled`).
- params: `min_fraction`, `max_fraction`, `score_range` (e.g., 0.58-1.0 maps to 2%-8%)
- `adjusted_fraction = min + (max - min) * (score - entry_threshold) / (1.0 - entry_threshold)`
- composable with vol-scaling: apply score scale first, then vol adjustment

### 4. correlation-aware sizing ✅ implemented
**priority: medium** — **status: done** (`MarketState.total_deployed_capital`, `SessionConfig.max_capital_deployed_pct`)

when multiple tickers signal entry simultaneously, they're often correlated (broad market selloff/rally). the system sizes each independently, concentrating risk.

**proposed implementation**: param on sizing actions.
- params: `max_total_exposure_pct` (e.g., 0.15 = 15% of capital across all positions)
- when entering a new position, check total deployed capital
- if adding this position would exceed `max_total_exposure_pct`, reduce size proportionally
- note: `max_concurrent_positions` exists in session config but is NOT enforced in the engine

### 5. adaptive max hold time ✅ implemented
**priority: medium** — **status: done** (`max_hold_timeout` action params: `profit_extension_ms`, `loss_reduction_ms`)

current max hold is fixed at 90 minutes. extending to 120 hurt P&L because losers ran longer. winners need more time, losers need less.

**proposed implementation**: modify `max_hold_timeout` action.
- params: `base_max_hold_ms`, `profit_extension_ms`, `loss_reduction_ms`
- if position is profitable: max_hold = base + profit_extension
- if position is at a loss: max_hold = base - loss_reduction
- allows winners to run longer while cutting losers faster

### 6. per-ticker parameter overrides — deferred
**priority: low** — **status: deferred** (lower priority, can be added later via `ticker_overrides` in StrategyConfig)

all tickers share the same ATR multiplier, sizing params, and stop levels. NVDA (high vol, avg ATR ~4%) and SPY (low vol, avg ATR ~0.8%) have very different characteristics.

**proposed implementation**: add optional `ticker_overrides` section to config.
```json
"ticker_overrides": {
  "NVDA": { "atr_multiplier": 8.0, "base_fraction": 0.03 },
  "SPY":  { "atr_multiplier": 4.0, "base_fraction": 0.06 }
}
```

---

## missing indicators

### 7. cross-ticker correlation indicator ✅ implemented
**priority: high** — **status: done** (indicator type `cross_ticker_correlation`, reads `MarketState.cross_ticker_correlation`)

no indicator measuring real-time correlation between tickers. on days when SPY, QQQ, and mega-caps are all moving together (high correlation regime), the system should reduce confidence in individual-stock signals since diversification is illusory.

**proposed implementation**: custom indicator `cross_ticker_correlation`.
- computes rolling pairwise correlation between all active tickers over recent candles
- high correlation (>0.8) → score -1.0 (bearish signal — "everything is moving together, reduce exposure")
- low correlation (<0.3) → score +1.0 (normal regime, trust individual signals)
- requires: access to multiple tickers' candle data simultaneously (current `MarketState` is per-ticker — this is a significant architecture change. `TradingEngine` operates independently per ticker with no cross-ticker visibility. implementing this requires either: (a) a shared cross-ticker state object passed to all engines, or (b) a pre-computation step that calculates correlation before per-ticker engines run. option (b) is more modular.)

### 8. volume profile / relative volume indicator ✅ implemented
**priority: medium** — **status: done** (indicator type `relative_volume`). A/B testing showed RVOL degraded PF at weights 0.05–0.20 — not included in winning config but available for agents.

no indicator measuring whether current volume is unusual relative to historical norms. abnormally high volume often precedes large moves and can validate or invalidate entry signals.

**proposed implementation**: custom indicator `relative_volume`.
- computes ratio of current period volume to average volume at the same time of day over lookback period
- RVOL > 2.0 with bullish signal → strong confirmation (+1.0)
- RVOL > 2.0 with no signal → caution (neutral)
- RVOL < 0.5 → low conviction, reduce score (−0.3)
- requires: intraday volume profile data (time-of-day bucketed averages)

### 9. market breadth indicator ✅ implemented
**priority: medium** — **status: done** (indicator type `market_breadth`, reads `MarketState.index_return`)

the system trades SPY/QQQ + mega-caps but has no sense of overall market breadth. a rallying SPY with narrow breadth (only a few stocks driving it) is a weaker signal than broad-based strength.

**proposed implementation**: custom indicator `market_breadth`.
- if trading mega-caps alongside index ETFs, compare individual stock moves to index
- when individual stock is outperforming index → stronger signal
- when individual stock is underperforming index → weaker signal
- simple version: (stock_return - index_return) over rolling window

### 10. intraday momentum persistence indicator ✅ implemented
**priority: low** — **status: done** (composable indicator type `momentum_persistence`)

current indicators (MACD, EMA, RSI) are mostly price-level based. no indicator specifically measuring whether recent momentum is accelerating or decelerating.

**proposed implementation**: composable indicator `momentum_persistence`.
- compute rate of change of rate of change (second derivative of price)
- positive and increasing → momentum is strengthening (+1.0)
- positive but decreasing → momentum is fading (0.0 to −0.5)
- useful for timing exits: exit when momentum starts fading even if price is still rising

---

## engine-level gaps

### 11. session config enforcement in backtest ✅ fixed
**priority: high** — **status: done** (`TradingEngine` now accepts `SessionConfig`, enforces all session rules)

`SessionConfig` fields (`avoid_first_minutes`, `no_new_entries_after`, `max_concurrent_positions`, `force_exit_by`) are defined in the config but NOT enforced in the backtest engine. the backtest replay builds a `TradingEngine` without passing `SessionConfig`, so these knobs have zero effect. this means we're tuning on an incomplete simulation.

### 12. backtest trade-level detail output ✅ implemented
**priority: medium** — **status: done** (`--verbose` flag on backtest CLI)

the current backtest output only shows per-ticker totals. to understand WHY a day lost money, we need to see individual trade entry/exit times, prices, and exit reasons. currently requires `--write-db` and SQL queries.

**proposed implementation**: add `--verbose` flag to backtest CLI that prints per-trade detail:
```
  NVDA  #1  LONG  entry 14:05 $142.50  exit 14:35 $139.20  -$330  (atr_trailing_stop)
  NVDA  #2  LONG  entry 14:38 $139.50  exit 15:08 $138.00  -$150  (max_hold_timeout)
```

### 13. multi-day equity curve tracking ✅ implemented
**priority: medium** — **status: done** (`--output-equity` flag + `--compound` mode in shell scripts)

the backtest script treats each day independently with fresh $10k capital. there's no compounding or equity curve across the full period. a more realistic simulation would track cumulative equity and adjust position sizes based on account growth/decline.

---

## bugs: broken / disconnected parameters — all fixed ✅

these parameters previously existed in `SessionConfig` and `StrategyConfig` but were **not wired into the engine**. **all 5 have been fixed as of 2026-03-05.**

### `avoid_first_minutes` ✅ fixed
- **where defined**: `SessionConfig.avoid_first_minutes` in `crates/types/src/config.rs`
- **what it should do**: block new entries for the first N minutes after market open (avoids opening volatility)
- **what actually happens**: the field is parsed but `TradingEngine::on_tick()` in `crates/engine/src/tick_loop.rs` never checks it. the backtest replay in `crates/backtest/src/replay.rs` also does not enforce it.
- **fix**: in `TradingEngine::on_tick()`, before evaluating entry actions, check if the current tick's timestamp is within `avoid_first_minutes` of market open. if so, skip entry evaluation. requires passing `SessionConfig` into `TradingEngine` (currently not passed in backtest mode — see item #11).

### `max_concurrent_positions` ✅ fixed
- **where defined**: `SessionConfig.max_concurrent_positions` in `crates/types/src/config.rs`
- **what it should do**: limit how many open positions can exist simultaneously across all tickers
- **what actually happens**: the field is parsed but never checked. `TradingEngine` manages a single position per ticker but nothing enforces a cross-ticker limit.
- **fix**: `TradingEngine` currently operates per-ticker in the backtest (one engine per ticker). enforcing cross-ticker limits requires either: (a) a shared position counter passed to all engines, or (b) a coordinator layer above the per-ticker engines that gates entries. option (b) is cleaner. in paper trading mode (`data_feed`), `LiveSession` already operates per-ticker — the same coordinator pattern would apply.

### `no_new_entries_after` ✅ fixed
- **where defined**: `SessionConfig.no_new_entries_after` in `crates/types/src/config.rs`
- **what it should do**: block new entries after a specified time (e.g., 15:30 ET), allowing existing positions to exit naturally
- **what actually happens**: partially enforced. the backtest replay added session constraint checking in the `run_backtest()` function, but `TradingEngine::on_tick()` itself does not check this field. in paper trading mode, it is not enforced.
- **fix**: add a time check in `TradingEngine::on_tick()` before entry evaluation, similar to `avoid_first_minutes`. the engine needs access to the current market time (already available via `MarketState.timestamp`) and the session config.

### `force_exit_by` ✅ fixed
- **where defined**: `SessionConfig.force_exit_by` in `crates/types/src/config.rs`
- **what it should do**: force-close all positions at a specified time (e.g., 15:55 ET)
- **what actually happens**: the `session_close` action handles this independently via its own `force_exit_by` param in the action config. the `SessionConfig.force_exit_by` field is redundant and ignored. these two values can drift out of sync.
- **fix**: either (a) remove `force_exit_by` from `SessionConfig` since it's handled by the `session_close` action, or (b) make the `session_close` action read from `SessionConfig` instead of its own params, creating a single source of truth.

### `exit_threshold` ✅ fixed
- **where defined**: `ScoringConfig.exit_threshold` in `crates/types/src/scoring.rs`
- **what it should do**: trigger a score-based exit when composite score drops below this threshold while in a position
- **what actually happens**: `TradingEngine::on_tick()` does not check `exit_threshold` against the composite score for open positions. all exits are handled by the action-based exit mechanisms (trailing stop, fixed stop, timeout, session close).
- **fix**: either (a) implement score-based exits in `on_tick()` — after computing composite, if in position and composite <= exit_threshold, generate an exit signal, or (b) remove `exit_threshold` from the config to avoid confusion. option (a) adds a new exit mechanism; option (b) is simpler but reduces flexibility. **note**: during tuning, positions always exited via stops/timeouts before the score would have triggered, so this may not add value even if implemented.

---

## discovered no-ops (by design, not bugs)

these parameters work as coded but have minimal or zero practical effect under the current config due to the system's dynamics. they don't need code fixes, but agents should be aware they exist so they don't waste analysis cycles on them.

**note (2026-03-05)**: `avoid_first_minutes` and `max_concurrent_positions` moved out of this section — they were bugs (not enforced), now fixed above.

| parameter | why it has no effect |
|-----------|---------------------|
| `entry_threshold` in range 0.58-0.62 | composite score jumps discretely; no entries land in this range |
| `short_threshold` | composite never drops below -0.60 with current indicator weights |
| `breakeven_trigger` | breakeven stop rarely activates given 7x ATR trailing width |
| `baseline_atr` in range 0.8-1.0 | minimal effect on position sizing (ATR values don't vary enough) |
| ATR trailing multiplier above 7x | 2.5% fixed stop catches exits before trailing stop triggers |
| `vol_lookback` 20 vs 30 | identical with 3-day backtest lookback data |
