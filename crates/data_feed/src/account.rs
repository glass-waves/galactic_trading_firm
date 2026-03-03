use std::fmt;

use tracing::{info, warn};

/// errors from account operations.
#[derive(Debug)]
pub enum AccountError {
    ConnectionFailed(String),
    FetchError(String),
}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountError::ConnectionFailed(msg) => write!(f, "account connection failed: {msg}"),
            AccountError::FetchError(msg) => write!(f, "account fetch error: {msg}"),
        }
    }
}

impl std::error::Error for AccountError {}

/// convert an apca Num to f64, defaulting to 0.0 on failure.
fn num_to_f64(n: &num_decimal::Num) -> f64 {
    n.to_f64().unwrap_or(0.0)
}

/// fetch account equity from alpaca paper trading API.
pub async fn fetch_alpaca_equity(
    api_key: &str,
    api_secret: &str,
) -> Result<f64, AccountError> {
    use apca::api::v2::account;
    use apca::ApiInfo;
    use apca::Client;

    let api_info = ApiInfo::from_parts(
        "https://paper-api.alpaca.markets",
        api_key,
        api_secret,
    )
    .map_err(|e| AccountError::ConnectionFailed(format!("{e}")))?;

    let client = Client::new(api_info);

    let acct = client
        .issue::<account::Get>(&())
        .await
        .map_err(|e| AccountError::FetchError(format!("{e}")))?;

    Ok(num_to_f64(&acct.equity))
}

/// resolve capital for the trading session.
/// if broker_mode is "alpaca_paper" and credentials are present, fetches
/// account equity from alpaca. otherwise returns the fallback value.
pub async fn resolve_capital(
    broker_mode: &str,
    api_key: &str,
    api_secret: &str,
    fallback: f64,
) -> f64 {
    if broker_mode == "alpaca_paper" && !api_key.is_empty() && !api_secret.is_empty() {
        match fetch_alpaca_equity(api_key, api_secret).await {
            Ok(equity) => {
                info!(equity, "fetched account equity from alpaca");
                equity
            }
            Err(e) => {
                warn!(error = %e, fallback, "failed to fetch alpaca equity, using fallback");
                fallback
            }
        }
    } else {
        info!(capital = fallback, "using configured capital");
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_error_display() {
        let e = AccountError::ConnectionFailed("timeout".to_string());
        assert!(e.to_string().contains("timeout"));

        let e = AccountError::FetchError("unauthorized".to_string());
        assert!(e.to_string().contains("unauthorized"));
    }

    #[tokio::test]
    async fn resolve_capital_simulated_fallback() {
        // non-alpaca mode should return fallback immediately
        let capital = resolve_capital("simulated", "key", "secret", 50_000.0).await;
        assert!((capital - 50_000.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn resolve_capital_empty_credentials_fallback() {
        // alpaca mode with empty credentials should fall back
        let capital = resolve_capital("alpaca_paper", "", "", 75_000.0).await;
        assert!((capital - 75_000.0).abs() < f64::EPSILON);
    }
}
