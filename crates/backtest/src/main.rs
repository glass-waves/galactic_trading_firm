use std::collections::{HashMap, HashSet};
use std::fs;
use std::process;

use tracing_subscriber::{prelude::*, EnvFilter};

use backtest::alpaca_loader::{build_backtest_data, fetch_bars_range, market_hours_utc};
use backtest::config_loader::{load_promoted_config_with_id, write_backtest_trades};
use backtest::replay::{BacktestConfig, BacktestCostConfig, BacktestData};
use backtest::{compute_metrics, load_candles_from_csv, run_backtest, to_json};
use chrono::{Duration, NaiveDate};
use engine::TradeRecord;
use types::action::{ActionConfig, ActionPhase};
use types::config::StrategyConfig;
use types::indicator::IndicatorConfig;
use types::market::Timescale;

/// CLI overrides for testing new features without modifying the DB config.
#[derive(Default)]
struct ConfigOverrides {
    entry_cooldown_ms: Option<i64>,
    max_daily_loss_pct: Option<f64>,
    profit_extension_ms: Option<i64>,
    loss_reduction_ms: Option<i64>,
    score_scaled_sizing: bool,
    score_scaled_min: Option<f64>,
    score_scaled_max: Option<f64>,
    add_rvol_weight: Option<f64>,
}

impl ConfigOverrides {
    fn any_active(&self) -> bool {
        self.entry_cooldown_ms.is_some()
            || self.max_daily_loss_pct.is_some()
            || self.profit_extension_ms.is_some()
            || self.loss_reduction_ms.is_some()
            || self.score_scaled_sizing
            || self.score_scaled_min.is_some()
            || self.score_scaled_max.is_some()
            || self.add_rvol_weight.is_some()
    }

    fn apply(&self, config: &mut StrategyConfig) {
        if let Some(cooldown) = self.entry_cooldown_ms {
            config.session.entry_cooldown_ms = cooldown;
        }
        if let Some(loss_pct) = self.max_daily_loss_pct {
            config.session.max_daily_loss_pct = Some(loss_pct);
        }
        if let Some(ext) = self.profit_extension_ms {
            for action in &mut config.actions {
                if action.action_type == "max_hold_timeout" {
                    action.params.insert("profit_extension_ms".to_string(), serde_json::json!(ext));
                }
            }
        }
        if let Some(red) = self.loss_reduction_ms {
            for action in &mut config.actions {
                if action.action_type == "max_hold_timeout" {
                    action.params.insert("loss_reduction_ms".to_string(), serde_json::json!(red));
                }
            }
        }
        if self.score_scaled_sizing {
            let min = self.score_scaled_min.unwrap_or(0.03);
            let max = self.score_scaled_max.unwrap_or(0.06);
            let mut params = HashMap::new();
            params.insert("min_fraction".to_string(), serde_json::json!(min));
            params.insert("max_fraction".to_string(), serde_json::json!(max));
            params.insert("entry_threshold".to_string(), serde_json::json!(0.58));
            // disable existing sizing actions
            for action in &mut config.actions {
                if action.phase == ActionPhase::Sizing {
                    action.enabled = false;
                }
            }
            config.actions.push(ActionConfig {
                action_type: "score_scaled".to_string(),
                instance_id: "sizing_score".to_string(),
                phase: ActionPhase::Sizing,
                enabled: true,
                priority: 0,
                params,
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override for testing".to_string()),
            });
        }
        if let Some(weight) = self.add_rvol_weight {
            let mut params = HashMap::new();
            params.insert("lookback_period".to_string(), serde_json::json!(20));
            params.insert("high_threshold".to_string(), serde_json::json!(1.5));
            params.insert("low_threshold".to_string(), serde_json::json!(0.5));
            config.indicators.push(IndicatorConfig {
                indicator_type: "relative_volume".to_string(),
                instance_id: "rvol_20_5min".to_string(),
                timescale: Timescale::FiveMinute,
                enabled: true,
                weight,
                params,
                last_modified_by: Some("backtest_cli".to_string()),
                last_modified_at: None,
                modification_reason: Some("CLI override for testing".to_string()),
            });
        }
    }
}

