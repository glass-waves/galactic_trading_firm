use std::collections::HashMap;

use types::indicator::IndicatorConfig;
use types::market::Timescale;
use types::scoring::TimescaleScores;

/// aggregate a set of (score, weight) pairs into a single weighted-sum score.
/// weights are renormalized to sum to 1.0 among the pairs provided.
/// returns None if no pairs are provided.
pub fn aggregate_timescale_scores(scores_and_weights: &[(f64, f64)]) -> Option<f64> {
    if scores_and_weights.is_empty() {
        return None;
    }

    let total_weight: f64 = scores_and_weights.iter().map(|(_, w)| w).sum();
    if total_weight.abs() < f64::EPSILON {
        return None;
    }

    let weighted_sum: f64 = scores_and_weights
        .iter()
        .map(|(score, weight)| score * weight)
        .sum();

    Some(weighted_sum / total_weight)
}

/// given indicator outputs (instance_id → score) and configs, compute per-timescale scores.
/// indicators that returned None are excluded and remaining weights are re-normalized.
pub fn compute_timescale_scores(
    outputs: &HashMap<String, Option<f64>>,
    configs: &[IndicatorConfig],
) -> TimescaleScores {
    // group configs by timescale
    let mut by_timescale: HashMap<Timescale, Vec<(f64, f64)>> = HashMap::new();
    for cfg in configs {
        if !cfg.enabled {
            continue;
        }
        if let Some(Some(score)) = outputs.get(&cfg.instance_id) {
            by_timescale
                .entry(cfg.timescale)
                .or_default()
                .push((*score, cfg.weight));
        }
    }

    let get_score = |ts: &Timescale| -> Option<f64> {
        by_timescale
            .get(ts)
            .and_then(|pairs| aggregate_timescale_scores(pairs))
    };

    TimescaleScores {
        one_minute: get_score(&Timescale::OneMinute),
        five_minute: get_score(&Timescale::FiveMinute),
        one_hour: get_score(&Timescale::OneHour),
        one_day: get_score(&Timescale::OneDay),
        one_month: get_score(&Timescale::OneMonth),
        composite: 0.0, // composite computed by engine scoring pipeline
    }
}
