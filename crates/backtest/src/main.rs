use std::collections::HashMap;
use std::fs;
use std::process;

use backtest::{run_backtest, load_candles_from_csv, to_json};
use backtest::replay::{BacktestConfig, BacktestData};
use types::config::StrategyConfig;
use types::market::Timescale;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let config_path = get_arg(&args, "--config");
    let data_path = get_arg(&args, "--data");
    let ticker = get_arg(&args, "--ticker");

    if config_path.is_none() || data_path.is_none() || ticker.is_none() {
        eprintln!("usage: backtest --config <path> --data <path> --ticker <name>");
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
        initial_capital: 100_000.0,
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
        Ok(result) => {
            match to_json(&result) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("error serializing result: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("backtest error: {}", e);
            process::exit(1);
        }
    }
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
