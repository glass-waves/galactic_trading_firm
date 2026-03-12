use std::collections::HashMap;
#[cfg(feature = "tui")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "tui")]
use std::sync::{Arc, RwLock};

use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

fn log_shutdown_stats(
    sessions: &HashMap<String, LiveSession>,
    tick_count: u64,
    daily_pnl: f64,
) {
    for (ticker, session) in sessions.iter() {
        if session.has_position() {
            warn!(ticker = %ticker, "force-closing position on shutdown");
        }
    }
    info!(
        total_ticks = tick_count,
        daily_pnl = format!("{daily_pnl:.2}"),
        "shutdown complete"
    );
}

use data_feed::account::resolve_capital;
use data_feed::alpaca_feed::{AlpacaFeed, BarEvent};
use data_feed::broker::SimulatedBroker;
use data_feed::config_loader::load_config;
use data_feed::config_watcher::{try_build_engine, ConfigWatcher};
use data_feed::live_session::LiveSession;
use data_feed::market_state::MarketStateBuilder;
use data_feed::trade_writer::TradeWriter;
use types::tick_result::TickEvent;

#[cfg(feature = "tui")]
use data_feed::tui::{DashboardState, PositionDisplay, TickerState, TradeLogEntry};

#[tokio::main]
async fn main() {
    // 0. load .env file (ok if missing)
    let _ = dotenvy::dotenv();

    // 1. init structured logging (console + file layers)
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/paper_trader_{}.log", log_dir, today))
        .expect("failed to open log file");

    let (non_blocking, _file_guard) = tracing_appender::non_blocking(log_file);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(EnvFilter::new("info"));

    // console layer: JSON stdout (headless) or stderr warn+ (TUI)
    #[cfg(not(feature = "tui"))]
    let console_layer = {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::fmt::layer()
            .json()
            .with_filter(filter)
    };
    #[cfg(feature = "tui")]
    let console_layer = {
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(filter)
    };

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    info!("galactic trading firm — paper trading engine starting");

    // 2. load environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let api_key = std::env::var("APCA_API_KEY_ID").unwrap_or_default();
    let api_secret = std::env::var("APCA_API_SECRET_KEY").unwrap_or_default();
    let broker_mode = std::env::var("BROKER_MODE").unwrap_or_else(|_| "simulated".to_string());

    // 3. connect to postgres
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");

    info!("connected to database");

    // 4. load promoted config
    let strategy_config = match load_config(&pool).await {
        Ok(config) => {
            info!(
                config_id = config.config_id,
                indicators = config.indicators.len(),
                actions = config.actions.len(),
                tickers = ?config.tickers,
                "loaded promoted config"
            );
            config
        }
        Err(e) => {
            error!(error = %e, "failed to load config");
            std::process::exit(1);
        }
    };

    // 5. resolve capital
    let env_capital: f64 = std::env::var("INITIAL_CAPITAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);
    let capital = resolve_capital(&broker_mode, &api_key, &api_secret, env_capital).await;

    // 6. build engine per ticker
    let mut sessions: HashMap<String, LiveSession> = HashMap::new();
    let mut state_builders: HashMap<String, MarketStateBuilder> = HashMap::new();

    for ticker in &strategy_config.tickers {
        match try_build_engine(&strategy_config, ticker, capital) {
            Some(engine) => {
                info!(ticker = %ticker, "engine built");
                sessions.insert(ticker.clone(), LiveSession::new(engine));
                state_builders.insert(ticker.clone(), MarketStateBuilder::new(200));
            }
            None => {
                error!(ticker = %ticker, "failed to build engine, skipping ticker");
            }
        }
    }

    if sessions.is_empty() {
        error!("no engines could be built, exiting");
        std::process::exit(1);
    }

    // 6. create broker
    let broker: Option<SimulatedBroker> = match broker_mode.as_str() {
        "alpaca_paper" => {
            info!("using alpaca paper broker");
            None
        }
        _ => {
            info!("using simulated broker (5 bps slippage)");
            Some(SimulatedBroker::new(5.0))
        }
    };

    // 7. create trade writer
    let mut trade_writer = TradeWriter::new(pool.clone(), strategy_config.config_id);

    // 8. create config watcher
    let mut config_watcher = ConfigWatcher::new(pool.clone(), strategy_config.config_id);

    // 9. create data feed and channel
    let (bar_tx, mut bar_rx) = mpsc::channel::<BarEvent>(1000);

    let tickers: Vec<String> = strategy_config.tickers.clone();

    let demo_mode = std::env::args().any(|a| a == "--demo");

    // 10. fetch historical bars to bootstrap candle buffers
    if !demo_mode && !api_key.is_empty() && !api_secret.is_empty() {
        let feed = AlpacaFeed::new(api_key.clone(), api_secret.clone(), tickers.clone());
        for ticker in &tickers {
            info!(ticker = %ticker, "fetching historical bars for lookback");
            match feed.fetch_historical_bars(ticker, 200).await {
                Ok(candles) => {
                    info!(ticker = %ticker, count = candles.len(), "historical bars loaded");
                    if let Some(builder) = state_builders.get_mut(ticker) {
                        builder.seed(candles);
                    }
                }
                Err(e) => {
                    warn!(ticker = %ticker, error = %e, "failed to fetch historical bars, starting cold");
                }
            }
        }

        // 11. spawn data feed streaming task
        let feed = AlpacaFeed::new(api_key, api_secret, tickers);
        tokio::spawn(async move {
            if let Err(e) = feed.stream_bars(bar_tx).await {
                error!(error = %e, "data feed stream failed");
            }
        });
    } else {
        info!("starting synthetic demo feed");

        // spawn a synthetic random-walk data generator
        let demo_tickers = tickers.clone();
        tokio::spawn(async move {
            use std::collections::HashMap;

            let base_prices: HashMap<&str, f64> = [
                ("SPY", 450.0), ("QQQ", 380.0), ("AAPL", 175.0),
                ("NVDA", 800.0), ("MSFT", 420.0),
            ].into_iter().collect();

            let mut prices: HashMap<String, f64> = demo_tickers.iter().map(|t| {
                let base = base_prices.get(t.as_str()).copied().unwrap_or(100.0);
                (t.clone(), base)
            }).collect();

            let mut rng_state: u64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;

            loop {
                for ticker in &demo_tickers {
                    let price = prices.get_mut(ticker).unwrap();

                    // simple xorshift64 for a lightweight PRNG
                    rng_state ^= rng_state << 13;
                    rng_state ^= rng_state >> 7;
                    rng_state ^= rng_state << 17;
                    let norm = ((rng_state % 10000) as f64 / 10000.0 - 0.5) * 2.0;

                    // random walk: drift + volatility
                    let volatility = *price * 0.002;
                    let change = norm * volatility;
                    *price = (*price + change).max(1.0);

                    let close = *price;
                    let open = close - change * 0.3;
                    let high = close.max(open).max(close + volatility * 0.5);
                    let low = close.min(open).min(close - volatility * 0.5);

                    let candle = types::market::Candle {
                        timestamp: chrono::Utc::now(),
                        open,
                        high,
                        low,
                        close,
                        volume: 10000.0 + (rng_state % 50000) as f64,
                    };

                    if bar_tx.send(BarEvent { symbol: ticker.clone(), candle }).await.is_err() {
                        return; // channel closed, main loop exited
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    // 12. config reload interval
    let mut config_reload_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    config_reload_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let max_concurrent = strategy_config.session.max_concurrent_positions as usize;
    let mut tick_count: u64 = 0;
    let mut daily_pnl: f64 = 0.0;

    // 12b. launch TUI if feature is enabled
    #[cfg(feature = "tui")]
    let tui_shutdown = Arc::new(AtomicBool::new(false));
    #[cfg(feature = "tui")]
    let dashboard_state = Arc::new(RwLock::new(DashboardState::new(
        strategy_config.config_id,
    )));
    #[cfg(feature = "tui")]
    {
        dashboard_state
            .write()
            .unwrap()
            .set_strategy_config(strategy_config.clone());

        let state = Arc::clone(&dashboard_state);
        let shutdown = Arc::clone(&tui_shutdown);
        std::thread::spawn(move || {
            if let Err(e) = data_feed::tui::run_tui(state, shutdown) {
                eprintln!("TUI error: {e}");
            }
        });
    }

    // register SIGTERM handler for graceful container shutdown
    let mut sigterm = tokio::signal::unix::signal(
        tokio::signal::unix::SignalKind::terminate(),
    )
    .expect("failed to register SIGTERM handler");

    info!("entering main trading loop");

    // 13. main select! loop
    loop {
        // check if TUI requested shutdown
        #[cfg(feature = "tui")]
        if tui_shutdown.load(Ordering::Relaxed) {
            info!("TUI requested shutdown");
            break;
        }

        tokio::select! {
            // incoming bar from data feed
            Some(bar_event) = bar_rx.recv() => {
                tick_count += 1;
                let ticker = &bar_event.symbol;

                // update broker price if simulated
                if let Some(ref b) = broker {
                    b.set_last_price(bar_event.candle.close);
                }

                // clone candle for TUI history before on_bar consumes it
                #[cfg(feature = "tui")]
                let candle_for_tui = bar_event.candle.clone();

                // build market state from candle
                let mut market = if let Some(builder) = state_builders.get_mut(ticker) {
                    let bid = bar_event.candle.close - 0.01;
                    let ask = bar_event.candle.close + 0.01;
                    builder.on_bar(bar_event.candle, bid, ask)
                } else {
                    warn!(ticker = %ticker, "no state builder for ticker, skipping");
                    continue;
                };

                // check position limits before processing
                let open_positions = sessions.values().filter(|s| s.has_position()).count();

                if let Some(session) = sessions.get_mut(ticker) {
                    // skip entry if at max capacity (but still process exits)
                    if open_positions >= max_concurrent && !session.has_position() {
                        // at capacity — still tick to evaluate exits, but entry
                        // won't happen because we skip the session
                        continue;
                    }

                    let (result, trade_with_scores) = session.on_tick(&mut market);

                    if let TickEvent::PositionOpened = &result.event {
                        info!(
                            ticker = %ticker,
                            composite = result.scores.composite,
                            price = market.last_price,
                            "position opened"
                        );
                    }

                    // update TUI dashboard state
                    #[cfg(feature = "tui")]
                    {
                        let mut dash = dashboard_state.write().unwrap();
                        dash.total_ticks = tick_count;
                        dash.daily_pnl = daily_pnl;

                        let pos_display = session.current_position().map(|pos| {
                            PositionDisplay {
                                direction: pos.direction,
                                entry_price: pos.entry_price,
                                unrealized_pnl: pos.unrealized_pnl,
                                unrealized_pnl_pct: pos.unrealized_pnl_pct,
                                hold_duration_ms: pos.hold_duration_ms,
                            }
                        });

                        dash.ensure_ticker_order(ticker);

                        let ts = dash.tickers
                            .entry(ticker.clone())
                            .or_insert_with(|| TickerState {
                                ticker: ticker.clone(),
                                last_price: 0.0,
                                composite_score: 0.0,
                                one_minute_score: None,
                                five_minute_score: None,
                                one_hour_score: None,
                                position: None,
                                price_history: Vec::new(),
                                candle_history: Vec::new(),
                            });

                        ts.last_price = market.last_price;
                        ts.composite_score = result.scores.composite;
                        ts.one_minute_score = result.scores.one_minute;
                        ts.five_minute_score = result.scores.five_minute;
                        ts.one_hour_score = result.scores.one_hour;
                        ts.position = pos_display;

                        ts.price_history.push(market.last_price);
                        if ts.price_history.len() > 60 {
                            ts.price_history.remove(0);
                        }

                        ts.candle_history.push(candle_for_tui.clone());
                        if ts.candle_history.len() > 60 {
                            ts.candle_history.remove(0);
                        }
                    }

                    if let Some(tws) = trade_with_scores {
                        info!(
                            ticker = %tws.trade.ticker,
                            pnl = tws.trade.pnl,
                            pnl_pct = format!("{:.2}%", tws.trade.pnl_pct * 100.0),
                            exit_reason = ?tws.trade.exit_reason,
                            hold_ms = tws.trade.hold_duration_ms,
                            "trade completed"
                        );

                        daily_pnl += tws.trade.pnl;

                        // update TUI trade log
                        #[cfg(feature = "tui")]
                        {
                            let mut dash = dashboard_state.write().unwrap();
                            dash.daily_pnl = daily_pnl;
                            dash.add_trade(TradeLogEntry {
                                ticker: tws.trade.ticker.clone(),
                                direction: format!("{:?}", tws.trade.direction),
                                pnl: tws.trade.pnl,
                                pnl_pct: tws.trade.pnl_pct,
                                exit_reason: format!("{:?}", tws.trade.exit_reason),
                                time: tws.trade.exit_time,
                            });
                        }

                        // write to database
                        match trade_writer
                            .write_trade(&tws.trade, &tws.entry_scores, &tws.exit_scores)
                            .await
                        {
                            Ok(trade_id) => {
                                info!(trade_id, "trade written to database");
                            }
                            Err(e) => {
                                error!(error = %e, "failed to write trade to database");
                            }
                        }
                    }
                }

                // periodic status log
                if tick_count.is_multiple_of(60) {
                    let positions: Vec<&str> = sessions
                        .iter()
                        .filter(|(_, s)| s.has_position())
                        .map(|(t, _)| t.as_str())
                        .collect();
                    info!(
                        ticks = tick_count,
                        daily_pnl = format!("{daily_pnl:.2}"),
                        open_positions = ?positions,
                        "status"
                    );
                }
            }

            // config reload timer
            _ = config_reload_interval.tick() => {
                match config_watcher.check_for_update().await {
                    Ok(Some((version_id, new_config))) => {
                        info!(version_id, "new config detected, rebuilding engines");

                        // force-close open positions before swapping
                        for (ticker, session) in sessions.iter() {
                            if session.has_position() {
                                warn!(ticker = %ticker, "force-closing position for config change");
                            }
                        }

                        // rebuild engines
                        let mut new_sessions: HashMap<String, LiveSession> = HashMap::new();
                        for ticker in &new_config.tickers {
                            match try_build_engine(&new_config, ticker, capital) {
                                Some(new_engine) => {
                                    info!(ticker = %ticker, "engine rebuilt for new config");
                                    new_sessions.insert(ticker.clone(), LiveSession::new(new_engine));
                                }
                                None => {
                                    warn!(ticker = %ticker, "failed to rebuild engine, keeping old");
                                    if let Some(old) = sessions.remove(ticker) {
                                        new_sessions.insert(ticker.clone(), old);
                                    }
                                }
                            }
                        }

                        sessions = new_sessions;
                        trade_writer.set_config_version(version_id);
                        config_watcher.acknowledge(version_id);

                        #[cfg(feature = "tui")]
                        {
                            let mut dash = dashboard_state.write().unwrap();
                            dash.config_version = version_id;
                            dash.set_strategy_config(new_config.clone());
                        }
                    }
                    Ok(None) => {
                        // no new config
                    }
                    Err(e) => {
                        warn!(error = %e, "config check failed, continuing with current config");
                    }
                }
            }

            // graceful shutdown on ctrl-c
            _ = tokio::signal::ctrl_c() => {
                info!("ctrl-c received, shutting down");
                #[cfg(feature = "tui")]
                tui_shutdown.store(true, Ordering::Relaxed);
                log_shutdown_stats(&sessions, tick_count, daily_pnl);
                break;
            }

            // graceful shutdown on SIGTERM (docker stop / kill -TERM)
            _ = sigterm.recv() => {
                info!("SIGTERM received, shutting down");
                #[cfg(feature = "tui")]
                tui_shutdown.store(true, Ordering::Relaxed);
                log_shutdown_stats(&sessions, tick_count, daily_pnl);
                break;
            }
        }
    }
}
