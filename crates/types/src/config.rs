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
    #[serde(default)]
    pub force_exit_by: String,

    /// skip the first N minutes of the session (opening volatility).
    pub avoid_first_minutes: u32,

    /// maximum concurrent positions.
    pub max_concurrent_positions: u32,

    /// maximum capital deployed as fraction of total.
    pub max_capital_deployed_pct: f64,

    /// minimum milliseconds between closing a position and opening a new one.
    #[serde(default)]
    pub entry_cooldown_ms: i64,

    /// if cumulative realized losses exceed this fraction of initial capital,
    /// block new entries for the rest of the session.
    #[serde(default)]
    pub max_daily_loss_pct: Option<f64>,
}

impl SessionConfig {
    /// parse "HH:MM" string to minutes since midnight.
    pub fn parse_hm_to_minutes(s: &str) -> Option<u32> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let h: u32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            Some(h * 60 + m)
        } else {
            None
        }
    }

    /// parse no_new_entries_after to minutes since midnight.
    pub fn no_new_entries_after_minutes(&self) -> Option<u32> {
        Self::parse_hm_to_minutes(&self.no_new_entries_after)
    }

    /// parse force_exit_by to minutes since midnight.
    pub fn force_exit_by_minutes(&self) -> Option<u32> {
        Self::parse_hm_to_minutes(&self.force_exit_by)
    }
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
                hard_gate_timescales: vec![Timescale::OneMinute], agreement: None, dynamic_fusion: None,
            },
            session: SessionConfig {
                no_new_entries_after: "15:30".to_string(),
                force_exit_by: "15:55".to_string(),
                avoid_first_minutes: 5,
                max_concurrent_positions: 3,
                max_capital_deployed_pct: 0.15,
                entry_cooldown_ms: 0,
                max_daily_loss_pct: None,
            },
        };

        let json = serde_json::to_string(&config).expect("serialize");
        let deserialized: StrategyConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_parse_hm_to_minutes() {
        assert_eq!(SessionConfig::parse_hm_to_minutes("15:30"), Some(930));
        assert_eq!(SessionConfig::parse_hm_to_minutes("09:30"), Some(570));
        assert_eq!(SessionConfig::parse_hm_to_minutes("bad"), None);
    }

    /// load the v2 seed config JSON and verify differentiated 1-min params.
    #[test]
    fn test_v2_config_rsi_7_1min() {
        let config: StrategyConfig = serde_json::from_str(V2_CONFIG_JSON).expect("parse v2 config");
        let rsi = config
            .indicators
            .iter()
            .find(|i| i.instance_id == "rsi_7_1min")
            .expect("rsi_7_1min not found");
        assert_eq!(rsi.params["period"].as_u64().unwrap(), 7);
        assert_eq!(rsi.timescale, Timescale::OneMinute);
    }

    #[test]
    fn test_v2_config_macd_fast_1min() {
        let config: StrategyConfig = serde_json::from_str(V2_CONFIG_JSON).expect("parse v2 config");
        let macd = config
            .indicators
            .iter()
            .find(|i| i.instance_id == "macd_fast_1min")
            .expect("macd_fast_1min not found");
        assert_eq!(macd.params["fast_period"].as_u64().unwrap(), 6);
        assert_eq!(macd.params["slow_period"].as_u64().unwrap(), 13);
        assert_eq!(macd.params["signal_period"].as_u64().unwrap(), 5);
    }

    #[test]
    fn test_v2_config_hourly_has_adx() {
        let config: StrategyConfig = serde_json::from_str(V2_CONFIG_JSON).expect("parse v2 config");
        let hourly: Vec<_> = config
            .indicators
            .iter()
            .filter(|i| i.timescale == Timescale::OneHour)
            .collect();
        assert_eq!(hourly.len(), 5, "hourly should have 5 indicators including ADX");
        assert!(
            hourly.iter().any(|i| i.instance_id == "adx_14_1hr"),
            "adx_14_1hr not found in hourly"
        );
    }

    #[test]
    fn test_v2_config_timescale_weights_sum() {
        let config: StrategyConfig = serde_json::from_str(V2_CONFIG_JSON).expect("parse v2 config");

        // check weights sum to ~1.0 per timescale
        for ts in &[Timescale::OneMinute, Timescale::FiveMinute, Timescale::OneHour] {
            let sum: f64 = config
                .indicators
                .iter()
                .filter(|i| i.timescale == *ts && i.enabled)
                .map(|i| i.weight)
                .sum();
            assert!(
                (sum - 1.0).abs() < 0.01,
                "weights for {:?} sum to {sum}, expected ~1.0",
                ts
            );
        }
    }

    const V2_CONFIG_JSON: &str = r#"{
        "schema_version": "0.2",
        "config_id": 2,
        "created_at": "2026-03-02T00:00:00Z",
        "created_by": "human",
        "parent_config_id": 1,
        "tickers": ["SPY", "QQQ", "AAPL", "NVDA", "MSFT"],
        "indicators": [
            {"indicator_type":"rsi","instance_id":"rsi_7_1min","timescale":"OneMinute","enabled":true,"weight":0.30,"params":{"period":7,"overbought":70,"oversold":30},"last_modified_by":"human","last_modified_at":"2026-03-02T00:00:00Z","modification_reason":"faster RSI"},
            {"indicator_type":"stochastic_fast","instance_id":"stoch_fast_14_1min","timescale":"OneMinute","enabled":true,"weight":0.25,"params":{"period":14},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"roc","instance_id":"roc_12_1min","timescale":"OneMinute","enabled":true,"weight":0.20,"params":{"period":12},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"macd","instance_id":"macd_fast_1min","timescale":"OneMinute","enabled":true,"weight":0.25,"params":{"fast_period":6,"slow_period":13,"signal_period":5,"normalization_factor":1.0},"last_modified_by":"human","last_modified_at":"2026-03-02T00:00:00Z","modification_reason":"faster MACD"},
            {"indicator_type":"rsi","instance_id":"rsi_14_5min","timescale":"FiveMinute","enabled":true,"weight":0.25,"params":{"period":14,"overbought":70,"oversold":30},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"ema","instance_id":"ema_20_5min","timescale":"FiveMinute","enabled":true,"weight":0.15,"params":{"period":20},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"bollinger","instance_id":"bb_20_5min","timescale":"FiveMinute","enabled":true,"weight":0.15,"params":{"period":20,"std_dev":2.0},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"macd","instance_id":"macd_5min","timescale":"FiveMinute","enabled":true,"weight":0.25,"params":{"fast_period":12,"slow_period":26,"signal_period":9,"normalization_factor":1.0},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"stochastic_rsi","instance_id":"stoch_rsi_5min","timescale":"FiveMinute","enabled":true,"weight":0.20,"params":{"rsi_period":14,"stoch_period":14},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"vwap_distance","instance_id":"vwap_dist_1hr","timescale":"OneHour","enabled":true,"weight":0.25,"params":{},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"supertrend","instance_id":"supertrend_1hr","timescale":"OneHour","enabled":true,"weight":0.25,"params":{"period":10,"multiplier":3.0},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"ema","instance_id":"ema_20_1hr","timescale":"OneHour","enabled":true,"weight":0.20,"params":{"period":20},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"bollinger_bandwidth","instance_id":"bb_bw_20_1hr","timescale":"OneHour","enabled":true,"weight":0.15,"params":{"period":20,"std_dev":2.0},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"indicator_type":"adx","instance_id":"adx_14_1hr","timescale":"OneHour","enabled":true,"weight":0.15,"params":{"period":14},"last_modified_by":"human","last_modified_at":"2026-03-02T00:00:00Z","modification_reason":"new ADX"}
        ],
        "actions": [
            {"action_type":"score_threshold_entry","instance_id":"entry_score","phase":"Entry","enabled":true,"priority":0,"params":{"entry_threshold":0.45,"short_threshold":-0.45},"last_modified_by":null,"last_modified_at":null,"modification_reason":null},
            {"action_type":"fixed_fractional","instance_id":"sizing_fixed","phase":"Sizing","enabled":true,"priority":0,"params":{"fraction":0.01},"last_modified_by":null,"last_modified_at":null,"modification_reason":null}
        ],
        "scoring": {
            "timescale_weights": {"OneMinute":0.20,"FiveMinute":0.50,"OneHour":0.30},
            "entry_threshold": 0.45,
            "exit_threshold": -0.15,
            "aggregation": "WeightedSumWithGates",
            "hard_gate_timescales": ["OneHour"]
        },
        "session": {
            "no_new_entries_after": "15:30",
            "force_exit_by": "15:55",
            "avoid_first_minutes": 5,
            "max_concurrent_positions": 2,
            "max_capital_deployed_pct": 0.10
        }
    }"#;
}
