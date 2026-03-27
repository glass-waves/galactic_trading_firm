# Entry Windows: Research-Backed Analysis & Recommendations

## Overall Assessment

The architecture is sound and the data analysis is strong. The core insight — that 78% of trades cluster at the barely-above-threshold band and 51% are 1m-noise-led — is compelling and well-evidenced. The proposed windows map directly to what the literature validates.

---

## Issues with the Proposed Windows

### Window 2 "aligned bias" has a hidden problem

Your data shows aligned entries have **65% win rate and $1.05/trade**, which looks good, but the exit reason table is the tell:

| Regime | ScoreExit win% | SessionClose win% | Timeout win% |
|--------|---------------|-------------------|--------------|
| aligned | 45.3% | 81.0% | 86.7% |

ScoreExit fires earliest and wins only 45% of the time on aligned entries. That means when the score reverses quickly after a "high-confidence aligned entry," you lose. The alignment condition says *all timescales currently agree*, but it says nothing about **momentum direction** — you're capturing both trend entries AND price-at-equilibrium entries. The latter will score-exit fast.

**Proposed fix:** Add a momentum confirmation condition. Specifically, require that `momentum_persistence_5min > 0.0` (your second-derivative ROC of ROC) — this filters out "aligned but flat" from "aligned and accelerating." Based on Dempster & Jones's findings on Boolean AND combinations of trend AND momentum, this is where the alpha lives.

---

### Window 3 "hourly trend" is missing a pullback filter

The pattern `weak / weak / strong` (1m weak, 5m weak, 1h strong) — 321 trades, 64.5% win rate, $1.12/trade — is your **highest-volume quality regime**. But the hourly trend window only requires `5m_score > 0.0`, which lets in cases where 5m is near zero, meaning price is still moving against the hourly trend. You want entries where the 1h trend is strong AND you're entering on a 5m pullback, not a 5m reversal.

**Proposed fix:** Add a `5m_score` range condition, not just a minimum: `5m_score ∈ [-0.10, +0.20]`. This explicitly captures the "pullback into hourly trend" setup — exactly what Elder's Triple Screen formalizes. This requires a new condition type `timescale_range` that your current schema doesn't have (you have `indicator_range` but not `timescale_range`).

---

### Window 4 "strong core" is redundant

Window 1 (`5m >= 0.50 AND 1h > 0.0`) and Window 3 (`1h >= 0.40 AND 5m > 0.0`) together cover most of what "5m >= 0.50 AND 1h >= 0.30" captures. Unless Windows 1 and 3 have different exits assigned, Window 4 mostly fires in overlap.

**Proposed fix:** Replace Window 4 with a dedicated OFI window (see New Windows below).

---

### Window 5 "engulfing reversal" needs a regime pre-filter

The candle pattern indicator has `mean_reversion_mode: true`. Mean-reversion entries during a trending regime get eaten alive — the pattern fires, price retraces briefly, then continues the trend. The microstructure literature (Cont et al. on OFI) and the regime research both confirm: candle pattern signals are predictive in low-trend, high-liquidity conditions, and noise in high-trend conditions.

**Proposed fix:** Gate this window on `adx_1hr < 0.25` (trend strength not dominant) OR `bollinger_bandwidth_1hr < 0.20` (volatility regime not expanded). This makes the engulfing window specifically a **mean-reversion-in-range** window rather than a universal one.

---

## Proposed New Windows

### New Window: "OFI Surge + 5m Confirmation"

Your OFI sits at 0.05 weight in the composite and is effectively invisible. Cont et al. (2014) shows OFI explains ~65% of contemporaneous price variation at short horizons — it's the single most informative intraday signal available, and it's **orthogonal** to price-based indicators. Burying it in a weighted sum destroys its utility.

```
conditions:
  ofi_5min_score >= 0.50          # strong order flow imbalance
  macd_5min_score >= 0.20         # trend direction confirms
  1h_score >= 0.0                 # hourly not bearish
```

Expected: fewer trades than Window 1, but potentially higher win rate since OFI is predictive at the 5-minute level specifically. Treat this as a **research window** (`enabled: false`) until you have empirical data.

