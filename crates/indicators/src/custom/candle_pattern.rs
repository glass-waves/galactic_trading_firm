use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{Candle, MarketState, Timescale};

/// candlestick pattern recognition indicator.
///
/// v1 implements engulfing pattern detection with confluence scoring.
/// research basis: Bulkowski (4.7M candle study), Quantified Strategies (SPY backtests).
///
/// bearish engulfing traded long (mean reversion) on SPY: 71-76% win rate, 2.5 PF.
/// standalone patterns without confirmation have minimal edge — confluence is essential.
pub struct CandlePattern {
    timescale: Timescale,
    #[allow(dead_code)]
    instance_id: String,

    // engulfing params
    engulfing_min_body_ratio: f64,
    engulfing_base_score: f64,

    // scoring mode: when true, bearish patterns score positive (mean reversion)
    mean_reversion_mode: bool,

    // when true, clamp negative scores to 0 (indicator can only boost, never block entries)
    affirmative_only: bool,

    // confluence params
    volume_lookback: usize,
    high_volume_threshold: f64,
    low_volume_threshold: f64,
    ema_fast: usize,
    ema_slow: usize,

    // whether to apply confluence multipliers
    use_confluence: bool,

    // minimum confluence product (vol * location * trend) to emit a non-zero score.
    // below this threshold, the pattern is detected but not strong enough to act on.
    min_confluence_product: f64,

    // how many candles back to scan for patterns (signal persistence).
    // a pattern found N candles ago contributes score * decay_factor^N.
    // the strongest (most recent) pattern wins.
    decay_candles: usize,
    decay_factor: f64,
}

impl CandlePattern {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        timescale: Timescale,
        instance_id: String,
        engulfing_min_body_ratio: f64,
        engulfing_base_score: f64,
        mean_reversion_mode: bool,
        volume_lookback: usize,
        high_volume_threshold: f64,
        low_volume_threshold: f64,
        ema_fast: usize,
        ema_slow: usize,
        use_confluence: bool,
        affirmative_only: bool,
        min_confluence_product: f64,
        decay_candles: usize,
        decay_factor: f64,
    ) -> Self {
        Self {
            timescale,
            instance_id,
            engulfing_min_body_ratio,
            engulfing_base_score,
            mean_reversion_mode,
            affirmative_only,
            volume_lookback,
            high_volume_threshold,
            low_volume_threshold,
            ema_fast,
            ema_slow,
            use_confluence,
            min_confluence_product,
            decay_candles,
            decay_factor,
        }
    }
}

/// engulfing pattern detection.
///
/// bullish engulfing: prev bearish + curr bullish, curr body fully contains prev body.
/// bearish engulfing: prev bullish + curr bearish, curr body fully contains prev body.
///
/// returns Some(true) for bullish, Some(false) for bearish, None if no match.
fn is_engulfing(prev: &Candle, curr: &Candle, min_body_ratio: f64) -> Option<bool> {
    let curr_range = curr.high - curr.low;
    if curr_range < f64::EPSILON {
        return None;
    }

    let curr_body = (curr.close - curr.open).abs();
    let curr_body_ratio = curr_body / curr_range;
    if curr_body_ratio < min_body_ratio {
        return None;
    }

    let prev_body_top = prev.close.max(prev.open);
    let prev_body_bottom = prev.close.min(prev.open);
    let curr_body_top = curr.close.max(curr.open);
    let curr_body_bottom = curr.close.min(curr.open);

    let prev_is_bullish = prev.close > prev.open;
    let curr_is_bullish = curr.close > curr.open;

    // must be opposite directions
    if prev_is_bullish == curr_is_bullish {
        return None;
    }

    // curr body must fully contain prev body
    if curr_body_top >= prev_body_top && curr_body_bottom <= prev_body_bottom {
        Some(curr_is_bullish)
    } else {
        None
    }
}

