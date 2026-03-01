use std::fmt;

use chrono::Utc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use types::market::Candle;

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

            match self.run_stream(&client, &tx).await {
                Ok(()) => {
                    info!("stream ended cleanly");
                    break Ok(());
                }
                Err(e) => {
                    warn!(error = %e, backoff_ms, "stream disconnected, reconnecting");
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(max_backoff_ms);
                }
            }
        }
    }

    async fn run_stream(
        &self,
        client: &apca::Client,
        tx: &mpsc::Sender<BarEvent>,
    ) -> Result<(), DataFeedError> {
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
                        info!("bar channel closed, stopping stream");
                        return Ok(());
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

        Ok(())
    }

    /// fetch historical bars for bootstrapping indicator lookback windows.
    /// lookback is the number of minutes of data to fetch.
    pub async fn fetch_historical_bars(
        &self,
        symbol: &str,
        lookback: usize,
    ) -> Result<Vec<Candle>, DataFeedError> {
        use apca::data::v2::bars;
        use apca::ApiInfo;
        use apca::Client;

        let api_info = ApiInfo::from_parts(
            "https://paper-api.alpaca.markets",
            &self.api_key,
            &self.api_secret,
        )
        .map_err(|e| DataFeedError::HistoricalFetchError(format!("{e}")))?;

        let client = Client::new(api_info);

        let end = Utc::now();
        let start = end - chrono::Duration::minutes(lookback as i64);

        let req = bars::ListReqInit {
            limit: Some(lookback),
            ..Default::default()
        }
        .init(symbol, start, end, bars::TimeFrame::OneMinute);

        let response = client
            .issue::<bars::List>(&req)
            .await
            .map_err(|e| DataFeedError::HistoricalFetchError(format!("{e}")))?;

        let candles: Vec<Candle> = response
            .bars
            .iter()
            .map(|bar| Candle {
                timestamp: bar.time,
                open: num_to_f64(&bar.open),
                high: num_to_f64(&bar.high),
                low: num_to_f64(&bar.low),
                close: num_to_f64(&bar.close),
                volume: bar.volume as f64,
            })
            .collect();

        Ok(candles)
    }
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
            let candles = feed.fetch_historical_bars("SPY", 10).await.unwrap();
            assert!(!candles.is_empty());
        });
    }
}
