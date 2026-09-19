use crate::error::{AppError, ErrorCode};
use crate::tx::{envelope_base64, sign_envelope, StellarKeypair};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use stellar_xdr::{Limits, ReadXdr, TransactionEnvelope};

#[derive(Debug, Clone)]
pub struct AnchorProvider {
    client: reqwest::Client,
    home_domain: String,
    sep24_url: Option<String>,
    web_auth_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorCurrency {
    pub code: String,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub anchor_asset_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorInfo {
    pub home_domain: String,
    pub web_auth_endpoint: String,
    pub transfer_server_sep24: String,
    pub signing_key: Option<String>,
    pub currencies: Vec<AnchorCurrency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorDeposit {
    pub id: String,
    pub url: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorTransaction {
    pub id: String,
    #[serde(default)]
    pub kind: Option<String>,
    pub status: String,
    #[serde(default)]
    pub amount_in: Option<String>,
    #[serde(default)]
    pub amount_out: Option<String>,
    #[serde(default)]
    pub asset_code: Option<String>,
    #[serde(default)]
    pub stellar_transaction_id: Option<String>,
    #[serde(default)]
    pub external_transaction_id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct StellarToml {
    #[serde(rename = "WEB_AUTH_ENDPOINT", default)]
    web_auth_endpoint: Option<String>,
    #[serde(rename = "TRANSFER_SERVER_SEP0024", default)]
    transfer_server_sep0024: Option<String>,
    #[serde(rename = "SIGNING_KEY", default)]
    signing_key: Option<String>,
    #[serde(rename = "CURRENCIES", default)]
    currencies: Vec<AnchorCurrency>,
}

impl AnchorProvider {
    pub fn new(
        home_domain: impl Into<String>,
        sep24_url: Option<String>,
        web_auth_endpoint: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            home_domain: home_domain.into(),
            sep24_url,
            web_auth_endpoint,
        }
    }

    pub fn home_domain(&self) -> &str {
        &self.home_domain
    }

    pub fn toml_url(&self) -> String {
        let base = self.home_domain.trim_end_matches('/');
        if base.starts_with("http://") || base.starts_with("https://") {
            format!("{base}/.well-known/stellar.toml")
        } else {
            format!("https://{base}/.well-known/stellar.toml")
        }
    }

    pub async fn fetch_info(&self) -> Result<AnchorInfo, AppError> {
        let url = self.toml_url();
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Failed to fetch stellar.toml from {url} (HTTP {status})"),
            )
            .with_details(json!({
                "url": url,
                "status": status.as_u16(),
                "body": text,
            })));
        }
        let toml_info: StellarToml = toml::from_str(&text).map_err(|error| {
            AppError::new(
                ErrorCode::ExternalService,
                format!("Invalid stellar.toml at {url}: {error}"),
            )
            .with_details(json!({
                "url": url,
                "error": error.to_string(),
                "body": text,
            }))
        })?;

        let web_auth_endpoint = clean_url(self.web_auth_endpoint.as_ref())
            .or_else(|| clean_url(toml_info.web_auth_endpoint.as_ref()));
        let transfer_server_sep24 = clean_url(self.sep24_url.as_ref())
            .or_else(|| clean_url(toml_info.transfer_server_sep0024.as_ref()));

        match (web_auth_endpoint, transfer_server_sep24) {
            (Some(web_auth_endpoint), Some(transfer_server_sep24)) => Ok(AnchorInfo {
                home_domain: self.home_domain.clone(),
                web_auth_endpoint,
                transfer_server_sep24,
                signing_key: toml_info.signing_key,
                currencies: toml_info.currencies,
            }),
            (web_auth_endpoint, transfer_server_sep24) => {
                let mut missing = Vec::new();
                if web_auth_endpoint.is_none() {
                    missing.push("WEB_AUTH_ENDPOINT");
                }
                if transfer_server_sep24.is_none() {
                    missing.push("TRANSFER_SERVER_SEP0024");
                }
                Err(AppError::new(
                    ErrorCode::Usage,
                    format!(
                        "Anchor {} does not support SEP-24 interactive deposits (missing {})",
                        self.home_domain,
                        missing.join(", ")
                    ),
                )
                .with_details(json!({
                    "home_domain": self.home_domain,
                    "missing": missing,
                    "toml_url": url,
                }))
                .with_suggestion(
                    "Configure anchor_sep24_url and anchor_web_auth_endpoint to use an anchor that does not publish them in stellar.toml",
                ))
            }
        }
    }

    pub async fn fetch_jwt(
        &self,
        account: &str,
        keypair: &StellarKeypair,
        passphrase: &str,
    ) -> Result<String, AppError> {
        let (web_auth_endpoint, _) = self.endpoints().await?;
        let response = self
            .client
            .get(&web_auth_endpoint)
            .query(&[("account", account)])
            .send()
            .await?;
        let value = parse_anchor_response(response, "SEP-10 challenge").await?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(AppError::new(
                ErrorCode::AuthenticationFailure,
                format!("Anchor rejected the SEP-10 challenge: {error}"),
            )
            .with_details(json!({"endpoint": web_auth_endpoint, "response": value})));
        }
        let transaction = value
            .get("transaction")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ExternalService,
                    "Anchor SEP-10 response is missing the challenge transaction",
                )
                .with_details(json!({"endpoint": web_auth_endpoint, "response": value}))
            })?;
        let network_passphrase = value
            .get("network_passphrase")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ExternalService,
                    "Anchor SEP-10 response is missing network_passphrase",
                )
                .with_details(json!({"endpoint": web_auth_endpoint, "response": value}))
            })?;
        if network_passphrase != passphrase {
            return Err(AppError::new(
                ErrorCode::AuthenticationFailure,
                format!(
                    "Anchor network passphrase mismatch: expected \"{passphrase}\", got \"{network_passphrase}\""
                ),
            )
            .with_details(json!({
                "endpoint": web_auth_endpoint,
                "expected_network_passphrase": passphrase,
                "anchor_network_passphrase": network_passphrase,
            })));
        }

        let envelope =
            TransactionEnvelope::from_xdr_base64(transaction, Limits::none()).map_err(|error| {
                AppError::new(
                    ErrorCode::ExternalService,
                    format!("Anchor returned an invalid SEP-10 challenge transaction: {error}"),
                )
                .with_details(json!({"endpoint": web_auth_endpoint}))
            })?;
        let signed = sign_envelope(envelope, keypair, passphrase)?;
        let encoded = envelope_base64(&signed)?;

        let response = self
            .client
            .post(&web_auth_endpoint)
            .form(&[("transaction", encoded)])
            .send()
            .await?;
        let value = parse_anchor_response(response, "SEP-10 token exchange").await?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(AppError::new(
                ErrorCode::AuthenticationFailure,
                format!("Anchor rejected the SEP-10 token request: {error}"),
            )
            .with_details(json!({"endpoint": web_auth_endpoint, "response": value})));
        }
        value
            .get("token")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::AuthenticationFailure,
                    "Anchor SEP-10 token response is missing the token",
                )
                .with_details(json!({"endpoint": web_auth_endpoint, "response": value}))
            })
    }

    pub async fn start_deposit(
        &self,
        jwt: &str,
        asset_code: &str,
        account: &str,
        amount: Option<&str>,
    ) -> Result<AnchorDeposit, AppError> {
        let (_, sep24_url) = self.endpoints().await?;
        let endpoint = format!(
            "{}/transactions/deposit/interactive",
            sep24_url.trim_end_matches('/')
        );
        let mut body = json!({
            "asset_code": asset_code,
            "account": account,
        });
        if let Some(amount) = amount {
            body["amount"] = json!(amount);
        }
        let response = self
            .client
            .post(&endpoint)
            .bearer_auth(jwt)
            .json(&body)
            .send()
            .await?;
        let value = parse_anchor_response(response, "SEP-24 deposit initiation").await?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Anchor refused the SEP-24 deposit: {error}"),
            )
            .with_details(json!({"endpoint": endpoint, "response": value})));
        }
        serde_json::from_value(value).map_err(|error| {
            AppError::new(
                ErrorCode::ExternalService,
                format!("Anchor returned an unexpected SEP-24 deposit response: {error}"),
            )
            .with_details(json!({"endpoint": endpoint}))
        })
    }

    pub async fn transaction_status(
        &self,
        jwt: &str,
        id: &str,
    ) -> Result<AnchorTransaction, AppError> {
        let (_, sep24_url) = self.endpoints().await?;
        let endpoint = format!("{}/transaction", sep24_url.trim_end_matches('/'));
        let response = self
            .client
            .get(&endpoint)
            .query(&[("id", id)])
            .bearer_auth(jwt)
            .send()
            .await?;
        let value = parse_anchor_response(response, "SEP-24 transaction query").await?;
        if let Some(error) = value.get("error").and_then(Value::as_str) {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Anchor refused the SEP-24 transaction query: {error}"),
            )
            .with_details(json!({"endpoint": endpoint, "response": value})));
        }
        let transaction = match value.get("transaction") {
            Some(transaction) if !transaction.is_null() => transaction.clone(),
            _ => {
                return Err(AppError::new(
                    ErrorCode::ResourceNotFound,
                    format!("Anchor has no transaction with id {id}"),
                )
                .with_details(json!({"endpoint": endpoint, "response": value})));
            }
        };
        serde_json::from_value(transaction).map_err(|error| {
            AppError::new(
                ErrorCode::ExternalService,
                format!("Anchor returned an unexpected SEP-24 transaction payload: {error}"),
            )
            .with_details(json!({"endpoint": endpoint}))
        })
    }

    async fn endpoints(&self) -> Result<(String, String), AppError> {
        if let (Some(web_auth_endpoint), Some(sep24_url)) = (
            clean_url(self.web_auth_endpoint.as_ref()),
            clean_url(self.sep24_url.as_ref()),
        ) {
            return Ok((web_auth_endpoint, sep24_url));
        }
        let info = self.fetch_info().await?;
        Ok((info.web_auth_endpoint, info.transfer_server_sep24))
    }
}

