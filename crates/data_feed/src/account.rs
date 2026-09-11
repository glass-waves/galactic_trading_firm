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

/// the subset of `/v2/account` we need. apca 0.30's `Account` type requires
/// fields alpaca now returns as null (`pattern_day_trader`), so we parse a
/// lenient struct ourselves and issue it through apca's client (which supplies
/// the base URL and auth headers).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AccountSummary {
    pub status: String,
    pub equity: String,
    pub cash: String,
    pub buying_power: String,
    #[serde(default)]
    pub trading_blocked: bool,
    #[serde(default)]
    pub shorting_enabled: bool,
    #[serde(default)]
    pub multiplier: String,
}

http_endpoint::EndpointDef! {
    /// GET /v2/account with lenient parsing.
    pub AccountGet(()),
    Ok => AccountSummary, [
        /// the account was retrieved successfully.
        OK,
    ],
    Err => AccountGetError, [
        /// invalid credentials.
        UNAUTHORIZED => Unauthorized,
        /// not permitted.
        FORBIDDEN => NotPermitted,
        /// rate limited.
        TOO_MANY_REQUESTS => RateLimitExceeded,
    ],
    ConversionErr => serde_json::Error,
    ApiErr => apca::ApiError,

    fn path(_input: &Self::Input) -> std::borrow::Cow<'static, str> {
        "/v2/account".into()
    }

    fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
        serde_json::from_slice::<Self::Output>(body)
    }

    fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
        serde_json::from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
    }
}

/// fetch the account summary from the alpaca paper trading API.
pub async fn fetch_alpaca_account(
    api_key: &str,
    api_secret: &str,
) -> Result<AccountSummary, AccountError> {
    use apca::ApiInfo;
    use apca::Client;

    let api_info = ApiInfo::from_parts(
        "https://paper-api.alpaca.markets",
        api_key,
        api_secret,
    )
    .map_err(|e| AccountError::ConnectionFailed(format!("{e}")))?;

    let client = Client::new(api_info);

    client
        .issue::<AccountGet>(&())
        .await
        .map_err(|e| AccountError::FetchError(format!("{e:?}")))
}

/// fetch account equity from alpaca paper trading API.
pub async fn fetch_alpaca_equity(
    api_key: &str,
    api_secret: &str,
) -> Result<f64, AccountError> {
    let acct = fetch_alpaca_account(api_key, api_secret).await?;
    if acct.trading_blocked {
        warn!(status = %acct.status, "alpaca account reports trading_blocked = true");
    }
    info!(
        status = %acct.status,
        cash = %acct.cash,
        buying_power = %acct.buying_power,
        multiplier = %acct.multiplier,
        shorting_enabled = acct.shorting_enabled,
        "alpaca account"
    );
    acct.equity
        .parse::<f64>()
        .map_err(|e| AccountError::FetchError(format!("equity not numeric ({}): {e}", acct.equity)))
}

/// resolve capital for the trading session.
/// if broker_mode is "alpaca_paper" and credentials are present, fetches
/// account equity from alpaca and returns the SMALLER of the configured
/// capital and live equity — the configured value is the sizing budget the
/// operator chose (and what backtests were run at); equity only ever lowers it
/// (hardening plan 1.6). otherwise returns the configured value.
pub async fn resolve_capital(
    broker_mode: &str,
    api_key: &str,
    api_secret: &str,
    configured: f64,
) -> f64 {
    if broker_mode == "alpaca_paper" && !api_key.is_empty() && !api_secret.is_empty() {
        match fetch_alpaca_equity(api_key, api_secret).await {
            Ok(equity) => {
                let capital = configured.min(equity);
                info!(equity, configured, capital, "capital = min(configured, alpaca equity)");
                capital
            }
            Err(e) => {
                warn!(error = %e, configured, "failed to fetch alpaca equity, using configured capital");
                configured
            }
        }
    } else {
        info!(capital = configured, "using configured capital");
        configured
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
    #[ignore] // requires live alpaca credentials: cargo test -p data_feed -- --ignored live_equity --nocapture
    async fn live_equity() {
        let _ = dotenvy::dotenv();
        let r = fetch_alpaca_equity(
            &std::env::var("APCA_API_KEY_ID").unwrap(),
            &std::env::var("APCA_API_SECRET_KEY").unwrap(),
        )
        .await;
        println!("equity result: {r:?}");
        assert!(r.is_ok());
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