fn parse_overrides(args: &[String]) -> ConfigOverrides {
    ConfigOverrides {
        entry_cooldown_ms: get_arg(args, "--entry-cooldown-ms").and_then(|s| s.parse().ok()),
        max_daily_loss_pct: get_arg(args, "--max-daily-loss-pct").and_then(|s| s.parse().ok()),
        profit_extension_ms: get_arg(args, "--profit-extension-ms").and_then(|s| s.parse().ok()),
        loss_reduction_ms: get_arg(args, "--loss-reduction-ms").and_then(|s| s.parse().ok()),
        score_scaled_sizing: args.iter().any(|a| a == "--score-scaled-sizing"),
        score_scaled_min: get_arg(args, "--score-scaled-min").and_then(|s| s.parse().ok()),
        score_scaled_max: get_arg(args, "--score-scaled-max").and_then(|s| s.parse().ok()),
        add_rvol_weight: get_arg(args, "--add-rvol").and_then(|s| s.parse().ok()),
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let capital: f64 = get_arg(&args, "--capital")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);

    let cost_config = parse_cost_config(&args);
    let overrides = parse_overrides(&args);

    let verbose = args.iter().any(|a| a == "--verbose");
    let output_equity = args.iter().any(|a| a == "--output-equity");

    if let Some(date_str) = get_arg(&args, "--date") {
        let lookback_days: i64 = get_arg(&args, "--lookback-days")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let write_db = args.iter().any(|a| a == "--write-db");
        run_date_mode(&date_str, lookback_days, write_db, capital, &cost_config, verbose, output_equity, &overrides).await;
    } else {
        run_legacy_mode(&args, capital, &cost_config);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_date_mode(date_str: &str, lookback_days: i64, write_db: bool, capital: f64, cost_config: &Option<BacktestCostConfig>, verbose: bool, output_equity: bool, overrides: &ConfigOverrides) {
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
    let (mut config, config_version_id) = match load_promoted_config_with_id(&pool).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    // apply CLI overrides for testing
    if overrides.any_active() {
        overrides.apply(&mut config);
    }

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
        println!("backtest: {} (lookback from {})", date, lookback_start);
    } else {
        println!("backtest: {}", date);
    }
    if let Some(ref costs) = cost_config {
        println!(
            "costs: slippage={:.1}bps  spread=${:.4}  commission=${:.4}/sh",
            costs.slippage_bps, costs.half_spread, costs.commission_per_share,
        );
    }
    println!();

    let mut total_pnl = 0.0;
    let mut ticker_results: Vec<(String, f64, usize, Vec<TradeRecord>)> = Vec::new();

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
            cost_config: cost_config.clone(),
            session_config: Some(config.session.clone()),
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
                    &[],
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

                ticker_results.push((ticker.clone(), pnl, trades, filtered_trades));
            }
            Err(e) => {
                eprintln!("  {:<6} error: {}", ticker, e);
            }
        }
    }

    // print summary
    for (ticker, pnl, trades, trade_records) in &ticker_results {
        let sign = if *pnl >= 0.0 { "+" } else { "-" };
        println!("  {:<6} {}${:.2}  ({} trades)", ticker, sign, pnl.abs(), trades);

        if verbose {
            for (idx, t) in trade_records.iter().enumerate() {
                let dir = match t.direction {
                    types::action::TradeDirection::Long => "LONG",
                    types::action::TradeDirection::Short => "SHORT",
                };
                let entry_et = t.entry_time.with_timezone(&chrono_tz::US::Eastern);
                let exit_et = t.exit_time.with_timezone(&chrono_tz::US::Eastern);
                let pnl_sign = if t.pnl >= 0.0 { "+" } else { "-" };
                println!(
                    "         #{:<2} {:<5} entry {}  ${:.2}  exit {}  ${:.2}  {}${:.2}  ({:?})",
                    idx + 1,
                    dir,
                    entry_et.format("%H:%M"),
                    t.entry_price,
                    exit_et.format("%H:%M"),
                    t.exit_price,
                    pnl_sign,
                    t.pnl.abs(),
                    t.exit_reason,
                );
            }
        }
    }

    if !ticker_results.is_empty() {
        let total_trades: usize = ticker_results.iter().map(|(_, _, t, _)| t).sum();
        let label = if total_pnl >= 0.0 { "profit" } else { "loss" };
        let sign = if total_pnl >= 0.0 { "+" } else { "-" };
        println!(
            "\n  {:<6} {}${:.2}  {}  ({} trades)",
            "total", sign, total_pnl.abs(), label, total_trades
        );
    }

    if output_equity {
        let ending_equity = capital + total_pnl;
        println!("ENDING_EQUITY={:.2}", ending_equity);
    }
}

fn run_legacy_mode(args: &[String], capital: f64, cost_config: &Option<BacktestCostConfig>) {
    let config_path = get_arg(args, "--config");
    let data_path = get_arg(args, "--data");
    let ticker = get_arg(args, "--ticker");

    if config_path.is_none() || data_path.is_none() || ticker.is_none() {
        eprintln!("usage: backtest --date <YYYY-MM-DD> [options]");
        eprintln!("       backtest --config <path> --data <path> --ticker <name> [options]");
        eprintln!();
        eprintln!("cost model options:");
        eprintln!("  --slippage-bps <N>          slippage in basis points (e.g. 2.0)");
        eprintln!("  --half-spread <N>           half bid-ask spread in dollars (e.g. 0.005)");
        eprintln!("  --commission-per-share <N>  commission per share (e.g. 0.0)");
        eprintln!("  --sec-fee-per-million <N>   SEC fee per million sell-side (e.g. 20.60)");
        eprintln!("  --finra-taf-per-share <N>   FINRA TAF per share sell-side (e.g. 0.000195)");
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
        cost_config: cost_config.clone(),
        session_config: Some(strategy_config.session),
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

fn parse_cost_config(args: &[String]) -> Option<BacktestCostConfig> {
    let slippage_bps: Option<f64> = get_arg(args, "--slippage-bps").and_then(|s| s.parse().ok());
    let half_spread: Option<f64> = get_arg(args, "--half-spread").and_then(|s| s.parse().ok());
    let commission: Option<f64> = get_arg(args, "--commission-per-share").and_then(|s| s.parse().ok());
    let sec_fee: Option<f64> = get_arg(args, "--sec-fee-per-million").and_then(|s| s.parse().ok());
    let finra_taf: Option<f64> = get_arg(args, "--finra-taf-per-share").and_then(|s| s.parse().ok());

    if slippage_bps.is_some() || half_spread.is_some() || commission.is_some() || sec_fee.is_some() || finra_taf.is_some() {
        Some(BacktestCostConfig {
            slippage_bps: slippage_bps.unwrap_or(0.0),
            half_spread: half_spread.unwrap_or(0.0),
            commission_per_share: commission.unwrap_or(0.0),
            sec_fee_per_million: sec_fee.unwrap_or(0.0),
            finra_taf_per_share: finra_taf.unwrap_or(0.0),
        })
    } else {
        None
    }
}

fn get_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}
