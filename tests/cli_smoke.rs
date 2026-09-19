//! Offline end-to-end smoke tests for the `stellar-card` CLI.
//!
//! Every provider base URL is overridden with `config set` to a wiremock
//! server before any command that touches the network, so these tests never
//! contact Stripe, Horizon, Coinbase, Soroban RPC, or Friendbot.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::process::Output;
use stellar_xdr::{
    LedgerFootprint, Limits, SorobanResources, SorobanTransactionData, SorobanTransactionDataExt,
    WriteXdr,
};
use tempfile::TempDir;
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TESTNET_USDC_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
const TESTNET_NATIVE_CONTRACT_ID: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

// Throwaway test-only keypair generated with:
//   stellar keys generate tmp-stellar-card-test --network testnet \
//     --config-dir /tmp/opencode/stellar-test-keys
// It holds no funds and is never used against a live network.
const TEST_STELLAR_SECRET: &str = "SDPMKUKTWEVCCXRNUZ7W7XWQUQXV2IRPYOCXR3PNRNKDBWFG5OFIXXBG";
const TEST_STELLAR_PUBLIC: &str = "GAIS4GAVXKBLLHKEQROOJB7LUZQXFBIULSC7IKQXKZXVRON4EWONYB3B";

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

async fn mount_stripe_happy_path(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/v1/account"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "acct_test_123"})))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"object": "balance"})))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/issuing/cardholders"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "ich_test_123"})))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/issuing/cards"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ic_test_123",
            "status": "active",
            "last4": "4242",
            "brand": "Visa",
            "exp_month": 4,
            "exp_year": 2029,
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/issuing/cards/ic_test_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ic_test_123",
            "status": "active",
            "last4": "4242",
            "brand": "Visa",
            "exp_month": 4,
            "exp_year": 2029,
            "number": "4242424242424242",
            "cvc": "123",
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/issuing/cards/ic_test_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "ic_test_123",
            "status": "inactive",
            "last4": "4242",
        })))
        .mount(server)
        .await;
}

fn sample_transaction_data() -> SorobanTransactionData {
    SorobanTransactionData {
        ext: SorobanTransactionDataExt::V0,
        resources: SorobanResources {
            footprint: LedgerFootprint {
                read_only: Default::default(),
                read_write: Default::default(),
            },
            instructions: 0,
            disk_read_bytes: 0,
            write_bytes: 0,
        },
        resource_fee: 0,
    }
}

fn rpc_response(result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": result,
    }))
}

