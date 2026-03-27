# pre-bug-fix learnings archive

this document preserves the valid insights from 6 phases of config tuning (96 iterations, jan–mar 2026) that were conducted before the position sizing bug was discovered and fixed on 2026-03-21.

**the bug**: `position_size = size_fraction * available_capital` (a dollar value, e.g. $300) was passed as the `size` parameter to `open_position()`, which treated it as the number of shares. this inflated all P&L by the share price factor (SPY ~400x, NVDA ~50-150x depending on period). the fix: `num_shares = position_dollars / fill_price`.

**what this means**: all absolute P&L numbers from pre-fix runs are invalid. however, timing-based decisions (when to enter/exit) and directional findings (which indicators help/hurt) remain valid because the same trades occurred regardless of sizing — only the P&L magnitude was wrong.

---

## timing insights (scale-independent, fully valid)

these decisions gate WHEN trades happen, not HOW BIG they are. the same entries/exits would occur at any position size.

- **morning-only window (`no_new_entries_after: 11:00`)**: afternoon entries were consistently negative across all configs. the 9:30-11:00 ET window captures the strongest intraday trends. tested: 11:00, 12:00, 13:00, 14:00, 15:30. morning-only won on both IS and OOS.

- **skip opening noise (`avoid_first_minutes: 30`)**: first 30 minutes of market open have high noise and false signals. skipping improves PF. tested: 0, 15, 30, 45, 60 minutes. 30 was optimal.

- **entry cooldown (`entry_cooldown_ms: 30000`)**: 30-second cooldown after position close prevents immediate re-entry on whipsaw days. eliminated churn that was the largest source of losses on volatile days like apr 7, 2025. tested: 0, 15s, 30s, 60s.

- **daily loss circuit breaker (`max_daily_loss_pct: 0.10`)**: blocks new entries after 10% cumulative daily loss. tested: 5%, 10%, 15%. 10% was conservative but effective.

- **adaptive hold timeout**: base 90 minutes, +30 min if profitable (`profit_extension_ms: 1800000`), -15 min if losing (`loss_reduction_ms: 900000`). marginally helpful. **warning**: profit_extension > 30 min was an overfitting trap — looked great on 20-day set but catastrophic on 100-day.

- **entry threshold 0.40**: tested 0.30-0.58 range across multiple datasets. 0.40 was the clear OOS winner on 100-day validation. lower thresholds increased trade count but degraded PF on larger datasets (classic overfitting to small sets).

- **exit threshold -0.05**: tighter than original -0.15. catches degrading positions earlier via score-based exit. tested: -0.25, -0.20, -0.15, -0.10, -0.05. tighter was better.

## indicator findings (directional validity preserved)

the RELATIVE contribution of indicators to signal quality is valid. which indicators help predict direction doesn't depend on position size.

### core signal: 5-minute trend-following
- **MACD (weight 0.40)**: primary trend signal on 5-min timescale. the strongest single indicator.
- **EMA (weight 0.30)**: trend direction confirmation. works in concert with MACD.
- **StochRSI (weight 0.15)**: momentum oscillator. adds value when aligned with trend.
- **RSI (weight 0.10)**: overbought/oversold filter. low weight is important — higher weight introduces mean-reversion bias that hurts on trend days.
- **Bollinger Bands (weight 0.05)**: volatility context. minimal direct contribution.

### trend filter: 1-hour hard gate
- **hourly hard gate**: if the 1hr timescale score < 0, composite floors to 0 and blocks entry. this is the single most important risk control — prevents counter-trend entries.
- **SuperTrend (0.30)**, **EMA (0.25)**, **ADX (0.20)**: the three dominant hourly indicators for trend detection.
- **VWAP distance (0.15)**, **Bollinger bandwidth (0.10)**: supplementary.

### timescale weights
- **1-min: 10%, 5-min: 60%, 1-hr: 30%**: the 5-min timescale dominates. reducing 1-min from 20% to 10% was a key breakthrough — the mean-reversion-biased 1-min signals were poisoning entries on volatile days.

