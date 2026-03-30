use std::collections::HashMap;

use types::market::Timescale;
use types::scoring::{
    AggregationMethod, AgreementConfig, AgreementMode, DynamicFusionConfig,
    ScoringConfig, TimescaleScores,
};

use engine::{compute_composite, compute_composite_with_indicators};

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
        hard_gate_timescales: gates, agreement: None, dynamic_fusion: None,
        hard_gate_indicators: HashMap::new(),
                hourly_exit_override: None,
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

// ── Agreement tests ──

fn make_agreement_config(
    weights: Vec<(Timescale, f64)>,
    agreement: AgreementConfig,
) -> ScoringConfig {
    ScoringConfig {
        timescale_weights: weights.into_iter().collect(),
        entry_threshold: 0.65,
        exit_threshold: -0.30,
        aggregation: AggregationMethod::WeightedSum,
        hard_gate_timescales: vec![],
        agreement: Some(agreement),
        dynamic_fusion: None,
        hard_gate_indicators: HashMap::new(),
                hourly_exit_override: None,
    }
}

#[test]
fn agreement_aligned_timescales_no_reduction() {
    let config = make_agreement_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AgreementConfig {
            enabled: true,
            mode: AgreementMode::ConfidenceMultiplier,
            exponent: 1.0,
            gate_threshold: 0.3,
        },
    );
    // all similar → agreement ≈ 1.0 → composite unchanged
    let mut scores = TimescaleScores {
        one_minute: Some(0.6),
        five_minute: Some(0.6),
        one_hour: Some(0.6),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    // agreement = 1.0 - (0.6 - 0.6)/2 = 1.0 → composite unchanged at 0.6
    assert!((scores.composite - 0.6).abs() < 1e-6, "aligned scores should give 0.6, got {}", scores.composite);
}

#[test]
fn agreement_divergent_reduces_composite() {
    let config = make_agreement_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AgreementConfig {
            enabled: true,
            mode: AgreementMode::ConfidenceMultiplier,
            exponent: 1.0,
            gate_threshold: 0.3,
        },
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.8),
        five_minute: Some(0.3),
        one_hour: Some(-0.3),
        ..Default::default()
    };

    // without agreement
    let mut scores_no_agree = scores.clone();
    let config_no_agree = make_scoring_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AggregationMethod::WeightedSum,
        vec![],
    );
    compute_composite(&mut scores_no_agree, &config_no_agree);

    compute_composite(&mut scores, &config);
    assert!(
        scores.composite.abs() < scores_no_agree.composite.abs(),
        "divergent should reduce magnitude: {} vs {}",
        scores.composite,
        scores_no_agree.composite,
    );
}

#[test]
fn agreement_disabled_no_effect() {
    let config = make_agreement_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::FiveMinute, 1.0),
        ],
        AgreementConfig {
            enabled: false,
            mode: AgreementMode::ConfidenceMultiplier,
            exponent: 1.0,
            gate_threshold: 0.3,
        },
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.8),
        five_minute: Some(-0.2),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    // should be plain weighted sum = (0.8 + -0.2) / 2 = 0.3
    assert!((scores.composite - 0.3).abs() < 1e-10, "disabled agreement should not affect, got {}", scores.composite);
}

