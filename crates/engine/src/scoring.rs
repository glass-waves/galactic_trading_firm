use std::collections::HashMap;

use types::market::Timescale;
use types::scoring::{AggregationMethod, AgreementMode, ScoringConfig, TimescaleScores};

/// get the timescale score from TimescaleScores by Timescale enum variant.
fn get_timescale_score(scores: &TimescaleScores, ts: &Timescale) -> Option<f64> {
    match ts {
        Timescale::OneMinute => scores.one_minute,
        Timescale::FiveMinute => scores.five_minute,
        Timescale::OneHour => scores.one_hour,
        Timescale::OneDay => scores.one_day,
        Timescale::OneMonth => scores.one_month,
    }
}

/// check if all hard gate timescales pass (score > 0).
/// None from a hard-gated timescale = gate failure (conservative).
fn hard_gates_pass(scores: &TimescaleScores, config: &ScoringConfig) -> bool {
    for ts in &config.hard_gate_timescales {
        match get_timescale_score(scores, ts) {
            Some(s) if s > 0.0 => {}
            _ => return false, // None or <= 0 → gate fails
        }
    }
    true
}

/// weighted sum of available timescale scores.
/// missing timescales are excluded and remaining weights re-normalized.
fn weighted_sum(scores: &TimescaleScores, config: &ScoringConfig) -> f64 {
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;

    for (ts, &weight) in &config.timescale_weights {
        if let Some(score) = get_timescale_score(scores, ts) {
            weighted_sum += score * weight;
            total_weight += weight;
        }
    }

    if total_weight.abs() < f64::EPSILON {
        0.0
    } else {
        weighted_sum / total_weight
    }
}

/// minimum of all available timescale scores.
fn min_score(scores: &TimescaleScores, config: &ScoringConfig) -> f64 {
    let mut min = f64::INFINITY;
    let mut found = false;

    for ts in config.timescale_weights.keys() {
        if let Some(score) = get_timescale_score(scores, ts) {
            min = min.min(score);
            found = true;
        }
    }

    if found {
        min
    } else {
        0.0
    }
}

/// compute cross-timescale agreement: 1.0 when all timescales agree,
/// approaching 0.0 when they diverge maximally.
fn compute_agreement(scores: &TimescaleScores, config: &ScoringConfig) -> f64 {
    let available: Vec<f64> = config
        .timescale_weights
        .keys()
        .filter_map(|ts| get_timescale_score(scores, ts))
        .collect();

    if available.len() <= 1 {
        return 1.0; // single timescale is trivially agreed
    }

    let max_score = available.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_score_val = available.iter().cloned().fold(f64::INFINITY, f64::min);
    1.0 - (max_score - min_score_val) / 2.0
}

/// compute the composite score from per-timescale scores and config.
/// mutates `scores.composite` in place.
///
/// the optional `indicator_outputs` parameter provides raw indicator scores
/// for dynamic fusion gate calculations.
pub fn compute_composite(
    scores: &mut TimescaleScores,
    config: &ScoringConfig,
) {
    compute_composite_with_indicators(scores, config, None)
}