### supplementary indicators (added in later phases)
- **OFI (order flow imbalance, weight 0.05)**: marginal value. synergistic with other indicators in combination, but zero effect when changed alone (because entry composites are typically well above threshold, so small weight changes don't flip entry decisions).
- **momentum persistence (ROC of ROC, weight 0.05)**: second derivative of price momentum. marginally improved results across all test datasets.

## what was tried and rejected

these features/indicators were A/B tested and found to be neutral or harmful. the directional findings are valid regardless of sizing.

| feature | result | reason |
|---------|--------|--------|
| **score-scaled sizing** | harmful | underperforms vol-scaled. the vol adjustment is load-bearing — score-scaled removes it. |
| **RVOL (relative volume)** | harmful at all weights | degrades PF from 14→4 on 20-day set. volume normalization doesn't add signal for these liquid names. |
| **cross-timescale agreement** | catastrophic | at all exponents (0.3, 0.5, 1.0), destroys too many entries. the hourly hard gate alone does the filtering job. |
| **dynamic fusion** | zero effect | sigmoid returns ~0.5 consistently with current indicator setup. no adaptive reweighting occurs. |
| **VPIN as scored indicator** | harmful (cuts P&L 39-45%) | filters too many trades by penalizing "informed trading" periods. |
| **VPIN hard gate** | no beneficial threshold | tested multiple raw VPIN thresholds. all reduced P&L. |
| **profit_extension > 30 min** | overfitting trap | +$144 on 20-day but -$6,960 on 100-day. |
| **position context meta-indicators** | non-functional | `position_direction`, `unrealized_pnl`, `hold_duration`, `session_remaining` — engine never populates `MarketState.position_context`. zero effect. |
| **market breadth** | not A/B tested (implemented but unused) | |
| **cross-ticker correlation** | not A/B tested (implemented but unused) | |
| **max_concurrent_positions > 1** | no effect | strategy naturally takes only 1 position at a time in the morning window. |

## architecture decisions (still valid)

- **2-agent consolidation (7→2)**: analysis agent + PM agent. based on research synthesis: task granularity > agent count. fewer agents eliminates ~28% of failure surface.
- **config versioning**: append-only, immutable. proposed → backtested → validated → promoted. rollback = promote older version.
- **tool belt model**: indicators (sensing, pure functions) + actions (doing, can be stateful within position lifetime). pluggable via registry.
- **entry/exit action separation**: entry is a single composite score threshold. exits are layered: score-exit → monitor (breakeven) → exit actions (ATR trailing, fixed stop, max hold, session close). priority order matters.

## per-ticker findings

- **AAPL benefits from entry_threshold=0.35** (vs 0.40 base): generates valid signals in the 0.35-0.40 composite range that other tickers don't. validated on both 20-day and 99-day IS + 40-day OOS. the only per-ticker override that survived validation.
- **SPY/QQQ** at lower thresholds: looked great on 20-day, degraded on 99-day. classic overfitting.
- **stops, ATR multiplier, sizing_fraction**: zero effect in morning-only regime. positions exit via score-exit, max hold, or session close before any stop triggers.
- **indicator weight per-ticker overrides**: zero effect. composites at entry are typically 0.55+, well above the 0.40 threshold, so ±0.10 weight changes (~0.01 composite shift) never flip entry decisions.

## infrastructure notes

- **alpaca rate limiting**: added retry with exponential backoff (3 retries, 1s/2s/4s delay) in `alpaca_loader.rs`. data quality warnings (candle count + rate limit detection) in all shell scripts.
- **`update_config.sh` dual-threshold trap**: the engine reads the `score_threshold_entry` action's `params.entry_threshold`, NOT `scoring.entry_threshold`. both must be updated when changing entry threshold. the CLI `--entry-threshold` flag handles both correctly; manual jq edits do not.
- **`get_arg()` uses `rposition`**: last CLI occurrence wins, enabling overrides (e.g., shell script sets slippage at 2.0, CLI extra arg overrides to 3.0).
