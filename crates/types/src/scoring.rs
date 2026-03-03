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

    /// cross-timescale agreement configuration.
    #[serde(default)]
    pub agreement: Option<AgreementConfig>,

    /// dynamic fusion gate configuration.
    #[serde(default)]
    pub dynamic_fusion: Option<DynamicFusionConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregationMethod {
    WeightedSum,
    WeightedSumWithGates,
    MinScore,
    DynamicFusion,
}

/// how cross-timescale agreement affects the composite score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgreementConfig {
    pub enabled: bool,
    pub mode: AgreementMode,
    /// exponent for ConfidenceMultiplier mode.
    pub exponent: f64,
    /// threshold for HardGate mode — below this, composite floors to 0.
    pub gate_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgreementMode {
    /// composite *= agreement^exponent
    ConfidenceMultiplier,
    /// if agreement < threshold → composite = 0
    HardGate,
}

/// config for dynamic regime-adaptive timescale fusion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicFusionConfig {
    /// how much volatility shifts weight toward fast timescales.
    pub volatility_weight: f64,
    /// how much trend strength shifts weight toward slow timescales.
    pub trend_weight: f64,
    /// bias term in sigmoid (0 = neutral).
    pub bias: f64,
    /// maximum weight adjustment magnitude.
    pub adjustment: f64,
    /// instance_id of the volatility indicator to read.
    pub volatility_indicator_id: String,
    /// instance_id of the trend indicator to read.
    pub trend_indicator_id: String,
}