/// compute composite with optional raw indicator outputs for dynamic fusion.
pub fn compute_composite_with_indicators(
    scores: &mut TimescaleScores,
    config: &ScoringConfig,
    indicator_outputs: Option<&HashMap<String, Option<f64>>>,
) {
    let raw_composite = match config.aggregation {
        AggregationMethod::WeightedSum => weighted_sum(scores, config),
        AggregationMethod::WeightedSumWithGates => {
            if hard_gates_pass(scores, config) {
                weighted_sum(scores, config)
            } else {
                0.0
            }
        }
        AggregationMethod::MinScore => min_score(scores, config),
        AggregationMethod::DynamicFusion => {
            dynamic_fusion(scores, config, indicator_outputs)
        }
    };

    // apply per-indicator hard gates
    if let Some(outputs) = indicator_outputs {
        for (instance_id, &min_score) in &config.hard_gate_indicators {
            match outputs.get(instance_id).and_then(|v| *v) {
                Some(score) if score < min_score => {
                    scores.composite = 0.0;
                    return;
                }
                _ => {} // indicator missing or above threshold — pass
            }
        }
    }

    // apply cross-timescale agreement adjustment
    let composite = match &config.agreement {
        Some(ac) if ac.enabled => {
            let agreement = compute_agreement(scores, config);
            match ac.mode {
                AgreementMode::ConfidenceMultiplier => {
                    raw_composite * agreement.powf(ac.exponent)
                }
                AgreementMode::HardGate => {
                    if agreement < ac.gate_threshold {
                        0.0
                    } else {
                        raw_composite
                    }
                }
            }
        }
        _ => raw_composite,
    };

    scores.composite = composite;
}

/// dynamic fusion: shift weights between fast and slow timescales
/// based on volatility and trend regime indicators.
fn dynamic_fusion(
    scores: &TimescaleScores,
    config: &ScoringConfig,
    indicator_outputs: Option<&HashMap<String, Option<f64>>>,
) -> f64 {
    // if no dynamic fusion config or no indicator data, fall back to weighted sum with gates
    let fusion_config = match &config.dynamic_fusion {
        Some(fc) => fc,
        None => {
            return if hard_gates_pass(scores, config) {
                weighted_sum(scores, config)
            } else {
                0.0
            };
        }
    };

    let outputs = match indicator_outputs {
        Some(o) => o,
        None => {
            return if hard_gates_pass(scores, config) {
                weighted_sum(scores, config)
            } else {
                0.0
            };
        }
    };

    // read volatility and trend indicator scores
    let vol_score = outputs
        .get(&fusion_config.volatility_indicator_id)
        .and_then(|v| *v);
    let trend_score = outputs
        .get(&fusion_config.trend_indicator_id)
        .and_then(|v| *v);

    // if either indicator is missing, fall back to static weights
    let (vol, trend) = match (vol_score, trend_score) {
        (Some(v), Some(t)) => (v, t),
        _ => {
            return if hard_gates_pass(scores, config) {
                weighted_sum(scores, config)
            } else {
                0.0
            };
        }
    };

    // compute sigmoid gate: high vol → positive shift (favor fast),
    // high trend → negative shift (favor slow)
    let gate_input = vol * fusion_config.volatility_weight
        - trend * fusion_config.trend_weight
        + fusion_config.bias;
    let gate = 1.0 / (1.0 + (-gate_input).exp()); // sigmoid [0, 1]
    let shift = (gate - 0.5) * 2.0 * fusion_config.adjustment; // [-adjustment, +adjustment]

    // adjust weights: fast timescales (OneMinute) get +shift, slow (OneHour) get -shift
    let mut adjusted_weights = config.timescale_weights.clone();
    for (ts, w) in adjusted_weights.iter_mut() {
        let adjustment = match ts {
            Timescale::OneMinute => shift,
            Timescale::FiveMinute => 0.0, // medium stays neutral
            Timescale::OneHour | Timescale::OneDay | Timescale::OneMonth => -shift,
        };
        *w = (*w + adjustment).max(0.0); // floor at 0
    }

    // re-normalize and compute
    let total: f64 = adjusted_weights.values().sum();
    if total.abs() < f64::EPSILON {
        return 0.0;
    }

    // check hard gates
    if !hard_gates_pass(scores, config) {
        return 0.0;
    }

    let mut sum = 0.0;
    let mut active_weight = 0.0;
    for (ts, &weight) in &adjusted_weights {
        if let Some(score) = get_timescale_score(scores, ts) {
            sum += score * weight;
            active_weight += weight;
        }
    }

    if active_weight.abs() < f64::EPSILON {
        0.0
    } else {
        sum / active_weight
    }
}