// --- future pattern stubs ---
//
// fn is_harami(prev: &Candle, curr: &Candle) -> Option<bool>
//     bullish harami: 76% win rate, 1.65 PF (Quantified Strategies, 306 trades on SPY).
//     curr body fully inside prev body, opposite direction. next priority after engulfing.
//
// fn is_hammer(candle: &Candle, wick_ratio: f64) -> bool
//     hammer: 60% base, 72% with RSI filter (TradesViz ES study).
//     small body at top of range, long lower shadow >= wick_ratio * body. reversal signal.
//
// fn is_shooting_star(candle: &Candle, wick_ratio: f64) -> bool
//     shooting star: inverse hammer for bearish signals.
//     small body at bottom of range, long upper shadow >= wick_ratio * body.
//
// fn is_pin_bar(candle: &Candle, wick_ratio: f64) -> Option<PinBarDirection>
//     pin bar: long rejection wick >= wick_ratio * body. needs VWAP/volume confirmation.
//     Some(Bullish) for lower wick rejection, Some(Bearish) for upper wick rejection.
//
// fn is_three_soldiers_or_crows(c: &[Candle; 3], min_body_ratio: f64) -> Option<bool>
//     three black crows: 78-79% reversal rate (Bulkowski). consistent across bull/bear markets.
//     three white soldiers: continuation pattern. 3-candle patterns degrade on intraday (<5m).
//     Some(true) for soldiers, Some(false) for crows.
//
// fn is_evening_star(c: &[Candle; 3], doji_threshold: f64) -> bool
//     evening star: 72% reversal rate (Bulkowski). bullish + small body + bearish.
//     morning star excluded: 51.85% win rate on intraday (TradesViz) — near random.

/// compute volume ratio: current candle volume / rolling average volume.
fn compute_volume_ratio(candles: &[Candle], lookback: usize) -> f64 {
    if candles.len() < 2 {
        return 1.0;
    }

    let last = candles.last().unwrap();
    let end = candles.len() - 1;
    let start = end.saturating_sub(lookback);
    let window = &candles[start..end];

    if window.is_empty() {
        return 1.0;
    }

    let avg_volume = window.iter().map(|c| c.volume).sum::<f64>() / window.len() as f64;
    if avg_volume < f64::EPSILON {
        return 1.0;
    }

    last.volume / avg_volume
}

/// compute simple EMA from close prices.
fn compute_ema(closes: &[f64], period: usize) -> f64 {
    if closes.is_empty() {
        return 0.0;
    }
    if closes.len() < period || period == 0 {
        // fallback: use simple average of available data
        return closes.iter().sum::<f64>() / closes.len() as f64;
    }

    let k = 2.0 / (period as f64 + 1.0);
    let mut ema = closes[0];
    for &c in &closes[1..] {
        ema = c * k + ema * (1.0 - k);
    }
    ema
}

/// volume multiplier: confirms pattern conviction via volume.
fn volume_multiplier(ratio: f64, high_thresh: f64, low_thresh: f64) -> f64 {
    if ratio >= high_thresh {
        1.3
    } else if ratio >= high_thresh * 0.75 {
        1.15
    } else if ratio < low_thresh {
        0.5
    } else {
        1.0
    }
}

/// location multiplier: confirms pattern location relative to VWAP.
/// reversal patterns below VWAP get boosted (buying at discount).
fn location_multiplier(vwap_distance_pct: f64, is_bullish_signal: bool) -> f64 {
    if is_bullish_signal && vwap_distance_pct < -0.001 {
        1.2 // bullish signal below VWAP
    } else if is_bullish_signal && vwap_distance_pct > 0.001 {
        0.8 // bullish signal above VWAP
    } else if !is_bullish_signal && vwap_distance_pct > 0.001 {
        1.2 // bearish signal above VWAP
    } else if !is_bullish_signal && vwap_distance_pct < -0.001 {
        0.8 // bearish signal below VWAP
    } else {
        1.0 // at VWAP
    }
}

/// trend multiplier: with-trend patterns get boosted.
fn trend_multiplier(ema_fast_val: f64, ema_slow_val: f64, is_bullish_signal: bool) -> f64 {
    let trend_is_up = ema_fast_val > ema_slow_val;
    if (trend_is_up && is_bullish_signal) || (!trend_is_up && !is_bullish_signal) {
        1.1 // with-trend
    } else {
        0.8 // counter-trend
    }
}

