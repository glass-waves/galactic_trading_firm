use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::action::ActionConfig;
use crate::indicator::IndicatorConfig;
use crate::scoring::ScoringConfig;

/// complete strategy configuration. this is what gets serialized to
/// the database as the config blob. the execution engine deserializes
/// this on startup and on hot-reload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// schema version for forward compatibility.
    pub schema_version: String,

    /// config metadata
    pub config_id: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub parent_config_id: Option<i64>,

    /// instruments this config applies to.
    pub tickers: Vec<String>,

    /// all indicator instances (the sensing tool belt).
    pub indicators: Vec<IndicatorConfig>,

    /// all action instances (the doing tool belt).
    pub actions: Vec<ActionConfig>,

    /// scoring pipeline configuration.
    pub scoring: ScoringConfig,

    /// session-level rules.
    pub session: SessionConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionConfig {
    /// don't open new positions after this time (HH:MM, exchange local).
    pub no_new_entries_after: String,

    /// force close all positions by this time.
    pub force_exit_by: String,

    /// skip the first N minutes of the session (opening volatility).
    pub avoid_first_minutes: u32,

    /// maximum concurrent positions.
    pub max_concurrent_positions: u32,

    /// maximum capital deployed as fraction of total.
    pub max_capital_deployed_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ActionPhase;
    use crate::market::Timescale;
    use crate::scoring::AggregationMethod;
    use std::collections::HashMap;

    #[test]
    fn test_strategy_config_roundtrip() {
        let config = StrategyConfig {
            schema_version: "0.1".to_string(),
            config_id: 1,
            created_at: Utc::now(),
            created_by: "test".to_string(),
            parent_config_id: None,
            tickers: vec!["SPY".to_string(), "QQQ".to_string()],
            indicators: vec![IndicatorConfig {
                indicator_type: "rsi".to_string(),
                instance_id: "rsi_14".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight: 1.0,
                params: {
                    let mut m = HashMap::new();
                    m.insert("period".to_string(), serde_json::json!(14));
                    m
                },
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            actions: vec![ActionConfig {
                action_type: "score_threshold_entry".to_string(),
                instance_id: "entry_1".to_string(),
                phase: ActionPhase::Entry,
                enabled: true,
                priority: 0,
                params: HashMap::new(),
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            scoring: ScoringConfig {
                timescale_weights: {
                    let mut m = HashMap::new();
                    m.insert(Timescale::FiveMinute, 1.0);
                    m
                },
                entry_threshold: 0.65,
                exit_threshold: -0.30,
                aggregation: AggregationMethod::WeightedSum,
                hard_gate_timescales: vec![Timescale::OneMinute],
            },
            session: SessionConfig {
                no_new_entries_after: "15:30".to_string(),
                force_exit_by: "15:55".to_string(),
                avoid_first_minutes: 5,
                max_concurrent_positions: 3,
                max_capital_deployed_pct: 0.15,
            },
        };

        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: StrategyConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, deserialized);
    }
}
