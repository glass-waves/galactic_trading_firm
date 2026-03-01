mod config;

use config::load_config;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    match load_config(&pool).await {
        Ok(config) => {
            println!(
                "loaded config v{} with {} indicators and {} actions",
                config.schema_version,
                config.indicators.len(),
                config.actions.len(),
            );
            println!("use `cargo run -p data_feed` for the live paper trading engine");
        }
        Err(e) => {
            eprintln!("failed to load config: {e}");
            std::process::exit(1);
        }
    }
}