#[tokio::test]
async fn full_mocked_flow_without_soroban() {
    let home = TempDir::new().unwrap();
    let stripe = MockServer::start().await;
    let horizon = MockServer::start().await;
    let coinbase = MockServer::start().await;

    let status = data(&run(&home, &["auth", "status"]));
    assert_eq!(status["authenticated"], false);

    set_config(&home, "stripe_base_url", &stripe.uri());
    set_config(&home, "horizon_url", &horizon.uri());
    set_config(&home, "coinbase_base_url", &coinbase.uri());

    let configured = data(&run(&home, &["auth", "status"]));
    assert_eq!(configured["stripe_base_url"], stripe.uri());
    assert_eq!(configured["horizon_url"], horizon.uri());
    assert_eq!(configured["coinbase_base_url"], coinbase.uri());

    mount_stripe_happy_path(&stripe).await;

    let login = data(&run(
        &home,
        &["auth", "login", "--api-key", "sk_test_demo_123456"],
    ));
    assert_eq!(login["authenticated"], true);
    assert_eq!(login["stripe_account_id"], "acct_test_123");

    let deposit = data(&run(
        &home,
        &[
            "deposit",
            "address",
            "--asset",
            "usdc",
            "--idempotency-key",
            "dep-key-1",
        ],
    ));
    let address = deposit["address"].as_str().unwrap().to_string();
    let deposit_id = deposit["id"].as_str().unwrap().to_string();
    assert_eq!(deposit["asset"], "usdc");
    assert!(address.starts_with('G'));
    assert_eq!(deposit["asset_issuer"], TESTNET_USDC_ISSUER);

    Mock::given(method("GET"))
        .and(path(format!("/accounts/{address}")))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "type": "https://stellar.org/horizon-errors/not_found",
            "title": "Resource Missing",
        })))
        .mount(&horizon)
        .await;

    let pending = run(&home, &["deposit", "status", deposit_id.as_str()]);
    let (code, pending_error) = error(&pending);
    assert_eq!(code, 10);
    assert_eq!(pending_error["code"], "DEPOSIT_PENDING");
    assert_eq!(pending_error["details"]["id"], deposit_id);

    let confirmed = data(&run(
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
    assert_eq!(confirmed["status"], "confirmed");
    assert_eq!(confirmed["amount_usd"], "25.00");

    let balance = data(&run(&home, &["balance"]));
    assert_eq!(balance["available_usd"], "25.00");
    assert_eq!(balance["deposits_confirmed"], 1);

    let card = data(&run(
        &home,
        &[
            "card",
            "buy",
            "--amount",
            "20.00",
            "--idempotency-key",
            "buy-key-1",
        ],
    ));
    let card_id = card["id"].as_str().unwrap().to_string();
    assert_eq!(card["last4"], "4242");
    assert_eq!(card["status"], "active");

    let shown = data(&run(&home, &["card", "show", card_id.as_str()]));
    assert_eq!(shown["number"], "4242424242424242");
    assert_eq!(shown["cvc"], "123");

    let frozen = data(&run(
        &home,
        &["card", "freeze", card_id.as_str(), "--confirm"],
    ));
    assert_eq!(frozen["status"], "frozen");

    let list = data(&run(&home, &["card", "list"]));
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn horizon_observation_confirms_xlm_deposit() {
    let home = TempDir::new().unwrap();
    let horizon = MockServer::start().await;
    let coinbase = MockServer::start().await;

    set_config(&home, "horizon_url", &horizon.uri());
    set_config(&home, "coinbase_base_url", &coinbase.uri());

    let deposit = data(&run(&home, &["deposit", "address", "--asset", "xlm"]));
    let address = deposit["address"].as_str().unwrap().to_string();
    let deposit_id = deposit["id"].as_str().unwrap().to_string();

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
        .and(path(format!("/accounts/{address}/transactions")))
        .and(query_param("order", "desc"))
        .and(query_param("limit", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "_embedded": {"records": [{"hash": "abc123"}]},
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

    let observed = data(&run(&home, &["deposit", "status", deposit_id.as_str()]));
    assert_eq!(observed["status"], "confirmed");
    assert_eq!(observed["amount_usd"], "20.00");
    assert_eq!(observed["amount_native"], "100.0000000");
    assert_eq!(observed["last_transaction_hash"], "abc123");
}

#[tokio::test]
async fn configured_secret_is_used_for_deposit_address() {
    let home = TempDir::new().unwrap();

    set_config(&home, "stellar_private_key", TEST_STELLAR_SECRET);

    let deposit = data(&run(&home, &["deposit", "address", "--asset", "xlm"]));
    assert_eq!(deposit["address"], TEST_STELLAR_PUBLIC);

    let status = data(&run(&home, &["auth", "status"]));
    assert_eq!(status["stellar_wallet_pubkey"], TEST_STELLAR_PUBLIC);
}

#[tokio::test]
async fn onchain_fee_collection_uses_mocked_soroban_rpc() {
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

    mount_stripe_happy_path(&stripe).await;
    data(&run(
        &home,
        &["auth", "login", "--api-key", "sk_test_demo_123456"],
    ));

    let deposit = data(&run(&home, &["deposit", "address", "--asset", "xlm"]));
    let address = deposit["address"].as_str().unwrap().to_string();
    let deposit_id = deposit["id"].as_str().unwrap().to_string();

    let confirmed = data(&run(
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
    assert_eq!(confirmed["status"], "confirmed");

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

    let transaction_data = sample_transaction_data()
        .to_xdr_base64(Limits::none())
        .expect("Soroban transaction data must encode to base64 XDR");
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "simulateTransaction"})))
        .respond_with(rpc_response(json!({
            "minResourceFee": "12345",
            "transactionData": transaction_data,
            "results": [{"auth": [], "xdr": null}],
            "latestLedger": 1,
        })))
        .mount(&rpc)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "sendTransaction"})))
        .respond_with(rpc_response(json!({
            "hash": "feehash123",
            "status": "PENDING",
            "latestLedger": 1,
        })))
        .mount(&rpc)
        .await;
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "getTransaction"})))
        .respond_with(rpc_response(json!({
            "status": "SUCCESS",
            "hash": "feehash123",
            "ledger": 42,
            "latestLedger": 2,
        })))
        .mount(&rpc)
        .await;

    let card = data(&run(
        &home,
        &[
            "card",
            "buy",
            "--amount",
            "20.00",
            "--idempotency-key",
            "buy-key-1",
        ],
    ));
    assert_eq!(card["last4"], "4242");
    assert_eq!(card["fee_payment"]["transaction_hash"], "feehash123");
    assert_eq!(card["fee_payment"]["fee_stroops"], 7_000_000);
    assert!(card["fee_stroops"].as_i64().unwrap() > 0);
    assert_eq!(card["fee_total_usd"], "0.14");
}
