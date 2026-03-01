use types::market::Timescale;
use types::scoring::{AggregationMethod, ScoringConfig, TimescaleScores};

use engine::compute_composite;

fn make_scoring_config(
    weights: Vec<(Timescale, f64)>,
    method: AggregationMethod,
    gates: Vec<Timescale>,
) -> ScoringConfig {
    ScoringConfig {
        timescale_weights: weights.into_iter().collect(),
        entry_threshold: 0.65,
        exit_threshold: -0.30,
        aggregation: method,
        hard_gate_timescales: gates,
    }
}

// ── WeightedSum tests ──

#[test]
fn weighted_sum_correct_composite() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 2.0),
            (Timescale::OneHour, 1.0),
        ],
        AggregationMethod::WeightedSum,
        vec![],
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.5),
        five_minute: Some(0.8),
        one_hour: Some(-0.2),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    // (0.5*1.0 + 0.8*2.0 + -0.2*1.0) / (1+2+1) = 1.9/4.0 = 0.475
    assert!((scores.composite - 0.475).abs() < 1e-10, "got {}", scores.composite);
}

#[test]
fn weighted_sum_missing_timescale_renormalizes() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AggregationMethod::WeightedSum,
        vec![],
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.6),
        five_minute: None, // missing
        one_hour: Some(0.2),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    // (0.6*1.0 + 0.2*1.0) / 2.0 = 0.4
    assert!((scores.composite - 0.4).abs() < 1e-10, "got {}", scores.composite);
}

// ── WeightedSumWithGates tests ──

#[test]
fn hard_gate_negative_floors_to_zero() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
        ],
        AggregationMethod::WeightedSumWithGates,
        vec![Timescale::OneMinute], // 1min is hard gated
    );
    let mut scores = TimescaleScores {
        one_minute: Some(-0.3), // gate fails
        five_minute: Some(0.9),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    assert!((scores.composite - 0.0).abs() < f64::EPSILON, "gate should floor to 0, got {}", scores.composite);
}

#[test]
fn hard_gate_positive_normal_computation() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
        ],
        AggregationMethod::WeightedSumWithGates,
        vec![Timescale::OneMinute],
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.5), // gate passes
        five_minute: Some(0.7),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    let expected = (0.5 + 0.7) / 2.0;
    assert!((scores.composite - expected).abs() < 1e-10, "got {}", scores.composite);
}

#[test]
fn hard_gate_none_conservative_floor() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
        ],
        AggregationMethod::WeightedSumWithGates,
        vec![Timescale::OneMinute],
    );
    let mut scores = TimescaleScores {
        one_minute: None, // no data → conservative gate failure
        five_minute: Some(0.9),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    assert!((scores.composite - 0.0).abs() < f64::EPSILON, "None gate should floor to 0, got {}", scores.composite);
}

#[test]
fn multiple_hard_gates_all_must_pass() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AggregationMethod::WeightedSumWithGates,
        vec![Timescale::OneMinute, Timescale::OneHour],
    );

    // one passes, one fails → composite 0
    let mut scores = TimescaleScores {
        one_minute: Some(0.5),
        five_minute: Some(0.8),
        one_hour: Some(-0.1), // gate fails
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    assert!((scores.composite - 0.0).abs() < f64::EPSILON);

    // both pass → normal
    scores.one_hour = Some(0.3);
    compute_composite(&mut scores, &config);
    assert!(scores.composite > 0.0);
}

// ── MinScore tests ──

#[test]
fn min_score_takes_minimum() {
    let config = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AggregationMethod::MinScore,
        vec![],
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.8),
        five_minute: Some(-0.3),
        one_hour: Some(0.5),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    assert!((scores.composite - (-0.3)).abs() < 1e-10, "got {}", scores.composite);
}

// ── threshold checks ──

#[test]
fn entry_threshold_check() {
    let config = make_scoring_config(
        vec![(Timescale::FiveMinute, 1.0)],
        AggregationMethod::WeightedSum,
        vec![],
    );

    let mut scores = TimescaleScores {
        five_minute: Some(0.7),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    assert!(scores.composite >= config.entry_threshold, "0.7 should exceed 0.65 entry threshold");

    scores.five_minute = Some(0.3);
    compute_composite(&mut scores, &config);
    assert!(scores.composite < config.entry_threshold, "0.3 should be below 0.65 entry threshold");
}

#[test]
fn exit_threshold_check() {
    let config = make_scoring_config(
        vec![(Timescale::FiveMinute, 1.0)],
        AggregationMethod::WeightedSum,
        vec![],
    );

    let mut scores = TimescaleScores {
        five_minute: Some(-0.5),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    assert!(scores.composite <= config.exit_threshold, "-0.5 should trigger exit at -0.3 threshold");

    scores.five_minute = Some(0.1);
    compute_composite(&mut scores, &config);
    assert!(scores.composite > config.exit_threshold, "0.1 should not trigger exit");
}

#[test]
fn timescale_weight_normalization() {
    // weights of different magnitude should produce same result when proportional
    let config_a = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 2.0),
        ],
        AggregationMethod::WeightedSum,
        vec![],
    );
    let config_b = make_scoring_config(
        vec![
            (Timescale::OneMinute, 10.0),
            (Timescale::FiveMinute, 20.0),
        ],
        AggregationMethod::WeightedSum,
        vec![],
    );

    let mut scores_a = TimescaleScores {
        one_minute: Some(0.5),
        five_minute: Some(0.8),
        ..Default::default()
    };
    let mut scores_b = scores_a.clone();

    compute_composite(&mut scores_a, &config_a);
    compute_composite(&mut scores_b, &config_b);

    assert!(
        (scores_a.composite - scores_b.composite).abs() < 1e-10,
        "proportional weights should give same result: {} vs {}",
        scores_a.composite,
        scores_b.composite
    );
}