fn clean_url(value: Option<&String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn parse_anchor_response(
    response: reqwest::Response,
    context: &str,
) -> Result<Value, AppError> {
    let status = response.status();
    let text = response.text().await?;
    let value: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({"raw": text}));
    if !status.is_success() {
        let message = value
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("Anchor {context} failed with HTTP {status}"));
        let code = if matches!(status.as_u16(), 401 | 403) {
            ErrorCode::AuthenticationFailure
        } else {
            ErrorCode::ExternalService
        };
        return Err(AppError::new(code, message).with_details(json!({
            "status": status.as_u16(),
            "response": value,
        })));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::{parse_account_id, sign_transaction};
    use stellar_xdr::{
        ManageDataOp, Memo, MuxedAccount, Operation, OperationBody, Preconditions, SequenceNumber,
        String64, Transaction, TransactionExt, Uint256,
    };
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const TEST_PASSPHRASE: &str = "Test SDF Network ; September 2015";
    const HOME_DOMAIN: &str = "testanchor.stellar.org";

    fn challenge_xdr(
        server: &StellarKeypair,
        client_account: &str,
        home_domain: &str,
        passphrase: &str,
    ) -> String {
        let operation = Operation {
            source_account: Some(MuxedAccount::Ed25519(Uint256(
                parse_account_id(client_account).unwrap(),
            ))),
            body: OperationBody::ManageData(ManageDataOp {
                data_name: String64::try_from(home_domain.as_bytes().to_vec()).unwrap(),
                data_value: None,
            }),
        };
        let transaction = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256(server.public_bytes())),
            fee: 100,
            seq_num: SequenceNumber(0),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![operation]
                .try_into()
                .expect("one operation fits the XDR limit"),
            ext: TransactionExt::V0,
        };
        let envelope = sign_transaction(transaction, server, passphrase).unwrap();
        envelope_base64(&envelope).unwrap()
    }

    fn provider_with_overrides(server: &MockServer) -> AnchorProvider {
        AnchorProvider::new(
            server.uri(),
            Some(format!("{}/sep24", server.uri())),
            Some(format!("{}/auth", server.uri())),
        )
    }

    fn decode_form_value(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut out = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'%' if index + 3 <= bytes.len() => {
                    let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap();
                    out.push(u8::from_str_radix(hex, 16).unwrap());
                    index += 3;
                }
                b'+' => {
                    out.push(b' ');
                    index += 1;
                }
                byte => {
                    out.push(byte);
                    index += 1;
                }
            }
        }
        String::from_utf8(out).unwrap()
    }

    #[tokio::test]
    async fn fetch_info_parses_stellar_toml_from_full_url_home_domain() {
        let server = MockServer::start().await;
        let toml_body = format!(
            r#"
NETWORK_PASSPHRASE = "{TEST_PASSPHRASE}"
SIGNING_KEY = "GCHLHDBOKG2JWMJQBTLSL5XG6NO7ESXI2TAQKZXCXWXB5WI2X6W233PR"
WEB_AUTH_ENDPOINT = "{uri}/auth"
TRANSFER_SERVER_SEP0024 = "{uri}/sep24"

[[CURRENCIES]]
code = "USDC"
issuer = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5"
status = "test"
anchor_asset_type = "crypto"

[[CURRENCIES]]
code = "native"
status = "test"
anchor_asset_type = "crypto"
"#,
            uri = server.uri()
        );
        Mock::given(method("GET"))
            .and(path("/.well-known/stellar.toml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(toml_body))
            .mount(&server)
            .await;

        let provider = AnchorProvider::new(server.uri(), None, None);
        let info = provider.fetch_info().await.unwrap();

        assert_eq!(info.home_domain, server.uri());
        assert_eq!(info.web_auth_endpoint, format!("{}/auth", server.uri()));
        assert_eq!(
            info.transfer_server_sep24,
            format!("{}/sep24", server.uri())
        );
        assert_eq!(
            info.signing_key.as_deref(),
            Some("GCHLHDBOKG2JWMJQBTLSL5XG6NO7ESXI2TAQKZXCXWXB5WI2X6W233PR")
        );
        assert_eq!(info.currencies.len(), 2);
        assert_eq!(info.currencies[0].code, "USDC");
        assert_eq!(
            info.currencies[0].issuer.as_deref(),
            Some("GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5")
        );
        assert_eq!(info.currencies[1].code, "native");
        assert_eq!(info.currencies[1].issuer, None);
    }

    #[tokio::test]
    async fn fetch_info_endpoint_overrides_win_over_toml() {
        let server = MockServer::start().await;
        let toml_body = format!(
            r#"
WEB_AUTH_ENDPOINT = "{uri}/from-toml-auth"
TRANSFER_SERVER_SEP0024 = "{uri}/from-toml-sep24"
"#,
            uri = server.uri()
        );
        Mock::given(method("GET"))
            .and(path("/.well-known/stellar.toml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(toml_body))
            .mount(&server)
            .await;

        let provider = AnchorProvider::new(
            server.uri(),
            Some(format!("{}/override-sep24", server.uri())),
            Some(format!("{}/override-auth", server.uri())),
        );
        let info = provider.fetch_info().await.unwrap();

        assert_eq!(
            info.web_auth_endpoint,
            format!("{}/override-auth", server.uri())
        );
        assert_eq!(
            info.transfer_server_sep24,
            format!("{}/override-sep24", server.uri())
        );
    }

    #[tokio::test]
    async fn fetch_info_without_sep24_endpoints_is_usage_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/.well-known/stellar.toml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "SIGNING_KEY = \"GCHLHDBOKG2JWMJQBTLSL5XG6NO7ESXI2TAQKZXCXWXB5WI2X6W233PR\"\n",
            ))
            .mount(&server)
            .await;

        let provider = AnchorProvider::new(server.uri(), None, None);
        let error = provider.fetch_info().await.unwrap_err();

        assert_eq!(error.code, ErrorCode::Usage);
        assert!(error.message.contains("does not support SEP-24"));
        assert_eq!(error.details["missing"][0], "WEB_AUTH_ENDPOINT");
        assert_eq!(error.details["missing"][1], "TRANSFER_SERVER_SEP0024");
    }

    #[tokio::test]
    async fn fetch_jwt_signs_sep10_challenge_and_returns_token() {
        let server = MockServer::start().await;
        let server_keypair = StellarKeypair::generate();
        let client_keypair = StellarKeypair::generate();
        let challenge = challenge_xdr(
            &server_keypair,
            &client_keypair.public(),
            HOME_DOMAIN,
            TEST_PASSPHRASE,
        );

        Mock::given(method("GET"))
            .and(path("/auth"))
            .and(query_param("account", client_keypair.public()))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction": challenge,
                "network_passphrase": TEST_PASSPHRASE,
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/auth"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"token": "jwt-token-123"})),
            )
            .mount(&server)
            .await;

        let provider = provider_with_overrides(&server);
        let token = provider
            .fetch_jwt(&client_keypair.public(), &client_keypair, TEST_PASSPHRASE)
            .await
            .unwrap();
        assert_eq!(token, "jwt-token-123");

        let requests = server.received_requests().await.unwrap();
        let posted = requests
            .iter()
            .find(|request| request.method.as_str() == "POST")
            .expect("SEP-10 token request must be posted");
        let body = String::from_utf8_lossy(&posted.body).to_string();
        let encoded = body
            .strip_prefix("transaction=")
            .expect("token request must carry the transaction form field");
        let decoded = decode_form_value(encoded);
        let envelope = TransactionEnvelope::from_xdr_base64(&decoded, Limits::none()).unwrap();
        match envelope {
            TransactionEnvelope::Tx(inner) => assert_eq!(inner.signatures.len(), 2),
            _ => panic!("expected a v1 challenge envelope"),
        }
    }

    #[tokio::test]
    async fn fetch_jwt_rejects_network_passphrase_mismatch() {
        let server = MockServer::start().await;
        let server_keypair = StellarKeypair::generate();
        let client_keypair = StellarKeypair::generate();
        let challenge = challenge_xdr(
            &server_keypair,
            &client_keypair.public(),
            HOME_DOMAIN,
            TEST_PASSPHRASE,
        );

        Mock::given(method("GET"))
            .and(path("/auth"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction": challenge,
                "network_passphrase": "Public Global Stellar Network ; September 2015",
            })))
            .mount(&server)
            .await;

        let provider = provider_with_overrides(&server);
        let error = provider
            .fetch_jwt(&client_keypair.public(), &client_keypair, TEST_PASSPHRASE)
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::AuthenticationFailure);
        assert!(error.message.contains("network passphrase mismatch"));
    }

    #[tokio::test]
    async fn start_deposit_posts_json_and_returns_interactive_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sep24/transactions/deposit/interactive"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "interactive_customer_info_needed",
                "url": "https://anchor.example.com/kyc?tx=1",
                "id": "abc123",
            })))
            .mount(&server)
            .await;

        let provider = provider_with_overrides(&server);
        let deposit = provider
            .start_deposit("jwt-token", "USDC", "GABC", Some("100"))
            .await
            .unwrap();

        assert_eq!(deposit.id, "abc123");
        assert_eq!(deposit.url, "https://anchor.example.com/kyc?tx=1");
        assert_eq!(deposit.kind, "interactive_customer_info_needed");

        let requests = server.received_requests().await.unwrap();
        let posted = requests
            .iter()
            .find(|request| request.method.as_str() == "POST")
            .unwrap();
        let body: Value = posted.body_json().unwrap();
        assert_eq!(body["asset_code"], "USDC");
        assert_eq!(body["account"], "GABC");
        assert_eq!(body["amount"], "100");
    }

    #[tokio::test]
    async fn start_deposit_surfaces_anchor_error_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sep24/transactions/deposit/interactive"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "amount exceeds asset's maximum limit: 100",
            })))
            .mount(&server)
            .await;

        let provider = provider_with_overrides(&server);
        let error = provider
            .start_deposit("jwt-token", "USDC", "GABC", Some("100"))
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert!(error.message.contains("maximum limit"));
        assert_eq!(
            error.details["response"]["error"],
            "amount exceeds asset's maximum limit: 100"
        );
    }

    #[tokio::test]
    async fn transaction_status_parses_anchor_transaction() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sep24/transaction"))
            .and(query_param("id", "abc123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "transaction": {
                    "id": "abc123",
                    "kind": "deposit",
                    "status": "completed",
                    "amount_in": "100.00",
                    "amount_out": "99.50",
                    "asset_code": "USDC",
                    "stellar_transaction_id": "deadbeef",
                    "external_transaction_id": "ext-1",
                    "message": "done",
                },
            })))
            .mount(&server)
            .await;

        let provider = provider_with_overrides(&server);
        let transaction = provider
            .transaction_status("jwt-token", "abc123")
            .await
            .unwrap();

        assert_eq!(transaction.id, "abc123");
        assert_eq!(transaction.kind.as_deref(), Some("deposit"));
        assert_eq!(transaction.status, "completed");
        assert_eq!(transaction.amount_in.as_deref(), Some("100.00"));
        assert_eq!(transaction.amount_out.as_deref(), Some("99.50"));
        assert_eq!(transaction.asset_code.as_deref(), Some("USDC"));
        assert_eq!(
            transaction.stellar_transaction_id.as_deref(),
            Some("deadbeef")
        );
        assert_eq!(
            transaction.external_transaction_id.as_deref(),
            Some("ext-1")
        );
        assert_eq!(transaction.message.as_deref(), Some("done"));
    }

    #[test]
    fn toml_url_uses_https_for_bare_domains() {
        let provider = AnchorProvider::new("anchor.example.com", None, None);
        assert_eq!(
            provider.toml_url(),
            "https://anchor.example.com/.well-known/stellar.toml"
        );
        let provider = AnchorProvider::new("http://127.0.0.1:8080/", None, None);
        assert_eq!(
            provider.toml_url(),
            "http://127.0.0.1:8080/.well-known/stellar.toml"
        );
    }
}
