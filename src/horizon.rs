use crate::cli::Asset;
use crate::error::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_NATIVE_BALANCE: &str = "0.0000000";
const ZERO_USD: &str = "0.00";
const STROOPS_PER_UNIT: i128 = 10_000_000;
const STROOPS_PER_CENT: i128 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceLine {
    pub asset_type: String,
    pub balance: String,
    pub asset_code: Option<String>,
    pub asset_issuer: Option<String>,
    pub limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountInfo {
    pub exists: bool,
    pub sequence: i64,
    pub subentry_count: u32,
    pub native_balance: String,
    pub balances: Vec<BalanceLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositObservation {
    pub amount_native: String,
    pub amount_usd: String,
    pub last_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmittedTransaction {
    pub hash: String,
    pub ledger: i64,
    pub successful: bool,
}

#[derive(Debug, Clone)]
pub struct HorizonProvider {
    client: reqwest::Client,
    horizon_url: String,
    coinbase_base_url: String,
}

impl HorizonProvider {
    pub fn new(horizon_url: impl Into<String>, coinbase_base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            horizon_url: horizon_url.into(),
            coinbase_base_url: coinbase_base_url.into(),
        }
    }

    pub async fn get_account(&self, address: &str) -> Result<AccountInfo, AppError> {
        let response = self
            .client
            .get(format!("{}/accounts/{}", self.horizon_url, address))
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;

        if status.as_u16() == 404 {
            return Ok(AccountInfo::default());
        }
        if !status.is_success() {
            let details = parse_json_or_raw(&text);
            if status.as_u16() == 400 {
                return Err(AppError::new(
                    ErrorCode::Usage,
                    horizon_error_message(&details, "Horizon rejected the account id"),
                )
                .with_details(details));
            }
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Horizon account request failed with status {status}"),
            )
            .with_details(details));
        }

        let value: Value = serde_json::from_str(&text).map_err(|error| {
            AppError::new(
                ErrorCode::ExternalService,
                format!("Horizon account response was not valid JSON: {error}"),
            )
            .with_details(json!({ "raw": text }))
        })?;

        let sequence = match value.get("sequence") {
            None => 0,
            Some(sequence) => sequence
                .as_str()
                .and_then(|raw| raw.parse::<i64>().ok())
                .or_else(|| sequence.as_i64())
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::ExternalService,
                        "Horizon account response has an invalid sequence value",
                    )
                    .with_details(json!({ "sequence": sequence }))
                })?,
        };
        let subentry_count = value
            .get("subentry_count")
            .and_then(Value::as_u64)
            .and_then(|count| u32::try_from(count).ok())
            .unwrap_or(0);
        let balances: Vec<BalanceLine> = value
            .get("balances")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(balance_line).collect())
            .unwrap_or_default();
        let native_balance = balances
            .iter()
            .find(|line| line.asset_type == "native")
            .map(|line| line.balance.clone())
            .unwrap_or_else(|| DEFAULT_NATIVE_BALANCE.to_string());

        Ok(AccountInfo {
            exists: true,
            sequence,
            subentry_count,
            native_balance,
            balances,
        })
    }

    pub async fn observe_deposit(
        &self,
        address: &str,
        asset: &Asset,
        xlm_price_usd_fallback: &str,
        usdc_issuer: &str,
    ) -> Result<DepositObservation, AppError> {
        let account = self.get_account(address).await?;
        if !account.exists {
            return Ok(zero_observation());
        }
        let last_signature = self.latest_transaction_hash(address).await;

        match asset {
            Asset::Xlm => {
                let stroops = parse_decimal_exact(&account.native_balance, 7).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::ExternalService,
                        format!(
                            "Horizon returned an invalid XLM balance: {}",
                            account.native_balance
                        ),
                    )
                })?;
                let live_price = self
                    .fetch_xlm_usd_spot_price()
                    .await
                    .unwrap_or_else(|_| xlm_price_usd_fallback.to_string());
                let price_cents = parse_decimal_rounded(&live_price, 2).ok_or_else(|| {
                    AppError::new(
                        ErrorCode::ExternalService,
                        format!("Invalid XLM price: {live_price}"),
                    )
                    .with_details(json!({ "price": live_price }))
                })?;
                let usd_cents = stroops
                    .checked_mul(price_cents)
                    .map(|value| value / STROOPS_PER_UNIT)
                    .ok_or_else(|| {
                        AppError::new(
                            ErrorCode::ExternalService,
                            "XLM balance is too large to price",
                        )
                    })?;
                Ok(DepositObservation {
                    amount_native: account.native_balance.clone(),
                    amount_usd: format_scaled(usd_cents, 2),
                    last_signature,
                })
            }
            Asset::Usdc => {
                let mut stroops: i128 = 0;
                for line in &account.balances {
                    if line.asset_code.as_deref() != Some("USDC")
                        || line.asset_issuer.as_deref() != Some(usdc_issuer)
                    {
                        continue;
                    }
                    let line_stroops = parse_decimal_exact(&line.balance, 7).ok_or_else(|| {
                        AppError::new(
                            ErrorCode::ExternalService,
                            format!("Horizon returned an invalid USDC balance: {}", line.balance),
                        )
                    })?;
                    stroops = stroops.checked_add(line_stroops).ok_or_else(|| {
                        AppError::new(
                            ErrorCode::ExternalService,
                            "USDC balance is too large to sum",
                        )
                    })?;
                }
                let usd_cents = stroops
                    .checked_add(STROOPS_PER_CENT / 2)
                    .map(|value| value.div_euclid(STROOPS_PER_CENT))
                    .ok_or_else(|| {
                        AppError::new(
                            ErrorCode::ExternalService,
                            "USDC balance is too large to price",
                        )
                    })?;
                Ok(DepositObservation {
                    amount_native: format_scaled(stroops, 7),
                    amount_usd: format_scaled(usd_cents, 2),
                    last_signature,
                })
            }
        }
    }

    pub async fn fetch_xlm_usd_spot_price(&self) -> Result<String, AppError> {
        let response = self
            .client
            .get(format!("{}/v2/prices/XLM-USD/spot", self.coinbase_base_url))
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        let value = parse_json_or_raw(&text);

        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Coinbase spot price request failed with status {status}"),
            )
            .with_details(value));
        }
        let amount = value
            .pointer("/data/amount")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ExternalService,
                    "Coinbase spot price response missing data.amount",
                )
                .with_details(value.clone())
            })?;
        Ok(amount.to_string())
    }

    pub async fn submit_transaction(
        &self,
        envelope_base64: &str,
    ) -> Result<SubmittedTransaction, AppError> {
        let response = self
            .client
            .post(format!("{}/transactions", self.horizon_url))
            .form(&[("tx", envelope_base64)])
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        let details = attach_result_codes(parse_json_or_raw(&text));

        if status.as_u16() == 429 {
            return Err(AppError::new(
                ErrorCode::RateLimited,
                "Horizon rate limited the transaction submission",
            )
            .with_details(details));
        }
        if status.as_u16() == 400 {
            let message = result_codes_message(&details);
            return Err(AppError::new(ErrorCode::ExternalService, message).with_details(details));
        }
        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Horizon transaction submission failed with status {status}"),
            )
            .with_details(details));
        }

        let hash = details
            .get("hash")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ExternalService,
                    "Horizon transaction response is missing the hash",
                )
                .with_details(details.clone())
            })?
            .to_string();
        let ledger = details
            .get("ledger")
            .and_then(|ledger| {
                ledger
                    .as_i64()
                    .or_else(|| ledger.as_str().and_then(|raw| raw.parse().ok()))
            })
            .unwrap_or(0);
        let successful = details
            .get("successful")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !successful {
            return Err(
                AppError::new(ErrorCode::ExternalService, result_codes_message(&details))
                    .with_details(details),
            );
        }

        Ok(SubmittedTransaction {
            hash,
            ledger,
            successful,
        })
    }

    pub async fn fund_with_friendbot(
        &self,
        friendbot_url: &str,
        address: &str,
    ) -> Result<Option<String>, AppError> {
        let response = self
            .client
            .get(friendbot_url)
            .query(&[("addr", address)])
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        let value = parse_json_or_raw(&text);

        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Friendbot request failed with status {status}"),
            )
            .with_details(value));
        }
        Ok(value
            .get("hash")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    async fn latest_transaction_hash(&self, address: &str) -> Option<String> {
        let response = self
            .client
            .get(format!(
                "{}/accounts/{}/transactions",
                self.horizon_url, address
            ))
            .query(&[("order", "desc"), ("limit", "1")])
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let value: Value = response.json().await.ok()?;
        value
            .pointer("/_embedded/records/0/hash")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
}

fn zero_observation() -> DepositObservation {
    DepositObservation {
        amount_native: DEFAULT_NATIVE_BALANCE.to_string(),
        amount_usd: ZERO_USD.to_string(),
        last_signature: None,
    }
}

fn balance_line(value: &Value) -> BalanceLine {
    BalanceLine {
        asset_type: value
            .get("asset_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        balance: value
            .get("balance")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        asset_code: value
            .get("asset_code")
            .and_then(Value::as_str)
            .map(str::to_string),
        asset_issuer: value
            .get("asset_issuer")
            .and_then(Value::as_str)
            .map(str::to_string),
        limit: value
            .get("limit")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn parse_json_or_raw(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| json!({ "raw": text }))
}

fn horizon_error_message(value: &Value, fallback: &str) -> String {
    value
        .get("detail")
        .and_then(Value::as_str)
        .or_else(|| value.get("title").and_then(Value::as_str))
        .unwrap_or(fallback)
        .to_string()
}

fn attach_result_codes(mut body: Value) -> Value {
    if let Some(codes) = body.pointer("/extras/result_codes").cloned() {
        if body.is_object() {
            body["result_codes"] = codes;
        } else {
            body = json!({ "response": body, "result_codes": codes });
        }
    }
    body
}

fn result_codes_message(value: &Value) -> String {
    let result_codes = value.get("result_codes").unwrap_or(value);
    let mut parts: Vec<String> = Vec::new();
    if let Some(transaction) = result_codes.get("transaction").and_then(Value::as_str) {
        parts.push(transaction.to_string());
    }
    if let Some(operations) = result_codes.get("operations").and_then(Value::as_array) {
        for operation in operations {
            if let Some(code) = operation.as_str() {
                parts.push(code.to_string());
            }
        }
    }
    if parts.is_empty() {
        "Transaction failed".to_string()
    } else {
        format!("Transaction failed: {}", parts.join(", "))
    }
}

fn parse_decimal_exact(value: &str, decimals: u32) -> Option<i128> {
    parse_decimal(value, decimals, false)
}

fn parse_decimal_rounded(value: &str, decimals: u32) -> Option<i128> {
    parse_decimal(value, decimals, true)
}

fn parse_decimal(value: &str, decimals: u32, round: bool) -> Option<i128> {
    let value = value.trim();
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value),
    };
    let (whole, fraction) = match digits.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (digits, ""),
    };
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let whole: i128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let scale = 10i128.checked_pow(decimals)?;
    let mut scaled_fraction: i128 = 0;
    let mut chars = fraction.chars();
    for _ in 0..decimals {
        let digit = chars.next().unwrap_or('0').to_digit(10)? as i128;
        scaled_fraction = scaled_fraction.checked_mul(10)?.checked_add(digit)?;
    }
    let mut scaled = whole.checked_mul(scale)?.checked_add(scaled_fraction)?;
    if round {
        if let Some(next) = chars.next() {
            if next.to_digit(10)? >= 5 {
                scaled = scaled.checked_add(1)?;
            }
        }
    }
    Some(if negative { -scaled } else { scaled })
}