impl Indicator for CandlePattern {
    fn name(&self) -> &str {
        "candle_pattern"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        // need at least 2 candles for engulfing + enough for EMA/volume lookback
        let pattern_min = 2;
        if self.use_confluence {
            pattern_min.max(self.volume_lookback + 1).max(self.ema_slow + 1)
        } else {
            pattern_min
        }
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut metadata = HashMap::new();
        let n = candles.len();

        // scan backwards through recent candle pairs for patterns.
        // the most recent pattern wins; older patterns decay exponentially.
        let scan_depth = self.decay_candles.max(1);
        let mut best_score: f64 = 0.0;
        let mut best_age: usize = 0;
        let mut best_pattern = "none";

        for age in 0..scan_depth {
            let idx = n.saturating_sub(2 + age);
            if idx + 1 >= n {
                continue;
            }
            let pair_prev = &candles[idx];
            let pair_curr = &candles[idx + 1];

            if let Some(is_bullish) = is_engulfing(pair_prev, pair_curr, self.engulfing_min_body_ratio) {
                let direction = if is_bullish { 1.0 } else { -1.0 };
                let score = direction * self.engulfing_base_score;
                let score = if self.mean_reversion_mode { -score } else { score };

                // apply exponential decay based on age
                let decay = self.decay_factor.powi(age as i32);
                let decayed_score = score * decay;

                // keep the strongest (highest absolute) signal
                if decayed_score.abs() > best_score.abs() {
                    best_score = decayed_score;
                    best_age = age;
                    best_pattern = if is_bullish { "bullish_engulfing" } else { "bearish_engulfing" };
                }
            }
        }

        metadata.insert("pattern_name".to_string(), match best_pattern {
            "bullish_engulfing" => 1.0,
            "bearish_engulfing" => -1.0,
            _ => 0.0,
        });
        metadata.insert("pattern_age".to_string(), best_age as f64);

        if best_score.abs() < f64::EPSILON {
            return Some(IndicatorOutput {
                score: 0.0,
                raw_value: 0.0,
                metadata,
            });
        }

        // apply confluence multipliers (computed on current candle state)
        let curr = &candles[n - 1];
        let final_score = if self.use_confluence {
            let vol_ratio = compute_volume_ratio(candles, self.volume_lookback);
            let vol_mult = volume_multiplier(vol_ratio, self.high_volume_threshold, self.low_volume_threshold);

            let vwap_dist = if market.session_vwap > f64::EPSILON {
                (curr.close - market.session_vwap) / market.session_vwap
            } else {
                0.0
            };
            let is_bullish_signal = best_score > 0.0;
            let loc_mult = location_multiplier(vwap_dist, is_bullish_signal);

            let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
            let ema_f = compute_ema(&closes, self.ema_fast);
            let ema_s = compute_ema(&closes, self.ema_slow);
            let trend_mult = trend_multiplier(ema_f, ema_s, is_bullish_signal);

            let confluence_product = vol_mult * loc_mult * trend_mult;

            metadata.insert("volume_ratio".to_string(), vol_ratio);
            metadata.insert("volume_mult".to_string(), vol_mult);
            metadata.insert("vwap_distance".to_string(), vwap_dist);
            metadata.insert("location_mult".to_string(), loc_mult);
            metadata.insert("trend_mult".to_string(), trend_mult);
            metadata.insert("confluence_product".to_string(), confluence_product);

            if confluence_product < self.min_confluence_product {
                0.0
            } else {
                (best_score * confluence_product).clamp(-1.0, 1.0)
            }
        } else {
            best_score.clamp(-1.0, 1.0)
        };

        let body_ratio = if (curr.high - curr.low).abs() > f64::EPSILON {
            (curr.close - curr.open).abs() / (curr.high - curr.low)
        } else {
            0.0
        };
        metadata.insert("body_ratio".to_string(), body_ratio);

        // affirmative-only: clamp negative scores to 0
        let output_score = if self.affirmative_only {
            final_score.max(0.0)
        } else {
            final_score
        };

        Some(IndicatorOutput {
            score: output_score,
            raw_value: best_score,
            metadata,
        })
    }
}

pub fn candle_pattern_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let engulfing_min_body_ratio = config
        .params
        .get("engulfing_min_body_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let engulfing_base_score = config
        .params
        .get("engulfing_base_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.70);
    let mean_reversion_mode = config
        .params
        .get("mean_reversion_mode")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let volume_lookback = config
        .params
        .get("volume_lookback")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let high_volume_threshold = config
        .params
        .get("high_volume_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);
    let low_volume_threshold = config
        .params
        .get("low_volume_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let ema_fast = config
        .params
        .get("ema_fast")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;
    let ema_slow = config
        .params
        .get("ema_slow")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let use_confluence = config
        .params
        .get("use_confluence")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let affirmative_only = config
        .params
        .get("affirmative_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let min_confluence_product = config
        .params
        .get("min_confluence_product")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let decay_candles = config
        .params
        .get("decay_candles")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;
    let decay_factor = config
        .params
        .get("decay_factor")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.6);

    Box::new(CandlePattern::new(
        config.timescale,
        config.instance_id.clone(),
        engulfing_min_body_ratio,
        engulfing_base_score,
        mean_reversion_mode,
        volume_lookback,
        high_volume_threshold,
        low_volume_threshold,
        ema_fast,
        ema_slow,
        use_confluence,
        affirmative_only,
        min_confluence_product,
        decay_candles,
        decay_factor,
    ))
}
