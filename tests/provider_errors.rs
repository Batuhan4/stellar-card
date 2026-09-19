//! Failure-path integration tests for `stellar-card`.
//!
//! The binary-level tests point every provider base URL at a wiremock server
//! via `config set`, so the suite runs fully offline. Provider-level tests
//! exercise the public `stellar_card` library API directly against mocks.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::process::Output;
use stellar_card::error::ErrorCode;
use stellar_card::horizon::HorizonProvider;
use stellar_card::providers::StripeProvider;
use tempfile::TempDir;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TESTNET_NATIVE_CONTRACT_ID: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

fn run(home: &TempDir, args: &[&str]) -> Output {
    let mut command =
        Command::cargo_bin("stellar-card").expect("stellar-card binary must be available");
    command
        .env("STELLAR_CARD_HOME", home.path())
        .env_remove("STELLAR_CARD_API_KEY")
        .args(args);
    command.output().expect("stellar-card must run")
}

fn data(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "stellar-card failed with {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be a JSON envelope");
    assert_eq!(envelope["ok"], true, "envelope: {envelope}");
    envelope["data"].clone()
}

fn error(output: &Output) -> (i32, Value) {
    let code = output
        .status
        .code()
        .expect("stellar-card exited without a status code");
    assert_ne!(code, 0, "expected a failure exit code");
    let envelope: Value =
        serde_json::from_slice(&output.stderr).expect("stderr must be a JSON envelope");
    assert_eq!(envelope["ok"], false, "envelope: {envelope}");
    (code, envelope["error"].clone())
}

fn set_config(home: &TempDir, key: &str, value: &str) {
    data(&run(home, &["config", "set", key, value]));
}

#[tokio::test]
async fn stripe_401_login_surfaces_authentication_failure() {
    let home = TempDir::new().unwrap();
    let stripe = MockServer::start().await;
    set_config(&home, "stripe_base_url", &stripe.uri());

    Mock::given(method("GET"))
        .and(path("/v1/account"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "type": "invalid_request_error",
                "message": "bad api key",
            },
        })))
        .mount(&stripe)
        .await;

    let output = run(
        &home,
        &["auth", "login", "--api-key", "sk_test_demo_123456"],
    );
    let (code, failure) = error(&output);
    assert_eq!(code, 3);
    assert_eq!(failure["code"], "AUTHENTICATION_FAILURE");
    assert!(failure["message"].as_str().unwrap().contains("bad api key"));
    assert_eq!(failure["details"]["error"]["message"], "bad api key");
}

#[tokio::test]
async fn stripe_cardholder_401_surfaces_authentication_failure() {
    let stripe = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/issuing/cardholders"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {"message": "bad api key"},
        })))
        .mount(&stripe)
        .await;

    let provider = StripeProvider::new(stripe.uri(), "sk_test_demo_123456").unwrap();
    let failure = provider
        .create_cardholder("Test User", "test@example.com")
        .await
        .unwrap_err();

    assert_eq!(failure.code, ErrorCode::AuthenticationFailure);
    assert!(failure.message.contains("bad api key"));
}

#[tokio::test]
async fn coinbase_spot_price_is_returned_verbatim() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/prices/XLM-USD/spot"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"amount": "123.45"},
        })))
        .mount(&server)
        .await;

    let provider = HorizonProvider::new(server.uri(), server.uri());

    assert_eq!(provider.fetch_xlm_usd_spot_price().await.unwrap(), "123.45");
}

#[tokio::test]
async fn soroban_rpc_500_fails_card_buy_with_external_service_error() {
    let home = TempDir::new().unwrap();
    let stripe = MockServer::start().await;
    let horizon = MockServer::start().await;
    let coinbase = MockServer::start().await;
    let rpc = MockServer::start().await;

    set_config(&home, "stripe_base_url", &stripe.uri());
    set_config(&home, "horizon_url", &horizon.uri());
    set_config(&home, "coinbase_base_url", &coinbase.uri());
    set_config(&home, "rpc_url", &rpc.uri());
    set_config(&home, "fee_contract_id", TESTNET_NATIVE_CONTRACT_ID);
    set_config(&home, "onchain_fee_collection_enabled", "true");
    set_config(&home, "fee_fixed_cents", "10");
    set_config(&home, "fee_variable_bps", "20");

    let deposit = data(&run(&home, &["deposit", "address", "--asset", "xlm"]));
    let address = deposit["address"].as_str().unwrap().to_string();
    let deposit_id = deposit["id"].as_str().unwrap().to_string();
    data(&run(
        &home,
        &[
            "debug",
            "confirm-deposit",
            deposit_id.as_str(),
            "--amount-usd",
            "25.00",
            "--amount-native",
            "25.0000000",
        ],
    ));

    Mock::given(method("GET"))
        .and(path(format!("/accounts/{address}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "sequence": "41",
            "subentry_count": 0,
            "balances": [{"asset_type": "native", "balance": "100.0000000"}],
        })))
        .mount(&horizon)
        .await;
    Mock::given(method("GET"))
        .and(path("/v2/prices/XLM-USD/spot"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {"amount": "0.20"},
        })))
        .mount(&coinbase)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "simulateTransaction"})))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {"code": -32000, "message": "rpc exploded"},
        })))
        .mount(&rpc)
        .await;

    let output = run(&home, &["card", "buy", "--amount", "20.00"]);
    let (code, failure) = error(&output);
    assert_eq!(code, 7);
    assert_eq!(failure["code"], "EXTERNAL_SERVICE_ERROR");
    assert!(failure["message"].as_str().unwrap().contains("HTTP 500"));
    assert!(failure["details"].is_object());
    assert_eq!(failure["details"]["error"]["message"], "rpc exploded");
}