fn format_scaled(value: i128, decimals: u32) -> String {
    let negative = value < 0;
    let magnitude = value.unsigned_abs();
    let scale = 10u128.pow(decimals);
    let whole = magnitude / scale;
    let fraction = magnitude % scale;
    let sign = if negative { "-" } else { "" };
    if decimals == 0 {
        return format!("{sign}{whole}");
    }
    format!(
        "{sign}{whole}.{fraction:0>width$}",
        width = decimals as usize
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const ADDRESS: &str = "GDEPOSITADDRESS";
    const USDC_ISSUER: &str = "GALLOWEDUSDCISSUER";
    const OTHER_ISSUER: &str = "GOTHERUSDCISSUER";
    const TX_HASH: &str = "0f3a1c2d4e5b6a7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e";

    fn provider(server: &MockServer) -> HorizonProvider {
        HorizonProvider::new(server.uri(), server.uri())
    }

    fn account_body(sequence: &str, balances: Value) -> Value {
        json!({
            "sequence": sequence,
            "subentry_count": 1,
            "balances": balances,
        })
    }

    async fn mount_account(server: &MockServer, status: u16, body: Value) {
        Mock::given(method("GET"))
            .and(path(format!("/accounts/{ADDRESS}")))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    async fn mount_transactions(server: &MockServer, status: u16, body: Value) {
        Mock::given(method("GET"))
            .and(path(format!("/accounts/{ADDRESS}/transactions")))
            .and(query_param("order", "desc"))
            .and(query_param("limit", "1"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    async fn mount_spot_price(server: &MockServer, status: u16, body: Value) {
        Mock::given(method("GET"))
            .and(path("/v2/prices/XLM-USD/spot"))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn get_account_404_returns_missing_account() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            404,
            json!({
                "type": "https://stellar.org/horizon-errors/not_found",
                "title": "Resource Missing",
            }),
        )
        .await;

        let account = provider(&server).get_account(ADDRESS).await.unwrap();

        assert!(!account.exists);
        assert_eq!(account.sequence, 0);
        assert_eq!(account.subentry_count, 0);
        assert!(account.native_balance.is_empty());
        assert!(account.balances.is_empty());
    }

    #[tokio::test]
    async fn get_account_400_maps_to_usage_error() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            400,
            json!({
                "type": "https://stellar.org/horizon-errors/bad_request",
                "title": "Bad Request",
                "status": 400,
                "detail": "Account ID is invalid",
            }),
        )
        .await;

        let error = provider(&server).get_account(ADDRESS).await.unwrap_err();

        assert_eq!(error.code, ErrorCode::Usage);
        assert_eq!(error.message, "Account ID is invalid");
        assert_eq!(
            error.details["type"],
            "https://stellar.org/horizon-errors/bad_request"
        );
    }

    #[tokio::test]
    async fn get_account_parses_balances_and_sequence() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            200,
            account_body(
                "123456789",
                json!([
                    {"asset_type": "native", "balance": "12.3456789"},
                    {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "USDC",
                        "asset_issuer": USDC_ISSUER,
                        "balance": "1.0000000",
                        "limit": "922337203685477.5807",
                    },
                ]),
            ),
        )
        .await;

        let account = provider(&server).get_account(ADDRESS).await.unwrap();

        assert!(account.exists);
        assert_eq!(account.sequence, 123456789);
        assert_eq!(account.subentry_count, 1);
        assert_eq!(account.native_balance, "12.3456789");
        assert_eq!(account.balances.len(), 2);
        assert_eq!(account.balances[1].asset_code.as_deref(), Some("USDC"));
        assert_eq!(
            account.balances[1].asset_issuer.as_deref(),
            Some(USDC_ISSUER)
        );
        assert_eq!(
            account.balances[1].limit.as_deref(),
            Some("922337203685477.5807")
        );
    }

    #[tokio::test]
    async fn observe_deposit_missing_account_returns_zeros() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            404,
            json!({
                "type": "https://stellar.org/horizon-errors/not_found",
                "title": "Resource Missing",
            }),
        )
        .await;

        for asset in [Asset::Xlm, Asset::Usdc] {
            let observation = provider(&server)
                .observe_deposit(ADDRESS, &asset, "0.30", USDC_ISSUER)
                .await
                .unwrap();
            assert_eq!(observation.amount_native, "0.0000000");
            assert_eq!(observation.amount_usd, "0.00");
            assert!(observation.last_signature.is_none());
        }
    }

    #[tokio::test]
    async fn observe_deposit_xlm_uses_live_coinbase_price() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            200,
            account_body(
                "41",
                json!([{"asset_type": "native", "balance": "10.0000000"}]),
            ),
        )
        .await;
        mount_spot_price(
            &server,
            200,
            json!({"data": {"amount": "0.2000000", "base": "XLM", "currency": "USD"}}),
        )
        .await;
        mount_transactions(
            &server,
            200,
            json!({
                "_embedded": {
                    "records": [{
                        "hash": TX_HASH,
                        "successful": true,
                        "ledger": 99,
                        "created_at": "2026-09-19T00:00:00Z",
                    }],
                },
            }),
        )
        .await;

        let observation = provider(&server)
            .observe_deposit(ADDRESS, &Asset::Xlm, "0.10", USDC_ISSUER)
            .await
            .unwrap();

        assert_eq!(observation.amount_native, "10.0000000");
        assert_eq!(observation.amount_usd, "2.00");
        assert_eq!(observation.last_signature.as_deref(), Some(TX_HASH));
    }

    #[tokio::test]
    async fn observe_deposit_xlm_falls_back_when_coinbase_fails() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            200,
            account_body(
                "42",
                json!([{"asset_type": "native", "balance": "2.0000000"}]),
            ),
        )
        .await;
        mount_spot_price(&server, 500, json!({"error": "internal server error"})).await;

        let observation = provider(&server)
            .observe_deposit(ADDRESS, &Asset::Xlm, "0.50", USDC_ISSUER)
            .await
            .unwrap();

        assert_eq!(observation.amount_native, "2.0000000");
        assert_eq!(observation.amount_usd, "1.00");
        assert!(observation.last_signature.is_none());
    }

    #[tokio::test]
    async fn observe_deposit_xlm_uses_integer_price_math() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            200,
            account_body(
                "43",
                json!([{"asset_type": "native", "balance": "1.2345678"}]),
            ),
        )
        .await;
        mount_spot_price(
            &server,
            200,
            json!({"data": {"amount": "0.196", "base": "XLM", "currency": "USD"}}),
        )
        .await;

        let observation = provider(&server)
            .observe_deposit(ADDRESS, &Asset::Xlm, "9.99", USDC_ISSUER)
            .await
            .unwrap();

        assert_eq!(observation.amount_native, "1.2345678");
        assert_eq!(observation.amount_usd, "0.24");
    }

    #[tokio::test]
    async fn observe_deposit_usdc_sums_only_matching_issuer() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            200,
            account_body(
                "44",
                json!([
                    {"asset_type": "native", "balance": "1.0000000"},
                    {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "USDC",
                        "asset_issuer": USDC_ISSUER,
                        "balance": "5.5000000",
                        "limit": "1000.0000000",
                    },
                    {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "USDC",
                        "asset_issuer": USDC_ISSUER,
                        "balance": "0.2500000",
                        "limit": "1000.0000000",
                    },
                    {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "USDC",
                        "asset_issuer": OTHER_ISSUER,
                        "balance": "9.0000000",
                        "limit": "1000.0000000",
                    },
                    {
                        "asset_type": "credit_alphanum4",
                        "asset_code": "EURT",
                        "asset_issuer": USDC_ISSUER,
                        "balance": "3.0000000",
                        "limit": "1000.0000000",
                    },
                ]),
            ),
        )
        .await;

        let observation = provider(&server)
            .observe_deposit(ADDRESS, &Asset::Usdc, "0.30", USDC_ISSUER)
            .await
            .unwrap();

        assert_eq!(observation.amount_native, "5.7500000");
        assert_eq!(observation.amount_usd, "5.75");
        assert!(observation.last_signature.is_none());
    }

    #[tokio::test]
    async fn observe_deposit_ignores_failing_transactions_request() {
        let server = MockServer::start().await;
        mount_account(
            &server,
            200,
            account_body(
                "45",
                json!([{"asset_type": "native", "balance": "3.0000000"}]),
            ),
        )
        .await;
        mount_spot_price(
            &server,
            200,
            json!({"data": {"amount": "0.1000000", "base": "XLM", "currency": "USD"}}),
        )
        .await;
        mount_transactions(&server, 500, json!({"title": "Internal Server Error"})).await;

        let observation = provider(&server)
            .observe_deposit(ADDRESS, &Asset::Xlm, "0.10", USDC_ISSUER)
            .await
            .unwrap();

        assert_eq!(observation.amount_usd, "0.30");
        assert!(observation.last_signature.is_none());
    }

    #[tokio::test]
    async fn fetch_xlm_usd_spot_price_returns_amount() {
        let server = MockServer::start().await;
        mount_spot_price(
            &server,
            200,
            json!({"data": {"amount": "0.1907445", "base": "XLM", "currency": "USD"}}),
        )
        .await;

        let amount = provider(&server).fetch_xlm_usd_spot_price().await.unwrap();

        assert_eq!(amount, "0.1907445");
    }

    #[tokio::test]
    async fn fetch_xlm_usd_spot_price_missing_amount_is_external_error() {
        let server = MockServer::start().await;
        mount_spot_price(&server, 200, json!({"data": {}})).await;

        let error = provider(&server)
            .fetch_xlm_usd_spot_price()
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert_eq!(error.details["data"], json!({}));
    }

    #[tokio::test]
    async fn submit_transaction_success_returns_hash() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transactions"))
            .and(body_string_contains("tx=AAAA"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hash": "deadbeef",
                "ledger": 77,
                "successful": true,
                "envelope_xdr": "AAAA",
                "result_xdr": "AAAA",
            })))
            .mount(&server)
            .await;

        let submitted = provider(&server).submit_transaction("AAAA").await.unwrap();

        assert_eq!(submitted.hash, "deadbeef");
        assert_eq!(submitted.ledger, 77);
        assert!(submitted.successful);
    }

    #[tokio::test]
    async fn submit_transaction_400_surfaces_result_codes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transactions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "title": "Transaction Failed",
                "extras": {"result_codes": {"transaction": "tx_bad_seq"}},
            })))
            .mount(&server)
            .await;

        let error = provider(&server)
            .submit_transaction("AAAA")
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert!(error.message.contains("tx_bad_seq"));
        assert_eq!(error.details["result_codes"]["transaction"], "tx_bad_seq");
        assert_eq!(
            error.details["extras"]["result_codes"]["transaction"],
            "tx_bad_seq"
        );
    }

    #[tokio::test]
    async fn submit_transaction_429_is_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transactions"))
            .respond_with(
                ResponseTemplate::new(429).set_body_json(json!({"title": "Rate Limit Exceeded"})),
            )
            .mount(&server)
            .await;

        let error = provider(&server)
            .submit_transaction("AAAA")
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::RateLimited);
    }

    #[tokio::test]
    async fn fund_with_friendbot_returns_hash() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .and(query_param("addr", ADDRESS))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hash": "friendbophash",
                "successful": true,
            })))
            .mount(&server)
            .await;

        let hash = provider(&server)
            .fund_with_friendbot(&server.uri(), ADDRESS)
            .await
            .unwrap();

        assert_eq!(hash.as_deref(), Some("friendbophash"));
    }

    #[tokio::test]
    async fn fund_with_friendbot_error_returns_details() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "detail": "createAccountAlreadyExist",
            })))
            .mount(&server)
            .await;

        let error = provider(&server)
            .fund_with_friendbot(&server.uri(), ADDRESS)
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert_eq!(error.details["detail"], "createAccountAlreadyExist");
    }
}