---

### New Window: "VWAP Reclaim"

Your Section 6 lists this as a future idea — it should be Window 6 now. Price reclaiming VWAP intraday is a strong institutional reference level signal; your system already computes `vwap_distance` as a 1h indicator. The literature on microstructure reference prices supports this as a distinct regime with different characteristics than momentum or alignment windows.

```
conditions:
  vwap_distance_1hr_score >= 0.30    # price meaningfully above VWAP
  5m_score >= 0.30                   # 5m confirms
  adx_1hr >= 0.15                    # some trend strength present
```

This captures **institutional-driven directional moves** — distinct from the momentum-led 5m thrust or multi-timeframe alignment windows.

---

### New Reject Gate: "Late Session High Volatility"

Your `no_new_entries_after: 11:00 ET` rule is a blunt instrument — it cuts off all entries regardless of quality. Nystrup et al.'s intraday regime classifier shows volatility patterns are persistent and classifiable in real time. A more nuanced gate:

```
reject if:
  session_time > 14:00 ET
  AND bollinger_bandwidth_1hr > 0.30    # volatility expanded (late-day positioning)
```

This lets high-quality setups through in the afternoon when volatility is normal, while blocking the chaotic last-hour action when it's elevated.

---

## On Coupling Exits to Entry Windows

Your own data already answers this question. The exit reason table by regime is decisive:

| Regime | ScoreExit win% | Best exit |
|--------|---------------|-----------|
| 5m thrust | 74.4% | ScoreExit works — momentum exhaustion is predictable |
| aligned | 45.3% | ScoreExit is destroying value; ride to session close |
| 1m noise | 37.1% | ScoreExit loses badly |

The practical problem: `ScoreExit` is computed from the same composite that's now partially replaced by discrete windows. If a 5m thrust entry fires, the composite score may drop below -0.05 quickly as the 1m noise settles — which is exactly when you should be *holding*, not exiting.

The López de Prado meta-labeling framework formalizes this: the triple barrier method sets different take-profit and stop-loss distances based on the trade's entry regime, not a fixed ATR multiplier. Your 7x ATR trailing stop is a universal exit applied to trades with fundamentally different holding characteristics. Elder's Triple Screen, Kaufman's *Trading Systems and Methods* Ch. 7, and the practitioner consensus all agree: **decoupled exits are the naive default, not the correct one**.

### Concrete Implementation

Use the `position.entry_reason` tagging from your Section 9, then add `exit_overrides` to the config schema:

**5m thrust** — fast momentum play, exit quickly on reversal:
```json
{
  "exit_overrides": {
    "score_exit_threshold": -0.20,
    "atr_multiplier": 5.0,
    "max_hold_ms": 2700000
  }
}
```

**aligned bias** — broad trend play, disable score exit and ride it:
```json
{
  "exit_overrides": {
    "score_exit_threshold": null,
    "max_hold_ms": 5400000,
    "use_session_close": true
  }
}
```

**Important:** Validate window quality with current exits first, *then* wire up per-window exit overrides. You don't want exit variance polluting your window quality measurement during initial validation.

---

## Summary of Recommended Changes

| Change | Priority | Rationale |
|--------|----------|-----------|
| Add `momentum_persistence > 0.0` to Window 2 | High | Filters "aligned but flat" — saves ScoreExit blowouts |
| Replace `5m > 0.0` with `5m ∈ [-0.10, 0.20]` in Window 3 | High | Explicitly captures pullback-into-hourly-trend setup |
| Gate Window 5 on `adx_1hr < 0.25` or `bb_width < 0.20` | High | Mean-reversion candle patterns need a range regime |
| Replace Window 4 with OFI Surge window | Medium | OFI is orthogonal alpha; Window 4 is redundant |
| Add VWAP Reclaim as Window 6 | Medium | Distinct institutional regime not covered |
| Add late-session volatility reject gate | Medium | Replaces blunt 11am hard cutoff with regime-aware gate |
| Add `timescale_range` condition type to schema | Low | Needed for the pullback filter in Window 3 |
| Couple exits to windows via `exit_overrides` | After validation | Your own data already proves this is necessary |
