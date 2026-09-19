//! Offline integration tests for the `stellar-card onramp` commands.
//!
//! The anchor is a wiremock server configured through `anchor_home_domain`
//! (SEP-1 discovery), its SEP-10/SEP-24 endpoints, and a mocked Horizon
//! server, so these tests never contact a real anchor.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::process::Output;
use stellar_card::tx::{envelope_base64, parse_account_id, sign_transaction, StellarKeypair};
use stellar_xdr::{
    ManageDataOp, Memo, MuxedAccount, Operation, OperationBody, Preconditions, SequenceNumber,
    String64, Transaction, TransactionExt, Uint256,
};
use tempfile::TempDir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_PASSPHRASE: &str = "Test SDF Network ; September 2015";

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

fn challenge_xdr(server: &StellarKeypair, client_account: &str) -> String {
    let operation = Operation {
        source_account: Some(MuxedAccount::Ed25519(Uint256(
            parse_account_id(client_account).unwrap(),
        ))),
        body: OperationBody::ManageData(ManageDataOp {
            data_name: String64::try_from(b"testanchor.stellar.org".to_vec()).unwrap(),
            data_value: None,
        }),
    };
    let transaction = Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(server.public_bytes())),
        fee: 100,
        seq_num: SequenceNumber(0),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![operation].try_into().unwrap(),
        ext: TransactionExt::V0,
    };
    envelope_base64(&sign_transaction(transaction, server, TEST_PASSPHRASE).unwrap()).unwrap()
}

async fn mount_sep1_toml(server: &MockServer) {
    let toml_body = format!(
        r#"
WEB_AUTH_ENDPOINT = "{uri}/auth"
TRANSFER_SERVER_SEP0024 = "{uri}/sep24"
NETWORK_PASSPHRASE = "{TEST_PASSPHRASE}"

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
        .mount(server)
        .await;
}

async fn mount_sep10(server: &MockServer, client_account: &str, server_keypair: &StellarKeypair) {
    let challenge = challenge_xdr(server_keypair, client_account);
    Mock::given(method("GET"))
        .and(path("/auth"))
        .and(query_param("account", client_account))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transaction": challenge,
            "network_passphrase": TEST_PASSPHRASE,
        })))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"token": "jwt-token"})))
        .mount(server)
        .await;
}

