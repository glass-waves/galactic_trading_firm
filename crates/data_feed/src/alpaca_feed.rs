use std::fmt;

use chrono::Utc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use types::market::Candle;

use crate::session_clock::is_regular_hours;

/// a bar event received from the data feed.
#[derive(Debug, Clone)]
pub struct BarEvent {
    pub symbol: String,
    pub candle: Candle,
}

/// errors from the data feed.
#[derive(Debug)]
pub enum DataFeedError {
    ConnectionFailed(String),
    StreamError(String),
    HistoricalFetchError(String),
}

impl fmt::Display for DataFeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFeedError::ConnectionFailed(msg) => write!(f, "connection failed: {msg}"),
            DataFeedError::StreamError(msg) => write!(f, "stream error: {msg}"),
            DataFeedError::HistoricalFetchError(msg) => write!(f, "historical fetch error: {msg}"),
        }
    }
}

impl std::error::Error for DataFeedError {}

/// convert an apca bar Num to f64, defaulting to 0.0 on failure.
fn num_to_f64(n: &num_decimal::Num) -> f64 {
    n.to_f64().unwrap_or(0.0)
}

/// alpaca data feed using the apca crate for websocket bar streaming
/// and REST historical bar fetching.
pub struct AlpacaFeed {
    api_key: String,
    api_secret: String,
    symbols: Vec<String>,
}

impl AlpacaFeed {
    pub fn new(api_key: String, api_secret: String, symbols: Vec<String>) -> Self {
        Self {
            api_key,
            api_secret,
            symbols,
        }
    }

