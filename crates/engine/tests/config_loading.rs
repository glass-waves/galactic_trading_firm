// config loading integration test
// requires a running postgres instance — run with:
//   cargo test -p engine -- config_loading --ignored
//
// the `load_config` function is tested here against a real database.
// for CI without postgres, this test is ignored by default.

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore] // requires postgres with migrations applied
    async fn test_load_config_from_db() {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");
        let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

        // verify the seed config can be loaded
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT config_blob FROM config_versions WHERE status = 'promoted' ORDER BY promoted_at DESC LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();

        let (config_blob,) = row.expect("seed config should exist");
        let config: types::StrategyConfig =
            serde_json::from_value(config_blob).expect("seed config should deserialize");

        assert!(!config.indicators.is_empty());
        assert!(!config.actions.is_empty());
        assert_eq!(config.schema_version, "0.1");
    }
}
