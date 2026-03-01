use types::market::Timescale;
use types::scoring::{AggregationMethod, ScoringConfig, TimescaleScores};

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

/// compute the composite score from per-timescale scores and config.
/// mutates `scores.composite` in place.
pub fn compute_composite(scores: &mut TimescaleScores, config: &ScoringConfig) {
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
    };

    scores.composite = raw_composite;
}