    /// stream real-time 1-minute bars via websocket.
    /// sends BarEvent through the provided channel.
    /// reconnects with exponential backoff on disconnect.
    pub async fn stream_bars(
        &self,
        tx: mpsc::Sender<BarEvent>,
    ) -> Result<(), DataFeedError> {
        use apca::ApiInfo;
        use apca::Client;

        let api_info = ApiInfo::from_parts(
            "https://paper-api.alpaca.markets",
            &self.api_key,
            &self.api_secret,
        )
        .map_err(|e| DataFeedError::ConnectionFailed(format!("{e}")))?;

        let client = Client::new(api_info);

        let mut backoff_ms = 1000u64;
        let max_backoff_ms = 30_000u64;

        loop {
            info!(symbols = ?self.symbols, "connecting to alpaca data stream");
            let connected_at = std::time::Instant::now();

            let outcome = self.run_stream(&client, &tx).await;

            // a connection that lived a while earns a fresh backoff schedule
            if connected_at.elapsed() > std::time::Duration::from_secs(60) {
                backoff_ms = 1000;
            }

            match outcome {
                Ok(StreamEnd::ChannelClosed) => {
                    info!("bar channel closed, stopping stream");
                    break Ok(());
                }
                Ok(StreamEnd::ServerClosed) => {
                    // alpaca closes idle/rotated connections without an error frame.
                    // this is a disconnect, not a shutdown — reconnect.
                    warn!(backoff_ms, "stream closed by server, reconnecting");
                }
                Err(e) => {
                    warn!(error = %e, backoff_ms, "stream disconnected, reconnecting");
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
        }
    }

    async fn run_stream(
        &self,
        client: &apca::Client,
        tx: &mpsc::Sender<BarEvent>,
    ) -> Result<StreamEnd, DataFeedError> {
        use apca::data::v2::stream;
        use futures::StreamExt;

        let (mut stream, mut subscription) = client
            .subscribe::<stream::RealtimeData<stream::IEX>>()
            .await
            .map_err(|e| DataFeedError::ConnectionFailed(format!("{e}")))?;

        // subscribe to bars for our symbols
        let symbols: Vec<String> = self.symbols.clone();
        let mut market_data = stream::MarketData::default();
        market_data.set_bars(symbols.clone());

        let subscribe_fut = subscription.subscribe(&market_data);
        let subscribe_fut = std::pin::pin!(subscribe_fut);
        stream::drive(subscribe_fut, &mut stream)
            .await
            .map_err(|e| DataFeedError::StreamError(format!("subscribe drive error: {e:?}")))?
            .map_err(|e| DataFeedError::StreamError(format!("subscribe error: {e}")))?
            .map_err(|e| DataFeedError::StreamError(format!("subscribe result error: {e}")))?;

        info!(symbols = ?symbols, "subscribed to bar stream");

        // process incoming messages
        while let Some(result) = stream.next().await {
            match result {
                Ok(Ok(stream::Data::Bar(bar))) => {
                    let candle = Candle {
                        timestamp: bar.timestamp,
                        open: num_to_f64(&bar.open_price),
                        high: num_to_f64(&bar.high_price),
                        low: num_to_f64(&bar.low_price),
                        close: num_to_f64(&bar.close_price),
                        volume: num_to_f64(&bar.volume),
                    };

                    let event = BarEvent {
                        symbol: bar.symbol.clone(),
                        candle,
                    };

                    if tx.send(event).await.is_err() {
                        return Ok(StreamEnd::ChannelClosed);
                    }
                }
                Ok(Ok(_)) => {
                    // quote or trade — ignore
                }
                Ok(Err(e)) => {
                    warn!(error = ?e, "stream data error");
                }
                Err(e) => {
                    error!(error = ?e, "stream transport error");
                    return Err(DataFeedError::StreamError(format!("{e:?}")));
                }
            }
        }

        Ok(StreamEnd::ServerClosed)
    }

    /// fetch historical 1-minute bars covering the last `lookback_days` calendar
    /// days, restricted to regular trading hours, oldest first. paginates the
    /// alpaca response so multi-day windows are complete. used to warm the
    /// 5-minute and hourly candle windows at startup (hourly indicators need
    /// ~21 hourly candles ≈ 3+ trading days).
    pub async fn fetch_historical_bars(
        &self,
        symbol: &str,
        lookback_days: i64,
    ) -> Result<Vec<Candle>, DataFeedError> {
        use apca::ApiInfo;
        use apca::Client;

        let api_info = ApiInfo::from_parts(
            "https://paper-api.alpaca.markets",
            &self.api_key,
            &self.api_secret,
        )
        .map_err(|e| DataFeedError::HistoricalFetchError(format!("{e}")))?;

        let client = Client::new(api_info);

        // warm up on the SAME feed the live stream uses (IEX). before 2026-09-24 this
        // fetched consolidated (SIP) bars: IEX carries ~3 % of consolidated volume for
        // these names, so the indicator windows mixed two volume scales and VPIN (bucket
        // size = average volume) was a different quantity live than in the replay, which
        // is what the v17 threshold was fitted on. IEX history has no 15-minute lag.
        let end = Utc::now();
        let start = end - chrono::Duration::days(lookback_days);

        match Self::fetch_bars_paged(&client, symbol, start, end, Some(apca::data::v2::Feed::IEX)).await {
            Ok(c) => Ok(c),
            Err(iex_err) => {
                warn!(symbol, error = %iex_err, "IEX historical bars unavailable, falling back to SIP (lagged 16 min) — volume scale will not match the live stream until the window rolls");
                let end = Utc::now() - chrono::Duration::minutes(16);
                Self::fetch_bars_paged(&client, symbol, end - chrono::Duration::days(lookback_days), end, None).await
            }
        }
    }

    async fn fetch_bars_paged(
        client: &apca::Client,
        symbol: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
        feed: Option<apca::data::v2::Feed>,
    ) -> Result<Vec<Candle>, DataFeedError> {
        use apca::data::v2::bars;

        let mut candles: Vec<Candle> = Vec::new();
        let mut page_token: Option<String> = None;
        for _page in 0..20 {
            let req = bars::ListReqInit {
                limit: Some(10_000),
                page_token: page_token.take(),
                feed,
                ..Default::default()
            }
            .init(symbol, start, end, bars::TimeFrame::OneMinute);

            let response = client
                .issue::<bars::List>(&req)
                .await
                .map_err(|e| DataFeedError::HistoricalFetchError(format!("{e:?}")))?;

            candles.extend(response.bars.iter().map(|bar| Candle {
                timestamp: bar.time,
                open: num_to_f64(&bar.open),
                high: num_to_f64(&bar.high),
                low: num_to_f64(&bar.low),
                close: num_to_f64(&bar.close),
                volume: bar.volume as f64,
            }));

            match response.next_page_token {
                Some(tok) if !tok.is_empty() => page_token = Some(tok),
                _ => break,
            }
        }

        candles.retain(|c| is_regular_hours(c.timestamp));
        candles.sort_by_key(|c| c.timestamp);
        Ok(candles)
    }
}

/// how a streaming session ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamEnd {
    /// the receiving side dropped the channel (process shutting down).
    ChannelClosed,
    /// the server ended the websocket without a transport error.
    ServerClosed,
}

/// convert an alpaca stream bar into our Candle type.
pub fn stream_bar_to_candle(bar: &apca::data::v2::stream::Bar) -> Candle {
    Candle {
        timestamp: bar.timestamp,
        open: num_to_f64(&bar.open_price),
        high: num_to_f64(&bar.high_price),
        low: num_to_f64(&bar.low_price),
        close: num_to_f64(&bar.close_price),
        volume: num_to_f64(&bar.volume),
    }
}

/// convert an alpaca historical bar into our Candle type.
pub fn historical_bar_to_candle(bar: &apca::data::v2::bars::Bar) -> Candle {
    Candle {
        timestamp: bar.time,
        open: num_to_f64(&bar.open),
        high: num_to_f64(&bar.high),
        low: num_to_f64(&bar.low),
        close: num_to_f64(&bar.close),
        volume: bar.volume as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn bar_event_construction() {
        let candle = Candle {
            timestamp: Utc::now(),
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 50000.0,
        };
        let event = BarEvent {
            symbol: "SPY".to_string(),
            candle: candle.clone(),
        };
        assert_eq!(event.symbol, "SPY");
        assert!((event.candle.close - 100.5).abs() < f64::EPSILON);
    }

    #[test]
    fn alpaca_feed_construction() {
        let feed = AlpacaFeed::new(
            "test_key".to_string(),
            "test_secret".to_string(),
            vec!["SPY".to_string(), "QQQ".to_string()],
        );
        assert_eq!(feed.symbols.len(), 2);
    }

    #[test]
    fn data_feed_error_display() {
        let e = DataFeedError::ConnectionFailed("timeout".to_string());
        assert!(e.to_string().contains("timeout"));
    }

    #[tokio::test]
    async fn channel_send_receive() {
        let (tx, mut rx) = mpsc::channel::<BarEvent>(10);
        let event = BarEvent {
            symbol: "SPY".to_string(),
            candle: Candle {
                timestamp: Utc::now(),
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 50000.0,
            },
        };
        tx.send(event).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.symbol, "SPY");
    }

    #[test]
    #[ignore] // requires live alpaca credentials
    fn live_historical_bars() {
        // run with: cargo test -p data_feed -- --ignored live_historical_bars
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let feed = AlpacaFeed::new(
                std::env::var("APCA_API_KEY_ID").unwrap(),
                std::env::var("APCA_API_SECRET_KEY").unwrap(),
                vec!["SPY".to_string()],
            );
            let candles = feed.fetch_historical_bars("SPY", 2).await.unwrap();
            assert!(!candles.is_empty());
        });
    }
}
