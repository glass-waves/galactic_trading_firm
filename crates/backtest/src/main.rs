use std::collections::{HashMap, HashSet};
use std::fs;
use std::process;

use tracing_subscriber::{prelude::*, EnvFilter};

use backtest::alpaca_loader::{build_backtest_data, fetch_bars_range, market_hours_utc};
use backtest::config_loader::{load_promoted_config_with_id, write_backtest_trades};
use backtest::replay::{BacktestConfig, BacktestData};
use backtest::{compute_metrics, load_candles_from_csv, run_backtest, to_json};
use chrono::{Duration, NaiveDate};
use types::config::StrategyConfig;
use types::market::Timescale;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let capital: f64 = get_arg(&args, "--capital")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);

    if let Some(date_str) = get_arg(&args, "--date") {
        let lookback_days: i64 = get_arg(&args, "--lookback-days")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let write_db = args.iter().any(|a| a == "--write-db");
        run_date_mode(&date_str, lookback_days, write_db, capital).await;
    } else {
        run_legacy_mode(&args, capital);
    }
}

async fn run_date_mode(date_str: &str, lookback_days: i64, write_db: bool, capital: f64) {
    dotenvy::dotenv().ok();

    // file + console layered logging
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/backtest_{}.log", log_dir, date_str))
        .expect("failed to open log file");

    // guard must live until end of function to flush buffered log writes
    let (non_blocking, _log_flush_guard) = tracing_appender::non_blocking(log_file);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(EnvFilter::new("info"));

    let console_layer = tracing_subscriber::fmt::layer()
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("backtest=info".parse().unwrap()),
        );

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: invalid date '{}': {}", date_str, e);
            process::exit(1);
        }
    };

    // connect to postgres
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("error: DATABASE_URL not set");
        process::exit(1);
    });

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("error: failed to connect to database: {e}");
            process::exit(1);
        });

    // load promoted config
    let (config, config_version_id) = match load_promoted_config_with_id(&pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    // alpaca credentials
    let api_key = std::env::var("APCA_API_KEY_ID").unwrap_or_else(|_| {
        eprintln!("error: APCA_API_KEY_ID not set");
        process::exit(1);
    });
    let api_secret = std::env::var("APCA_API_SECRET_KEY").unwrap_or_else(|_| {
        eprintln!("error: APCA_API_SECRET_KEY not set");
        process::exit(1);
    });

    // collect all unique timescales from indicators + scoring weights
    let required_timescales = collect_timescales(&config);

    // compute lookback range for indicator warmup
    let lookback_start = date - Duration::days(lookback_days);

    // target date market hours for filtering results
    let (target_open_utc, target_close_utc) = match market_hours_utc(date) {
        Ok(bounds) => bounds,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    if lookback_days > 0 {
        println!("backtest: {} (lookback from {})\n", date, lookback_start);
    } else {
        println!("backtest: {}\n", date);
    }

    let mut total_pnl = 0.0;
    let mut ticker_results: Vec<(String, f64, usize)> = Vec::new();

    for ticker in &config.tickers {
        let candles = match fetch_bars_range(
            &api_key,
            &api_secret,
            ticker,
            lookback_start,
            date,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  {:<6} error: {}", ticker, e);
                continue;
            }
        };

        if candles.is_empty() {
            eprintln!("  {:<6} no data", ticker);
            continue;
        }

        let backtest_data = build_backtest_data(candles, &required_timescales);

        let backtest_config = BacktestConfig {
            ticker: ticker.clone(),
            initial_capital: capital,
            indicator_configs: config.indicators.clone(),
            action_configs: config.actions.clone(),
            scoring_config: config.scoring.clone(),
        };

        match run_backtest(&backtest_config, &backtest_data) {
            Ok(result) => {
                // filter trades to target date's market hours only,
                // keeping scores parallel with their trades
                let mut filtered_trades = Vec::new();
                let mut filtered_scores = Vec::new();
                for (i, trade) in result.trades.into_iter().enumerate() {
                    if trade.entry_time >= target_open_utc && trade.entry_time <= target_close_utc {
                        filtered_trades.push(trade);
                        if let Some(scores) = result.trade_scores.get(i) {
                            filtered_scores.push(scores.clone());
                        }
                    }
                }

                let (metrics, _equity_curve) = compute_metrics(
                    &filtered_trades,
                    backtest_config.initial_capital,
                    target_open_utc,
                    target_close_utc,
                );

                let pnl = metrics.total_pnl;
                let trades = filtered_trades.len();
                total_pnl += pnl;

                if write_db && !filtered_trades.is_empty() {
                    match write_backtest_trades(
                        &pool,
                        config_version_id,
                        &filtered_trades,
                        &filtered_scores,
                    )
                    .await
                    {
                        Ok(n) => println!("  {:<6} wrote {} trades to db", ticker, n),
                        Err(e) => eprintln!("  {:<6} db write error: {}", ticker, e),
                    }
                }

                ticker_results.push((ticker.clone(), pnl, trades));
            }
            Err(e) => {
                eprintln!("  {:<6} error: {}", ticker, e);
            }
        }
    }

    // print summary
    for (ticker, pnl, trades) in &ticker_results {
        let sign = if *pnl >= 0.0 { "+" } else { "-" };
        println!("  {:<6} {}${:.2}  ({} trades)", ticker, sign, pnl.abs(), trades);
    }

    if !ticker_results.is_empty() {
        let total_trades: usize = ticker_results.iter().map(|(_, _, t)| t).sum();
        let label = if total_pnl >= 0.0 { "profit" } else { "loss" };
        let sign = if total_pnl >= 0.0 { "+" } else { "-" };
        println!(
            "\n  {:<6} {}${:.2}  {}  ({} trades)",
            "total", sign, total_pnl.abs(), label, total_trades
        );
    }
}

