use std::fmt;

use types::StrategyConfig;

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    DatabaseError(sqlx::Error),
    DeserializationError(serde_json::Error),
    ValidationError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound => write!(f, "no promoted config found"),
            ConfigError::DatabaseError(e) => write!(f, "database error: {e}"),
            ConfigError::DeserializationError(e) => write!(f, "config deserialization error: {e}"),
            ConfigError::ValidationError(msg) => write!(f, "config validation error: {msg}"),
        }
    }
}

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

/// load the latest promoted config. returns the `config_versions.id` row id
/// alongside the parsed blob — callers must use the row id (not the blob's
/// `config_id`) for trade attribution and hot-reload tracking.
pub async fn load_config(pool: &sqlx::PgPool) -> Result<(i64, StrategyConfig), ConfigError> {
    let row: Option<(i64, serde_json::Value)> = sqlx::query_as(
        "SELECT id, config_blob FROM config_versions WHERE status = 'promoted' \
         ORDER BY promoted_at DESC NULLS LAST, id DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let (row_id, config_blob) = row.ok_or(ConfigError::NotFound)?;
    let config: StrategyConfig = serde_json::from_value(config_blob)?;

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

    Ok((row_id, config))
}
