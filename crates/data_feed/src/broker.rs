use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use types::action::TradeDirection;

/// result of a filled order.
#[derive(Debug, Clone)]
pub struct OrderFill {
    pub ticker: String,
    pub direction: TradeDirection,
    pub quantity: f64,
    pub fill_price: f64,
    pub filled_at: DateTime<Utc>,
}

/// broker errors.
#[derive(Debug)]
pub enum BrokerError {
    OrderRejected(String),
    ConnectionError(String),
    NoPosition(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::OrderRejected(msg) => write!(f, "order rejected: {msg}"),
            BrokerError::ConnectionError(msg) => write!(f, "connection error: {msg}"),
            BrokerError::NoPosition(msg) => write!(f, "no position: {msg}"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// a position the broker currently holds.
#[derive(Debug, Clone)]
pub struct BrokerPosition {
    pub ticker: String,
    pub direction: TradeDirection,
    pub quantity: f64,
    pub entry_price: f64,
}

/// trait for order execution. implementations can be live (alpaca) or simulated.
#[async_trait]
pub trait Broker: Send + Sync {
    async fn submit_order(
        &self,
        ticker: &str,
        direction: TradeDirection,
        quantity: f64,
    ) -> Result<OrderFill, BrokerError>;

    async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError>;

    /// positions the broker currently holds (for startup reconciliation).
    async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError>;

    /// update the market price. default no-op for brokers that don't need it.
    fn set_last_price(&self, _price: f64) {}
}

/// simulated broker that fills at last_price ± slippage. no external deps.
pub struct SimulatedBroker {
    /// slippage in basis points (e.g., 5 = 0.05% = 5 bps).
    slippage_bps: f64,
    /// current market price — updated externally before each order.
    last_price: std::sync::atomic::AtomicU64,
    /// track simulated positions: ticker → (direction, quantity, entry_price).
    positions: tokio::sync::Mutex<std::collections::HashMap<String, (TradeDirection, f64, f64)>>,
}

impl SimulatedBroker {
    pub fn new(slippage_bps: f64) -> Self {
        Self {
            slippage_bps,
            last_price: std::sync::atomic::AtomicU64::new(0),
            positions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn get_last_price(&self) -> f64 {
        f64::from_bits(
            self.last_price
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    fn apply_slippage(&self, price: f64, direction: &TradeDirection) -> f64 {
        let slip = price * self.slippage_bps / 10_000.0;
        match direction {
            TradeDirection::Long => price + slip,  // buy at higher price
            TradeDirection::Short => price - slip, // sell at lower price
        }
    }
}

#[async_trait]
impl Broker for SimulatedBroker {
    fn set_last_price(&self, price: f64) {
        self.last_price
            .store(price.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }

    async fn submit_order(
        &self,
        ticker: &str,
        direction: TradeDirection,
        quantity: f64,
    ) -> Result<OrderFill, BrokerError> {
        let base_price = self.get_last_price();
        if base_price <= 0.0 {
            return Err(BrokerError::OrderRejected(
                "last_price not set".to_string(),
            ));
        }

        let fill_price = self.apply_slippage(base_price, &direction);

        // track position
        let mut positions = self.positions.lock().await;
        positions.insert(
            ticker.to_string(),
            (direction, quantity, fill_price),
        );

        Ok(OrderFill {
            ticker: ticker.to_string(),
            direction,
            quantity,
            fill_price,
            filled_at: Utc::now(),
        })
    }

    async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
        let positions = self.positions.lock().await;
        Ok(positions
            .iter()
            .map(|(t, (d, q, p))| BrokerPosition {
                ticker: t.clone(),
                direction: *d,
                quantity: *q,
                entry_price: *p,
            })
            .collect())
    }

    async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError> {
        let mut positions = self.positions.lock().await;
        let (direction, quantity, _entry_price) = positions
            .remove(ticker)
            .ok_or_else(|| BrokerError::NoPosition(ticker.to_string()))?;

        let base_price = self.get_last_price();
        // closing: reverse direction for slippage
        let close_direction = match direction {
            TradeDirection::Long => TradeDirection::Short,
            TradeDirection::Short => TradeDirection::Long,
        };
        let fill_price = self.apply_slippage(base_price, &close_direction);

        Ok(OrderFill {
            ticker: ticker.to_string(),
            direction: close_direction,
            quantity,
            fill_price,
            filled_at: Utc::now(),
        })
    }
}

/// convert an apca Num to f64.
fn num_to_f64(n: &num_decimal::Num) -> f64 {
    n.to_f64().unwrap_or(0.0)
}

/// alpaca paper trading broker. uses the apca crate for order execution.
pub struct AlpacaBroker {
    client: apca::Client,
}

impl AlpacaBroker {
    pub fn new(api_key: String, api_secret: String) -> Result<Self, BrokerError> {
        let api_info = apca::ApiInfo::from_parts(
            "https://paper-api.alpaca.markets",
            &api_key,
            &api_secret,
        )
        .map_err(|e| BrokerError::ConnectionError(format!("invalid credentials: {e}")))?;

        Ok(Self {
            client: apca::Client::new(api_info),
        })
    }
}

/// how long to wait for a market order to fill before giving up.
const FILL_TIMEOUT_MS: u64 = 15_000;
const FILL_POLL_MS: u64 = 250;

impl AlpacaBroker {
    /// alpaca's create/close responses come back as `accepted`/`pending_new`
    /// before the fill. poll the order until it is filled (or terminal), so the
    /// caller gets the real average fill price. on timeout the order is
    /// cancelled and an error returned so the engine can roll back.
    async fn wait_for_fill(
        &self,
        order: apca::api::v2::order::Order,
    ) -> Result<apca::api::v2::order::Order, BrokerError> {
        use apca::api::v2::order::{self, Status};

        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(FILL_TIMEOUT_MS);
        let mut current = order;
        loop {
            match current.status {
                Status::Filled => return Ok(current),
                Status::Canceled | Status::Expired | Status::Rejected | Status::Stopped | Status::Suspended => {
                    return Err(BrokerError::OrderRejected(format!(
                        "order {} ended in status {:?}",
                        current.id.0, current.status
                    )));
                }
                _ => {}
            }
            if std::time::Instant::now() >= deadline {
                let id = current.id;
                let _ = self.client.issue::<order::Delete>(&id).await;
                return Err(BrokerError::OrderRejected(format!(
                    "order {} not filled within {}ms (last status {:?}); cancel requested",
                    id.0, FILL_TIMEOUT_MS, current.status
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(FILL_POLL_MS)).await;
            let id = current.id;
            current = self
                .client
                .issue::<order::Get>(&id)
                .await
                .map_err(|e| BrokerError::ConnectionError(format!("order status poll failed: {e:?}")))?;
        }
    }

    /// wait for a fill with a caller-supplied timeout (used by tests).
    #[doc(hidden)]
    pub async fn submit_market_order_with_timeout(
        &self,
        ticker: &str,
        direction: TradeDirection,
        shares: i64,
        timeout_ms: u64,
    ) -> Result<OrderFill, BrokerError> {
        use apca::api::v2::order::{self, Status};

        let side = match direction {
            TradeDirection::Long => order::Side::Buy,
            TradeDirection::Short => order::Side::Sell,
        };
        let req = order::CreateReqInit {
            type_: order::Type::Market,
            time_in_force: order::TimeInForce::Day,
            ..Default::default()
        }
        .init(ticker, side, order::Amount::quantity(shares));

        let created = self
            .client
            .issue::<order::Create>(&req)
            .await
            .map_err(|e| BrokerError::OrderRejected(format!("{e:?}")))?;

        // short-circuit for tests that want to observe the cancel path quickly
        let filled = if timeout_ms == 0 && created.status != Status::Filled {
            let _ = self.client.issue::<order::Delete>(&created.id).await;
            return Err(BrokerError::OrderRejected(format!(
                "order {} not filled immediately (status {:?}); cancelled",
                created.id.0, created.status
            )));
        } else {
            self.wait_for_fill(created).await?
        };

        let fill_price = filled.average_fill_price.as_ref().map(num_to_f64).unwrap_or(0.0);
        if fill_price <= 0.0 {
            return Err(BrokerError::OrderRejected(format!(
                "filled order {} reported no average fill price",
                filled.id.0
            )));
        }
        Ok(OrderFill {
            ticker: ticker.to_string(),
            direction,
            quantity: num_to_f64(&filled.filled_quantity),
            fill_price,
            filled_at: filled.filled_at.unwrap_or_else(Utc::now),
        })
    }
}

#[async_trait]
impl Broker for AlpacaBroker {
    async fn submit_order(
        &self,
        ticker: &str,
        direction: TradeDirection,
        quantity: f64,
    ) -> Result<OrderFill, BrokerError> {
        // truncate to whole shares for alpaca
        let shares = quantity.floor() as i64;
        if shares <= 0 {
            return Err(BrokerError::OrderRejected(
                "quantity must be at least 1 share".to_string(),
            ));
        }
        self.submit_market_order_with_timeout(ticker, direction, shares, FILL_TIMEOUT_MS)
            .await
    }

    async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
        use apca::api::v2::{position, positions};

        let list = self
            .client
            .issue::<positions::List>(&())
            .await
            .map_err(|e| BrokerError::ConnectionError(format!("{e:?}")))?;

        Ok(list
            .into_iter()
            .map(|p| BrokerPosition {
                ticker: p.symbol.clone(),
                direction: match p.side {
                    position::Side::Long => TradeDirection::Long,
                    position::Side::Short => TradeDirection::Short,
                },
                quantity: num_to_f64(&p.quantity).abs(),
                entry_price: num_to_f64(&p.average_entry_price),
            })
            .collect())
    }

    async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError> {
        use apca::api::v2::position;

        let symbol = apca::api::v2::asset::Symbol::Sym(ticker.to_string());

        let order = self
            .client
            .issue::<position::Delete>(&symbol)
            .await
            .map_err(|e| {
                // check if the error is a NotFound
                let msg = format!("{e:?}");
                if msg.contains("404") || msg.contains("not found") || msg.contains("NotFound") {
                    BrokerError::NoPosition(ticker.to_string())
                } else {
                    BrokerError::ConnectionError(msg)
                }
            })?;

        // the closing order side tells us the close direction
        let direction = match order.side {
            apca::api::v2::order::Side::Buy => TradeDirection::Long,
            apca::api::v2::order::Side::Sell => TradeDirection::Short,
        };

        // a close that does not fill leaves the position open at the broker —
        // surface that loudly rather than pretending it closed.
        let filled = self.wait_for_fill(order).await?;

        let fill_price = filled.average_fill_price.as_ref().map(num_to_f64).unwrap_or(0.0);
        Ok(OrderFill {
            ticker: ticker.to_string(),
            direction,
            quantity: num_to_f64(&filled.filled_quantity),
            fill_price,
            filled_at: filled.filled_at.unwrap_or_else(Utc::now),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simulated_fill_at_last_price_no_slippage() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(150.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Long, 100.0)
            .await
            .unwrap();

        assert_eq!(fill.ticker, "SPY");
        assert!((fill.fill_price - 150.0).abs() < f64::EPSILON);
        assert!((fill.quantity - 100.0).abs() < f64::EPSILON);
        assert!(matches!(fill.direction, TradeDirection::Long));
    }

    #[tokio::test]
    async fn simulated_long_slippage_increases_price() {
        let broker = SimulatedBroker::new(10.0); // 10 bps = 0.1%
        broker.set_last_price(100.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Long, 50.0)
            .await
            .unwrap();

        // 100.0 + 100.0 * 10 / 10000 = 100.10
        assert!((fill.fill_price - 100.10).abs() < 1e-10);
    }

    #[tokio::test]
    async fn simulated_short_slippage_decreases_price() {
        let broker = SimulatedBroker::new(10.0); // 10 bps
        broker.set_last_price(100.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Short, 50.0)
            .await
            .unwrap();

        // 100.0 - 100.0 * 10 / 10000 = 99.90
        assert!((fill.fill_price - 99.90).abs() < 1e-10);
    }

    #[tokio::test]
    async fn simulated_close_position() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(100.0);

        broker
            .submit_order("SPY", TradeDirection::Long, 50.0)
            .await
            .unwrap();

        broker.set_last_price(105.0);
        let close = broker.close_position("SPY").await.unwrap();
        assert!((close.fill_price - 105.0).abs() < f64::EPSILON);
        assert!(matches!(close.direction, TradeDirection::Short));
    }

    #[tokio::test]
    async fn simulated_open_positions_reflects_state() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(100.0);
        assert!(broker.open_positions().await.unwrap().is_empty());
        broker.submit_order("SPY", TradeDirection::Long, 5.0).await.unwrap();
        let open = broker.open_positions().await.unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].ticker, "SPY");
        assert!((open[0].quantity - 5.0).abs() < f64::EPSILON);
        broker.close_position("SPY").await.unwrap();
        assert!(broker.open_positions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn simulated_close_no_position_errors() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(100.0);

        let result = broker.close_position("SPY").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn simulated_no_price_set_errors() {
        let broker = SimulatedBroker::new(0.0);
        let result = broker
            .submit_order("SPY", TradeDirection::Long, 50.0)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn simulated_direction_handling() {
        let broker = SimulatedBroker::new(5.0); // 5 bps
        broker.set_last_price(200.0);

        let long_fill = broker
            .submit_order("QQQ", TradeDirection::Long, 10.0)
            .await
            .unwrap();
        assert!(long_fill.fill_price > 200.0);

        // close long → direction is Short
        broker.set_last_price(210.0);
        let close = broker.close_position("QQQ").await.unwrap();
        assert!(matches!(close.direction, TradeDirection::Short));
        // closing a long means selling (Short slippage direction) → price decreases
        assert!(close.fill_price < 210.0);
    }

    /// live: creates a 1-share order with zero fill timeout and expects it to be
    /// cancelled (outside market hours it cannot fill; inside, it may fill — the
    /// test then closes it). exercises apca order create/get/delete deserialization.
    /// run: cargo test -p data_feed -- --ignored live_order_roundtrip --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_order_roundtrip() {
        let _ = dotenvy::dotenv();
        let broker = AlpacaBroker::new(
            std::env::var("APCA_API_KEY_ID").unwrap(),
            std::env::var("APCA_API_SECRET_KEY").unwrap(),
        )
        .unwrap();
        let r = broker
            .submit_market_order_with_timeout("AAPL", TradeDirection::Long, 1, 0)
            .await;
        println!("order result: {r:?}");
        match r {
            Ok(fill) => {
                // market must be open: clean up
                let c = broker.close_position("AAPL").await;
                println!("close result: {c:?}");
                assert!(fill.fill_price > 0.0);
            }
            Err(BrokerError::OrderRejected(msg)) => {
                assert!(msg.contains("cancelled"), "unexpected rejection: {msg}");
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
        let open = broker.open_positions().await.unwrap();
        assert!(open.iter().all(|p| p.ticker != "AAPL"), "AAPL position left open: {open:?}");
    }

    #[test]
    fn alpaca_broker_construction() {
        let broker = AlpacaBroker::new("key".to_string(), "secret".to_string());
        assert!(broker.is_ok());
    }

    #[test]
    fn alpaca_set_last_price_noop() {
        let broker = AlpacaBroker::new("key".to_string(), "secret".to_string()).unwrap();
        // default trait method — should not panic
        broker.set_last_price(150.0);
    }

    #[tokio::test]
    async fn simulated_set_last_price_via_trait() {
        let broker: Box<dyn Broker> = Box::new(SimulatedBroker::new(0.0));
        broker.set_last_price(200.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Long, 10.0)
            .await
            .unwrap();
        assert!((fill.fill_price - 200.0).abs() < f64::EPSILON);
    }
}
