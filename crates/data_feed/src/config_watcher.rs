use std::fmt;

use tracing::{info, warn};
use types::StrategyConfig;

/// errors from config watching.
#[derive(Debug)]
pub enum ConfigError {
    DatabaseError(sqlx::Error),
    DeserializationError(serde_json::Error),
    ValidationError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::DatabaseError(e) => write!(f, "database error: {e}"),
            ConfigError::DeserializationError(e) => write!(f, "config deserialization error: {e}"),
            ConfigError::ValidationError(msg) => write!(f, "config validation error: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<sqlx::Error> for ConfigError {
    fn from(e: sqlx::Error) -> Self {
        ConfigError::DatabaseError(e)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::DeserializationError(e)
    }
}

/// polls postgres for new promoted configs on a timer.
/// returns Some when a newer config version exists.
pub struct ConfigWatcher {
    pool: sqlx::PgPool,
    current_version_id: i64,
}

impl ConfigWatcher {
    pub fn new(pool: sqlx::PgPool, initial_version_id: i64) -> Self {
        Self {
            pool,
            current_version_id: initial_version_id,
        }
    }

    /// check if a newer promoted config exists.
    /// returns Some((version_id, config)) if a new version was found.
    pub async fn check_for_update(
        &self,
    ) -> Result<Option<(i64, StrategyConfig)>, ConfigError> {
        let row: Option<(i64, serde_json::Value)> = sqlx::query_as(
            "SELECT id, config_blob FROM config_versions \
             WHERE status = 'promoted' AND id > $1 \
             ORDER BY promoted_at DESC LIMIT 1",
        )
        .bind(self.current_version_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some((version_id, config_blob)) => {
                let config: StrategyConfig = serde_json::from_value(config_blob)?;

                // basic validation
                if config.indicators.is_empty() {
                    return Err(ConfigError::ValidationError(
                        "config must have at least one indicator".to_string(),
                    ));
                }
                if config.actions.is_empty() {
                    return Err(ConfigError::ValidationError(
                        "config must have at least one action".to_string(),
                    ));
                }

                info!(
                    old_version = self.current_version_id,
                    new_version = version_id,
                    indicators = config.indicators.len(),
                    actions = config.actions.len(),
                    "new config version detected"
                );

                Ok(Some((version_id, config)))
            }
            None => Ok(None),
        }
    }

    /// acknowledge a config version after successfully loading it.
    /// this updates the tracked version so we don't re-detect it.
    pub fn acknowledge(&mut self, version_id: i64) {
        info!(
            old_version = self.current_version_id,
            new_version = version_id,
            "config version acknowledged"
        );
        self.current_version_id = version_id;
    }

    /// get the currently tracked version id.
    pub fn current_version_id(&self) -> i64 {
        self.current_version_id
    }
}

/// attempt to build a new engine from a config.
/// returns None if the config fails to build (bad indicator type, etc.).
/// on failure, logs a warning and the caller should keep the old engine.
pub fn try_build_engine(
    config: &StrategyConfig,
    ticker: &str,
    capital: f64,
) -> Option<engine::TradingEngine> {
    let ind_reg = indicators::default_indicator_registry();
    let indicators = match indicators::build_indicators(&config.indicators, &ind_reg) {
        Ok(i) => i,
        Err(e) => {
            warn!(error = %e, "failed to build indicators from new config, keeping old config");
            return None;
        }
    };

    let act_reg = actions::default_action_registry();
    let action_sets = match actions::build_actions(&config.actions, &act_reg) {
        Ok(a) => a,
        Err(e) => {
            warn!(error = %e, "failed to build actions from new config, keeping old config");
            return None;
        }
    };

    Some(engine::TradingEngine::new(
        indicators,
        config.indicators.clone(),
        config.scoring.clone(),
        action_sets.entry,
        action_sets.monitor,
        action_sets.exit,
        action_sets.sizing,
        ticker.to_string(),
        capital,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn config_watcher_construction() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let watcher = ConfigWatcher::new(pool, 1);
        assert_eq!(watcher.current_version_id(), 1);
    }

    #[tokio::test]
    async fn acknowledge_updates_version() {
        let pool = sqlx::PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let mut watcher = ConfigWatcher::new(pool, 1);
        watcher.acknowledge(5);
        assert_eq!(watcher.current_version_id(), 5);
    }

    #[test]
    fn invalid_config_fallback() {
        use types::scoring::{AggregationMethod, ScoringConfig};
        use types::config::{SessionConfig, StrategyConfig};

        // config with unknown indicator type
        let config = StrategyConfig {
            schema_version: "0.1".to_string(),
            config_id: 1,
            created_at: chrono::Utc::now(),
            created_by: "test".to_string(),
            parent_config_id: None,
            tickers: vec!["SPY".to_string()],
            indicators: vec![types::IndicatorConfig {
                indicator_type: "nonexistent_indicator".to_string(),
                instance_id: "bad_1".to_string(),
                timescale: types::Timescale::FiveMinute,
                enabled: true,
                weight: 1.0,
                params: std::collections::HashMap::new(),
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            actions: vec![types::ActionConfig {
                action_type: "score_threshold_entry".to_string(),
                instance_id: "e1".to_string(),
                phase: types::ActionPhase::Entry,
                enabled: true,
                priority: 0,
                params: std::collections::HashMap::new(),
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            scoring: ScoringConfig {
                timescale_weights: std::collections::HashMap::new(),
                entry_threshold: 0.5,
                exit_threshold: -0.3,
                aggregation: AggregationMethod::WeightedSum,
                hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            },
            session: SessionConfig {
                no_new_entries_after: "15:30".to_string(),
                force_exit_by: "15:55".to_string(),
                avoid_first_minutes: 5,
                max_concurrent_positions: 3,
                max_capital_deployed_pct: 0.15,
            },
        };

        // should return None (fallback) for bad indicator
        let result = try_build_engine(&config, "SPY", 100_000.0);
        assert!(result.is_none());
    }

    #[test]
    fn valid_config_builds_engine() {
        use serde_json::json;
        use types::scoring::{AggregationMethod, ScoringConfig};
        use types::config::{SessionConfig, StrategyConfig};
        use types::market::Timescale;
        use types::test_fixtures::make_indicator_config;

        let config = StrategyConfig {
            schema_version: "0.1".to_string(),
            config_id: 1,
            created_at: chrono::Utc::now(),
            created_by: "test".to_string(),
            parent_config_id: None,
            tickers: vec!["SPY".to_string()],
            indicators: vec![
                make_indicator_config("rsi", "rsi_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
            ],
            actions: vec![types::ActionConfig {
                action_type: "score_threshold_entry".to_string(),
                instance_id: "e1".to_string(),
                phase: types::ActionPhase::Entry,
                enabled: true,
                priority: 0,
                params: vec![("entry_threshold", json!(0.5))]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            scoring: ScoringConfig {
                timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
                entry_threshold: 0.5,
                exit_threshold: -0.3,
                aggregation: AggregationMethod::WeightedSum,
                hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            },
            session: SessionConfig {
                no_new_entries_after: "15:30".to_string(),
                force_exit_by: "15:55".to_string(),
                avoid_first_minutes: 5,
                max_concurrent_positions: 3,
                max_capital_deployed_pct: 0.15,
            },
        };

        let result = try_build_engine(&config, "SPY", 100_000.0);
        assert!(result.is_some());
    }

    #[test]
    fn config_error_display() {
        let e = ConfigError::ValidationError("bad config".to_string());
        assert!(e.to_string().contains("bad config"));
    }
}
