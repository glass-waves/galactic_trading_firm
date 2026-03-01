use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::market::Timescale;

/// aggregated scores per timescale, computed from all active indicators.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TimescaleScores {
    pub one_minute: Option<f64>,
    pub five_minute: Option<f64>,
    pub one_hour: Option<f64>,
    pub one_day: Option<f64>,
    pub one_month: Option<f64>,
    pub composite: f64,
}

/// how timescale scores are combined into the composite score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// weight per timescale in composite score calculation.
    pub timescale_weights: HashMap<Timescale, f64>,

    /// composite score above this → eligible for entry.
    pub entry_threshold: f64,

    /// composite score below this while in position → exit signal.
    pub exit_threshold: f64,

    /// aggregation method.
    pub aggregation: AggregationMethod,

    /// hard gates: timescales whose score must be positive (>0).
    pub hard_gate_timescales: Vec<Timescale>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationMethod {
    WeightedSum,
    WeightedSumWithGates,
    MinScore,
}