#[test]
fn agreement_hard_gate_mode() {
    let config = make_agreement_config(
        vec![
            (Timescale::OneMinute, 1.0),
            (Timescale::OneHour, 1.0),
        ],
        AgreementConfig {
            enabled: true,
            mode: AgreementMode::HardGate,
            exponent: 1.0,
            gate_threshold: 0.7,
        },
    );
    let mut scores = TimescaleScores {
        one_minute: Some(0.8),
        one_hour: Some(-0.3),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    // agreement = 1.0 - (0.8 - -0.3)/2 = 1.0 - 0.55 = 0.45 < 0.7 → floors to 0
    assert!((scores.composite - 0.0).abs() < f64::EPSILON, "low agreement should floor to 0, got {}", scores.composite);
}

#[test]
fn agreement_single_timescale_trivially_agreed() {
    let config = make_agreement_config(
        vec![(Timescale::FiveMinute, 1.0)],
        AgreementConfig {
            enabled: true,
            mode: AgreementMode::ConfidenceMultiplier,
            exponent: 2.0,
            gate_threshold: 0.3,
        },
    );
    let mut scores = TimescaleScores {
        five_minute: Some(0.7),
        ..Default::default()
    };
    compute_composite(&mut scores, &config);
    // single timescale → agreement = 1.0 → no reduction
    assert!((scores.composite - 0.7).abs() < 1e-10, "single timescale should not be reduced, got {}", scores.composite);
}

// ── DynamicFusion tests ──

fn make_fusion_config() -> ScoringConfig {
    ScoringConfig {
        timescale_weights: vec![
            (Timescale::OneMinute, 0.20),
            (Timescale::FiveMinute, 0.50),
            (Timescale::OneHour, 0.30),
        ]
        .into_iter()
        .collect(),
        entry_threshold: 0.45,
        exit_threshold: -0.15,
        aggregation: AggregationMethod::DynamicFusion,
        hard_gate_timescales: vec![],
        agreement: None,
        dynamic_fusion: Some(DynamicFusionConfig {
            volatility_weight: 2.0,
            trend_weight: 2.0,
            bias: 0.0,
            adjustment: 0.10,
            volatility_indicator_id: "atr_indicator".to_string(),
            trend_indicator_id: "adx_indicator".to_string(),
        }),
        hard_gate_indicators: HashMap::new(),
                hourly_exit_override: None,
    }
}

#[test]
fn dynamic_fusion_high_vol_shifts_to_fast() {
    let config = make_fusion_config();
    let mut scores = TimescaleScores {
        one_minute: Some(0.8),
        five_minute: Some(0.5),
        one_hour: Some(0.3),
        ..Default::default()
    };

    // high vol indicator → shift toward fast
    let mut indicators = HashMap::new();
    indicators.insert("atr_indicator".to_string(), Some(0.9)); // high vol
    indicators.insert("adx_indicator".to_string(), Some(0.1)); // low trend

    compute_composite_with_indicators(&mut scores, &config, Some(&indicators));
    let composite_high_vol = scores.composite;

    // low vol → shift toward slow
    let mut scores2 = TimescaleScores {
        one_minute: Some(0.8),
        five_minute: Some(0.5),
        one_hour: Some(0.3),
        ..Default::default()
    };
    let mut indicators2 = HashMap::new();
    indicators2.insert("atr_indicator".to_string(), Some(0.1)); // low vol
    indicators2.insert("adx_indicator".to_string(), Some(0.1)); // low trend

    compute_composite_with_indicators(&mut scores2, &config, Some(&indicators2));
    let composite_low_vol = scores2.composite;

    // when 1min score is highest, high vol should give higher composite
    assert!(
        composite_high_vol > composite_low_vol,
        "high vol should favor fast timescale: {composite_high_vol} vs {composite_low_vol}",
    );
}

#[test]
fn dynamic_fusion_strong_trend_shifts_to_slow() {
    let config = make_fusion_config();
    let mut scores = TimescaleScores {
        one_minute: Some(0.3),
        five_minute: Some(0.5),
        one_hour: Some(0.8),
        ..Default::default()
    };

    // high trend → shift toward slow
    let mut indicators = HashMap::new();
    indicators.insert("atr_indicator".to_string(), Some(0.1)); // low vol
    indicators.insert("adx_indicator".to_string(), Some(0.9)); // high trend

    compute_composite_with_indicators(&mut scores, &config, Some(&indicators));
    let composite_high_trend = scores.composite;

    // low trend → shift toward fast
    let mut scores2 = TimescaleScores {
        one_minute: Some(0.3),
        five_minute: Some(0.5),
        one_hour: Some(0.8),
        ..Default::default()
    };
    let mut indicators2 = HashMap::new();
    indicators2.insert("atr_indicator".to_string(), Some(0.1)); // low vol
    indicators2.insert("adx_indicator".to_string(), Some(0.1)); // low trend

    compute_composite_with_indicators(&mut scores2, &config, Some(&indicators2));
    let composite_low_trend = scores2.composite;

    // when hourly is highest, strong trend should favor slow → higher composite
    assert!(
        composite_high_trend > composite_low_trend,
        "strong trend should favor slow timescale: {composite_high_trend} vs {composite_low_trend}",
    );
}

#[test]
fn dynamic_fusion_disabled_matches_static() {
    // DynamicFusion aggregation but no config → falls back to weighted sum with gates
    let config = ScoringConfig {
        timescale_weights: vec![
            (Timescale::OneMinute, 0.20),
            (Timescale::FiveMinute, 0.50),
            (Timescale::OneHour, 0.30),
        ]
        .into_iter()
        .collect(),
        entry_threshold: 0.45,
        exit_threshold: -0.15,
        aggregation: AggregationMethod::DynamicFusion,
        hard_gate_timescales: vec![],
        agreement: None,
        dynamic_fusion: None, // no config → fallback
        hard_gate_indicators: HashMap::new(),
                hourly_exit_override: None,
    };

    let mut scores = TimescaleScores {
        one_minute: Some(0.5),
        five_minute: Some(0.6),
        one_hour: Some(0.4),
        ..Default::default()
    };
    compute_composite_with_indicators(&mut scores, &config, None);

    // should match static weighted sum
    let expected = (0.5 * 0.20 + 0.6 * 0.50 + 0.4 * 0.30) / (0.20 + 0.50 + 0.30);
    assert!((scores.composite - expected).abs() < 1e-10, "expected {expected}, got {}", scores.composite);
}

#[test]
fn dynamic_fusion_respects_hard_gates() {
    let mut config = make_fusion_config();
    config.hard_gate_timescales = vec![Timescale::OneHour];

    let mut scores = TimescaleScores {
        one_minute: Some(0.8),
        five_minute: Some(0.7),
        one_hour: Some(-0.2), // gate fails
        ..Default::default()
    };

    let mut indicators = HashMap::new();
    indicators.insert("atr_indicator".to_string(), Some(0.5));
    indicators.insert("adx_indicator".to_string(), Some(0.5));

    compute_composite_with_indicators(&mut scores, &config, Some(&indicators));
    assert!((scores.composite - 0.0).abs() < f64::EPSILON, "hard gate should floor to 0, got {}", scores.composite);
}

#[test]
fn dynamic_fusion_gate_clamped() {
    let config = make_fusion_config();
    let mut scores = TimescaleScores {
        one_minute: Some(0.5),
        five_minute: Some(0.5),
        one_hour: Some(0.5),
        ..Default::default()
    };

    // extreme inputs should not produce negative weights
    let mut indicators = HashMap::new();
    indicators.insert("atr_indicator".to_string(), Some(1.0));
    indicators.insert("adx_indicator".to_string(), Some(-1.0));

    compute_composite_with_indicators(&mut scores, &config, Some(&indicators));
    // should still produce a valid score
    assert!(scores.composite >= -1.0 && scores.composite <= 1.0, "score out of range: {}", scores.composite);
}

#[test]
fn dynamic_fusion_missing_indicator_defaults() {
    let config = make_fusion_config();
    let mut scores = TimescaleScores {
        one_minute: Some(0.5),
        five_minute: Some(0.6),
        one_hour: Some(0.4),
        ..Default::default()
    };

    // missing indicator → falls back to static weights
    let mut indicators = HashMap::new();
    indicators.insert("atr_indicator".to_string(), Some(0.5));
    // adx_indicator is missing

    compute_composite_with_indicators(&mut scores, &config, Some(&indicators));
    let expected = (0.5 * 0.20 + 0.6 * 0.50 + 0.4 * 0.30) / (0.20 + 0.50 + 0.30);
    assert!((scores.composite - expected).abs() < 1e-10, "missing indicator should fall back to static, expected {expected}, got {}", scores.composite);
}

#[test]
fn dynamic_fusion_weight_conservation() {
    let config = make_fusion_config();
    let mut scores = TimescaleScores {
        one_minute: Some(1.0),
        five_minute: Some(1.0),
        one_hour: Some(1.0),
        ..Default::default()
    };

    let mut indicators = HashMap::new();
    indicators.insert("atr_indicator".to_string(), Some(0.5));
    indicators.insert("adx_indicator".to_string(), Some(0.5));

    compute_composite_with_indicators(&mut scores, &config, Some(&indicators));
    // when all scores are 1.0, composite should be 1.0 regardless of weight distribution
    assert!((scores.composite - 1.0).abs() < 1e-6, "uniform scores should give 1.0, got {}", scores.composite);
}
