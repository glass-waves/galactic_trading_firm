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

    /// update the simulated market price before submitting orders.
    pub fn set_last_price(&self, price: f64) {
        self.last_price
            .store(price.to_bits(), std::sync::atomic::Ordering::Relaxed);
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

/// alpaca paper trading broker. uses the apca crate for order execution.
pub struct AlpacaBroker {
    _api_key: String,
    _api_secret: String,
}

impl AlpacaBroker {
    pub fn new(api_key: String, api_secret: String) -> Self {
        Self {
            _api_key: api_key,
            _api_secret: api_secret,
        }
    }
}

#[async_trait]
impl Broker for AlpacaBroker {
    async fn submit_order(
        &self,
        _ticker: &str,
        _direction: TradeDirection,
        _quantity: f64,
    ) -> Result<OrderFill, BrokerError> {
        // TODO: implement with apca client
        Err(BrokerError::ConnectionError(
            "alpaca broker not yet implemented".to_string(),
        ))
    }

    async fn close_position(&self, _ticker: &str) -> Result<OrderFill, BrokerError> {
        Err(BrokerError::ConnectionError(
            "alpaca broker not yet implemented".to_string(),
        ))
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

    #[test]
    fn alpaca_broker_construction() {
        let _broker = AlpacaBroker::new("key".to_string(), "secret".to_string());
    }

    #[tokio::test]
    async fn alpaca_broker_not_implemented() {
        let broker = AlpacaBroker::new("key".to_string(), "secret".to_string());
        let result = broker
            .submit_order("SPY", TradeDirection::Long, 100.0)
            .await;
        assert!(result.is_err());
    }
}