fn run_legacy_mode(args: &[String], capital: f64) {
    let config_path = get_arg(args, "--config");
    let data_path = get_arg(args, "--data");
    let ticker = get_arg(args, "--ticker");

    if config_path.is_none() || data_path.is_none() || ticker.is_none() {
        eprintln!("usage: backtest --date <YYYY-MM-DD>");
        eprintln!("       backtest --config <path> --data <path> --ticker <name>");
        process::exit(1);
    }

    let config_path = config_path.unwrap();
    let data_path = data_path.unwrap();
    let ticker = ticker.unwrap();

    // load strategy config from JSON
    let config_json = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading config file {}: {}", config_path, e);
            process::exit(1);
        }
    };

    let strategy_config: StrategyConfig = match serde_json::from_str(&config_json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error parsing config JSON: {}", e);
            process::exit(1);
        }
    };

    // load candle data from CSV
    let csv_data = match fs::read_to_string(&data_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading data file {}: {}", data_path, e);
            process::exit(1);
        }
    };

    let candles = match load_candles_from_csv(csv_data.as_bytes()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error parsing candle CSV: {}", e);
            process::exit(1);
        }
    };

    if candles.is_empty() {
        eprintln!("error: no candle data found in {}", data_path);
        process::exit(1);
    }

    // build backtest config from strategy config
    let backtest_config = BacktestConfig {
        ticker: ticker.clone(),
        initial_capital: capital,
        indicator_configs: strategy_config.indicators,
        action_configs: strategy_config.actions,
        scoring_config: strategy_config.scoring,
    };

    let mut candle_map = HashMap::new();
    candle_map.insert(Timescale::FiveMinute, candles);

    let backtest_data = BacktestData {
        candles: candle_map,
        primary_timescale: Timescale::FiveMinute,
    };

    // run backtest
    match run_backtest(&backtest_config, &backtest_data) {
        Ok(result) => match to_json(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!("error serializing result: {}", e);
                process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("backtest error: {}", e);
            process::exit(1);
        }
    }
}

fn collect_timescales(config: &StrategyConfig) -> HashSet<Timescale> {
    let mut timescales = HashSet::new();
    for ind in &config.indicators {
        timescales.insert(ind.timescale);
    }
    for ts in config.scoring.timescale_weights.keys() {
        timescales.insert(*ts);
    }
    timescales
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
