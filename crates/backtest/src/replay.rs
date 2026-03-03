use std::collections::HashMap;
use std::io::Read;

use chrono::DateTime;

use engine::TradingEngine;
use types::action::ActionConfig;
use types::indicator::IndicatorConfig;
use types::market::{Candle, MarketState, Timescale};
use types::scoring::{ScoringConfig, TimescaleScores};
use types::tick_result::TickEvent;

use actions::{build_actions, default_action_registry};
use indicators::{build_indicators, default_indicator_registry};

use crate::report::{compute_metrics, BacktestResult};

/// configuration for a backtest run.
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub ticker: String,
    pub initial_capital: f64,
    pub indicator_configs: Vec<IndicatorConfig>,
    pub action_configs: Vec<ActionConfig>,
    pub scoring_config: ScoringConfig,
}

/// historical candle data for replay, keyed by timescale.
#[derive(Debug, Clone)]
pub struct BacktestData {
    pub candles: HashMap<Timescale, Vec<Candle>>,
    /// which timescale drives the tick loop. each candle in this
    /// timescale produces one tick.
    pub primary_timescale: Timescale,
}

/// run a backtest: replay historical data through a trading engine and collect results.
pub fn run_backtest(config: &BacktestConfig, data: &BacktestData) -> Result<BacktestResult, String> {
    let ind_reg = default_indicator_registry();
    let indicators = build_indicators(&config.indicator_configs, &ind_reg)?;

    let act_reg = default_action_registry();
    let actions = build_actions(&config.action_configs, &act_reg)?;

    let mut engine = TradingEngine::new(
        indicators,
        config.indicator_configs.clone(),
        config.scoring_config.clone(),
        actions.entry,
        actions.monitor,
        actions.exit,
        actions.sizing,
        config.ticker.clone(),
        config.initial_capital,
    );

    let primary_candles = data
        .candles
        .get(&data.primary_timescale)
        .ok_or_else(|| format!("no candles for primary timescale {:?}", data.primary_timescale))?;

    if primary_candles.is_empty() {
        return Err("no candle data provided".to_string());
    }

    let start_time = primary_candles.first().unwrap().timestamp;
    let end_time = primary_candles.last().unwrap().timestamp;

    // track entry scores per position so we can pair them with exit scores
    let mut pending_entry_scores: Option<TimescaleScores> = None;
    let mut trade_scores: Vec<(TimescaleScores, TimescaleScores)> = Vec::new();

    // replay tick by tick: at each tick, provide candles up to that point
    for i in 1..=primary_candles.len() {
        let mut candle_map: HashMap<Timescale, Vec<Candle>> = HashMap::new();

        // for the primary timescale, use a growing window
        candle_map.insert(data.primary_timescale, primary_candles[..i].to_vec());

        // for other timescales, include all candles up to the current timestamp
        let current_ts = primary_candles[i - 1].timestamp;
        for (&ts, candles) in &data.candles {
            if ts == data.primary_timescale {
                continue;
            }
            let relevant: Vec<Candle> = candles
                .iter()
                .filter(|c| c.timestamp <= current_ts)
                .cloned()
                .collect();
            if !relevant.is_empty() {
                candle_map.insert(ts, relevant);
            }
        }

        let last_candle = &primary_candles[i - 1];
        let market = MarketState {
            last_price: last_candle.close,
            bid: last_candle.close - 0.01,
            ask: last_candle.close + 0.01,
            timestamp: last_candle.timestamp,
            candles: candle_map,
            spread: 0.02,
            session_vwap: last_candle.close,
            session_volume: 1_000_000.0,
            position_context: None,
            session_progress: None,
        };

        let result = engine.on_tick(&market);

        match &result.event {
            TickEvent::PositionOpened => {
                pending_entry_scores = Some(result.scores.clone());
            }
            TickEvent::PositionClosed => {
                let entry_scores = pending_entry_scores
                    .take()
                    .unwrap_or_default();
                trade_scores.push((entry_scores, result.scores.clone()));
            }
            TickEvent::Nothing => {}
        }
    }

    let trades = engine.completed_trades().to_vec();
    let (metrics, equity_curve) =
        compute_metrics(&trades, config.initial_capital, start_time, end_time);

    Ok(BacktestResult {
        config_id: "backtest".to_string(),
        ticker: config.ticker.clone(),
        start_time,
        end_time,
        initial_capital: config.initial_capital,
        trades,
        trade_scores,
        equity_curve,
        metrics,
    })
}

/// parse candle data from CSV format: timestamp_epoch,open,high,low,close,volume
pub fn load_candles_from_csv<R: Read>(reader: R) -> Result<Vec<Candle>, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);

    let mut candles = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("csv parse error: {}", e))?;
        if record.len() < 6 {
            return Err(format!("expected 6 fields, got {}", record.len()));
        }

        let ts_epoch: i64 = record[0]
            .parse()
            .map_err(|e| format!("bad timestamp: {}", e))?;
        let timestamp = DateTime::from_timestamp(ts_epoch, 0)
            .ok_or_else(|| format!("invalid timestamp: {}", ts_epoch))?;

        let open: f64 = record[1].parse().map_err(|e| format!("bad open: {}", e))?;
        let high: f64 = record[2].parse().map_err(|e| format!("bad high: {}", e))?;
        let low: f64 = record[3].parse().map_err(|e| format!("bad low: {}", e))?;
        let close: f64 = record[4].parse().map_err(|e| format!("bad close: {}", e))?;
        let volume: f64 = record[5]
            .parse()
            .map_err(|e| format!("bad volume: {}", e))?;

        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        });
    }

    Ok(candles)
}