async fn mount_interactive_deposit(server: &MockServer, anchor_transaction_id: &str) {
    Mock::given(method("POST"))
        .and(path("/sep24/transactions/deposit/interactive"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "interactive_customer_info_needed",
            "url": "https://anchor.example.com/kyc?tx=1",
            "id": anchor_transaction_id,
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn onramp_info_lists_anchor_currencies() {
    let home = TempDir::new().unwrap();
    let anchor = MockServer::start().await;
    set_config(&home, "anchor_home_domain", &anchor.uri());
    mount_sep1_toml(&anchor).await;

    let info = data(&run(&home, &["onramp", "info"]));

    assert_eq!(info["home_domain"], anchor.uri());
    assert_eq!(info["web_auth_endpoint"], format!("{}/auth", anchor.uri()));
    assert_eq!(
        info["transfer_server_sep24"],
        format!("{}/sep24", anchor.uri())
    );
    let codes: Vec<&str> = info["currencies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|currency| currency["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, vec!["USDC", "native"]);
}

#[tokio::test]
async fn onramp_start_and_status_flow() {
    let home = TempDir::new().unwrap();
    let anchor = MockServer::start().await;
    let horizon = MockServer::start().await;
    set_config(&home, "anchor_home_domain", &anchor.uri());
    set_config(&home, "horizon_url", &horizon.uri());
    mount_sep1_toml(&anchor).await;

    let deposit = data(&run(
        &home,
        &[
            "deposit",
            "address",
            "--asset",
            "usdc",
            "--idempotency-key",
            "onramp-flow-1",
        ],
    ));
    let deposit_id = deposit["id"].as_str().unwrap().to_string();
    let address = deposit["address"].as_str().unwrap().to_string();

    let server_keypair = StellarKeypair::generate();
    mount_sep10(&anchor, &address, &server_keypair).await;
    mount_interactive_deposit(&anchor, "anchor-tx-1").await;

    let started = data(&run(
        &home,
        &[
            "onramp",
            "start",
            "--asset",
            "usdc",
            "--deposit",
            deposit_id.as_str(),
            "--amount",
            "10",
        ],
    ));
    let ramp_id = started["id"].as_str().unwrap().to_string();
    assert!(ramp_id.starts_with("ramp_"));
    assert_eq!(started["deposit_id"], deposit_id);
    assert_eq!(started["account"], address);
    assert_eq!(started["asset_code"], "USDC");
    assert_eq!(started["amount"], "10");
    assert_eq!(started["anchor_transaction_id"], "anchor-tx-1");
    assert_eq!(
        started["interactive_url"],
        "https://anchor.example.com/kyc?tx=1"
    );
    assert_eq!(started["status"], "incomplete");
    assert_eq!(started["home_domain"], anchor.uri());
    assert!(started["hint"]
        .as_str()
        .unwrap()
        .contains("stellar-card onramp status"));

    Mock::given(method("GET"))
        .and(path("/accounts/{address}"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "type": "https://stellar.org/horizon-errors/not_found",
        })))
        .mount(&horizon)
        .await;
    Mock::given(method("GET"))
        .and(path("/sep24/transaction"))
        .and(query_param("id", "anchor-tx-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "transaction": {
                "id": "anchor-tx-1",
                "kind": "deposit",
                "status": "completed",
                "amount_in": "10.00",
                "amount_out": "9.90",
                "asset_code": "USDC",
                "stellar_transaction_id": "deadbeef",
                "external_transaction_id": "ext-42",
            },
        })))
        .mount(&anchor)
        .await;

    let status = data(&run(&home, &["onramp", "status", ramp_id.as_str()]));
    assert_eq!(status["id"], ramp_id);
    assert_eq!(status["anchor_transaction_id"], "anchor-tx-1");
    assert_eq!(status["status"], "completed");
    assert_eq!(status["amount_in"], "10.00");
    assert_eq!(status["amount_out"], "9.90");
    assert_eq!(status["stellar_transaction_id"], "deadbeef");
    assert_eq!(status["external_transaction_id"], "ext-42");
    assert_eq!(
        status["interactive_url"],
        "https://anchor.example.com/kyc?tx=1"
    );
    assert_eq!(status["deposit"]["id"], deposit_id);
    assert_eq!(status["deposit"]["status"], "pending");

    let by_anchor_id = data(&run(&home, &["onramp", "status", "anchor-tx-1"]));
    assert_eq!(by_anchor_id["id"], ramp_id);
    assert_eq!(by_anchor_id["status"], "completed");
}

#[tokio::test]
async fn onramp_start_reuses_idempotent_deposit_and_rejects_asset_mismatch() {
    let home = TempDir::new().unwrap();
    let anchor = MockServer::start().await;
    set_config(&home, "anchor_home_domain", &anchor.uri());
    mount_sep1_toml(&anchor).await;

    let deposit = data(&run(
        &home,
        &[
            "deposit",
            "address",
            "--asset",
            "usdc",
            "--idempotency-key",
            "onramp-flow-2",
        ],
    ));
    let deposit_id = deposit["id"].as_str().unwrap().to_string();
    let address = deposit["address"].as_str().unwrap().to_string();

    let server_keypair = StellarKeypair::generate();
    mount_sep10(&anchor, &address, &server_keypair).await;
    mount_interactive_deposit(&anchor, "anchor-tx-2").await;

    let first = data(&run(
        &home,
        &[
            "onramp",
            "start",
            "--asset",
            "usdc",
            "--amount",
            "5",
            "--idempotency-key",
            "onramp-flow-2",
        ],
    ));
    let second = data(&run(
        &home,
        &[
            "onramp",
            "start",
            "--asset",
            "usdc",
            "--amount",
            "5",
            "--idempotency-key",
            "onramp-flow-2",
        ],
    ));
    assert_eq!(first["deposit_id"], deposit_id);
    assert_eq!(second["deposit_id"], deposit_id);
    assert_ne!(first["id"], second["id"]);

    let deposits = data(&run(&home, &["deposit", "list"]));
    assert_eq!(deposits["items"].as_array().unwrap().len(), 1);

    let output = run(
        &home,
        &[
            "onramp",
            "start",
            "--asset",
            "xlm",
            "--deposit",
            deposit_id.as_str(),
        ],
    );
    let (code, failure) = error(&output);
    assert_eq!(code, 2);
    assert_eq!(failure["code"], "INVALID_ARGUMENTS");
    assert!(failure["message"].as_str().unwrap().contains("holds usdc"));
}
