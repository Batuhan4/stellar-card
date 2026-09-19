use std::time::Duration;

use crate::error::{AppError, ErrorCode};
use crate::fee_contract::parse_contract_id;
use crate::tx::{envelope_base64, sign_transaction, StellarKeypair};
use serde_json::{json, Value};
use stellar_xdr::{
    ContractId, Hash, HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Limits, Memo,
    MuxedAccount, Operation, OperationBody, Preconditions, ReadXdr, ScAddress, ScSymbol, ScVal,
    SequenceNumber, SorobanAuthorizationEntry, SorobanTransactionData, Transaction,
    TransactionEnvelope, TransactionExt, TransactionV1Envelope, Uint256, VecM,
};

const SEND_RETRY_LIMIT: u32 = 3;
const SEND_RETRY_DELAY: Duration = Duration::from_millis(1_000);
const POLL_ATTEMPT_LIMIT: u32 = 60;
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub struct SorobanProvider {
    client: reqwest::Client,
    rpc_url: String,
    network_passphrase: String,
}

#[derive(Debug, Clone)]
pub struct InvokeOutcome {
    pub hash: String,
    pub status: String,
    pub ledger: Option<u32>,
}

impl SorobanProvider {
    pub fn new(rpc_url: impl Into<String>, network_passphrase: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            rpc_url: rpc_url.into(),
            network_passphrase: network_passphrase.into(),
        }
    }

    pub async fn invoke_contract(
        &self,
        contract_id: &str,
        function: &str,
        args: Vec<ScVal>,
        source: &StellarKeypair,
        sequence: i64,
        inclusion_fee: u32,
    ) -> Result<InvokeOutcome, AppError> {
        let contract_bytes = parse_contract_id(contract_id)?;
        let transaction = build_invoke_transaction(
            source,
            sequence,
            inclusion_fee,
            contract_bytes,
            function,
            args,
        )?;
        let unsigned = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx: transaction.clone(),
            signatures: VecM::default(),
        });

        let simulate_response = self
            .rpc_call(
                "simulateTransaction",
                json!({ "transaction": envelope_base64(&unsigned)? }),
            )
            .await?;
        let simulate_result = rpc_result(&simulate_response)?;
        if let Some(error) = simulate_result.get("error") {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!(
                    "Soroban contract simulation failed: {}",
                    error_message(error)
                ),
            )
            .with_details(simulate_response));
        }

        let transaction_data_base64 = simulate_result
            .get("transactionData")
            .and_then(Value::as_str)
            .ok_or_else(|| missing_field(&simulate_response, "transactionData"))?;
        let min_resource_fee = simulate_result
            .get("minResourceFee")
            .and_then(parse_i64)
            .ok_or_else(|| missing_field(&simulate_response, "minResourceFee"))?;
        let auth_entries = parse_auth_entries(&simulate_result)?;

        let transaction_data =
            SorobanTransactionData::from_xdr_base64(transaction_data_base64, Limits::none())
                .map_err(|error| {
                    AppError::new(
                        ErrorCode::ExternalService,
                        format!("Invalid Soroban transaction data: {error}"),
                    )
                })?;
        let resource_fee = u32::try_from(min_resource_fee).map_err(|_| {
            AppError::new(
                ErrorCode::ExternalService,
                format!("Invalid Soroban resource fee: {min_resource_fee}"),
            )
        })?;
        let fee = inclusion_fee.checked_add(resource_fee).ok_or_else(|| {
            AppError::new(
                ErrorCode::ExternalService,
                "Soroban transaction fee overflows u32",
            )
        })?;
        let signed = assemble_signed_transaction(
            transaction,
            transaction_data,
            auth_entries,
            fee,
            source,
            &self.network_passphrase,
        )?;
        let signed_base64 = envelope_base64(&signed)?;

        let mut attempt = 0;
        let (send_response, send_result) = loop {
            let response = self
                .rpc_call("sendTransaction", json!({ "transaction": signed_base64 }))
                .await?;
            let result = rpc_result(&response)?;
            let status = result
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if status == "TRY_AGAIN_LATER" && attempt < SEND_RETRY_LIMIT {
                attempt += 1;
                tokio::time::sleep(SEND_RETRY_DELAY).await;
                continue;
            }
            break (response, result);
        };

        let send_status = send_result
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match send_status {
            "PENDING" | "DUPLICATE" => {}
            "ERROR" => {
                return Err(AppError::new(
                    ErrorCode::ExternalService,
                    "Soroban sendTransaction was rejected",
                )
                .with_details(json!({
                    "response": send_response,
                    "errorResultXdr": send_result
                        .get("errorResultXdr")
                        .cloned()
                        .unwrap_or(Value::Null),
                })));
            }
            "TRY_AGAIN_LATER" => {
                return Err(AppError::new(
                    ErrorCode::ExternalService,
                    "Soroban sendTransaction stayed TRY_AGAIN_LATER after retries",
                )
                .with_details(send_response));
            }
            other => {
                return Err(AppError::new(
                    ErrorCode::ExternalService,
                    format!("Unexpected Soroban sendTransaction status: {other}"),
                )
                .with_details(send_response));
            }
        }

        let hash = send_result
            .get("hash")
            .and_then(Value::as_str)
            .ok_or_else(|| missing_field(&send_response, "hash"))?
            .to_string();

        for attempt in 0..POLL_ATTEMPT_LIMIT {
            if attempt > 0 {
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            let response = self
                .rpc_call("getTransaction", json!({ "hash": hash }))
                .await?;
            let result = rpc_result(&response)?;
            match result
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
            {
                "NOT_FOUND" => continue,
                "SUCCESS" => {
                    let ledger = result
                        .get("ledger")
                        .and_then(Value::as_u64)
                        .map(|ledger| ledger as u32);
                    return Ok(InvokeOutcome {
                        hash,
                        status: "SUCCESS".to_string(),
                        ledger,
                    });
                }
                "FAILED" => {
                    return Err(AppError::new(
                        ErrorCode::ExternalService,
                        "Soroban transaction failed",
                    )
                    .with_details(json!({
                        "response": response,
                        "resultXdr": result
                            .get("resultXdr")
                            .cloned()
                            .unwrap_or(Value::Null),
                        "resultMetaXdr": result
                            .get("resultMetaXdr")
                            .cloned()
                            .unwrap_or(Value::Null),
                    })));
                }
                other => {
                    return Err(AppError::new(
                        ErrorCode::ExternalService,
                        format!("Unexpected Soroban transaction status: {other}"),
                    )
                    .with_details(response));
                }
            }
        }

        Err(AppError::new(
            ErrorCode::Network,
            format!("Timed out waiting for Soroban transaction {hash}"),
        ))
    }

    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value, AppError> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await?;
        let value: Value = serde_json::from_str(&text).map_err(|error| {
            AppError::new(
                ErrorCode::ExternalService,
                format!("Soroban RPC {method} returned invalid JSON: {error}"),
            )
            .with_details(json!({ "status": status.as_u16(), "body": text }))
        })?;
        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Soroban RPC {method} returned HTTP {status}"),
            )
            .with_details(value));
        }
        if let Some(error) = value.get("error") {
            return Err(AppError::new(
                ErrorCode::ExternalService,
                format!("Soroban RPC {method} error: {}", error_message(error)),
            )
            .with_details(value));
        }
        Ok(value)
    }
}

fn build_invoke_transaction(
    source: &StellarKeypair,
    sequence: i64,
    inclusion_fee: u32,
    contract_bytes: [u8; 32],
    function: &str,
    args: Vec<ScVal>,
) -> Result<Transaction, AppError> {
    let function_name = ScSymbol(function.as_bytes().to_vec().try_into().map_err(|_| {
        AppError::new(
            ErrorCode::Usage,
            format!("Invalid Soroban contract function name: {function}"),
        )
    })?);
    let invoke = InvokeContractArgs {
        contract_address: ScAddress::Contract(ContractId(Hash(contract_bytes))),
        function_name,
        args: args.try_into().map_err(|_| {
            AppError::new(
                ErrorCode::ExternalService,
                "Too many Soroban contract arguments",
            )
        })?,
    };
    let operation = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function: HostFunction::InvokeContract(invoke),
            auth: VecM::default(),
        }),
    };
    let seq_num = SequenceNumber(sequence.checked_add(1).ok_or_else(|| {
        AppError::new(
            ErrorCode::Usage,
            "Stellar account sequence number overflows i64",
        )
    })?);
    Ok(Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(source.public_bytes())),
        fee: inclusion_fee,
        seq_num,
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![operation].try_into().map_err(|_| {
            AppError::new(
                ErrorCode::General,
                "Soroban transaction exceeds the operation limit",
            )
        })?,
        ext: TransactionExt::V0,
    })
}

fn assemble_signed_transaction(
    mut transaction: Transaction,
    transaction_data: SorobanTransactionData,
    auth_entries: Vec<SorobanAuthorizationEntry>,
    fee: u32,
    source: &StellarKeypair,
    network_passphrase: &str,
) -> Result<TransactionEnvelope, AppError> {
    transaction.fee = fee;
    let mut operations: Vec<Operation> = transaction.operations.into();
    if let Some(operation) = operations.first_mut() {
        if let OperationBody::InvokeHostFunction(invoke) = &mut operation.body {
            invoke.auth = auth_entries.try_into().map_err(|_| {
                AppError::new(
                    ErrorCode::ExternalService,
                    "Too many Soroban authorization entries",
                )
            })?;
        }
    }
    transaction.operations = operations.try_into().map_err(|_| {
        AppError::new(
            ErrorCode::General,
            "Soroban transaction exceeds the operation limit",
        )
    })?;
    transaction.ext = TransactionExt::V1(transaction_data);
    sign_transaction(transaction, source, network_passphrase)
}

fn parse_auth_entries(simulate_result: &Value) -> Result<Vec<SorobanAuthorizationEntry>, AppError> {
    let Some(entries) = simulate_result
        .get("results")
        .and_then(Value::as_array)
        .and_then(|results| results.first())
        .and_then(|result| result.get("auth"))
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };
    entries
        .iter()
        .map(|entry| {
            let encoded = entry.as_str().ok_or_else(|| {
                AppError::new(
                    ErrorCode::ExternalService,
                    "Soroban auth entry is not a base64 string",
                )
            })?;
            SorobanAuthorizationEntry::from_xdr_base64(encoded, Limits::none()).map_err(|error| {
                AppError::new(
                    ErrorCode::ExternalService,
                    format!("Invalid Soroban auth entry: {error}"),
                )
            })
        })
        .collect()
}

fn rpc_result(response: &Value) -> Result<Value, AppError> {
    response.get("result").cloned().ok_or_else(|| {
        AppError::new(
            ErrorCode::ExternalService,
            "Soroban RPC response is missing a result",
        )
        .with_details(response.clone())
    })
}

fn missing_field(response: &Value, field: &str) -> AppError {
    AppError::new(
        ErrorCode::ExternalService,
        format!("Soroban RPC response is missing {field}"),
    )
    .with_details(response.clone())
}

fn error_message(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| error.to_string())
}

fn parse_i64(value: &Value) -> Option<i64> {
    match value {
        Value::String(text) => text.parse().ok(),
        Value::Number(_) => value.as_i64(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fee_contract::collect_fee_args;
    use stellar_xdr::{
        ContractDataDurability, LedgerFootprint, LedgerKey, LedgerKeyContractCode,
        LedgerKeyContractData, SorobanAuthorizedFunction, SorobanAuthorizedInvocation,
        SorobanCredentials, SorobanResources, SorobanTransactionDataExt, WriteXdr,
    };
    use wiremock::matchers::{body_partial_json, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const NETWORK_PASSPHRASE: &str = "Test SDF Network ; September 2015";
    const CONTRACT_BYTES: [u8; 32] = [9u8; 32];
    const TRANSACTION_HASH: &str =
        "f0e1d2c3b4a5968778695a4b3c2d1e0f112233445566778899aabbccddeeff00";
    const MIN_RESOURCE_FEE: i64 = 12_345;

    fn contract_id() -> String {
        format!("{}", stellar_strkey::Contract(CONTRACT_BYTES))
    }

    fn sample_transaction_data() -> SorobanTransactionData {
        SorobanTransactionData {
            ext: SorobanTransactionDataExt::V0,
            resources: SorobanResources {
                footprint: LedgerFootprint {
                    read_only: vec![LedgerKey::ContractCode(LedgerKeyContractCode {
                        hash: Hash([1u8; 32]),
                    })]
                    .try_into()
                    .unwrap(),
                    read_write: vec![LedgerKey::ContractData(LedgerKeyContractData {
                        contract: ScAddress::Contract(ContractId(Hash(CONTRACT_BYTES))),
                        key: ScVal::LedgerKeyContractInstance,
                        durability: ContractDataDurability::Persistent,
                    })]
                    .try_into()
                    .unwrap(),
                },
                instructions: 2_000_000,
                disk_read_bytes: 512,
                write_bytes: 256,
            },
            resource_fee: MIN_RESOURCE_FEE,
        }
    }

    fn sample_auth_entry() -> SorobanAuthorizationEntry {
        SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                    contract_address: ScAddress::Contract(ContractId(Hash(CONTRACT_BYTES))),
                    function_name: ScSymbol(b"collect_fee".to_vec().try_into().unwrap()),
                    args: VecM::default(),
                }),
                sub_invocations: VecM::default(),
            },
        }
    }

    fn simulate_payload(
        transaction_data: &SorobanTransactionData,
        auth_entries: Vec<SorobanAuthorizationEntry>,
    ) -> Value {
        let auth: Vec<String> = auth_entries
            .iter()
            .map(|entry| entry.to_xdr_base64(Limits::none()).unwrap())
            .collect();
        json!({
            "transactionData": transaction_data.to_xdr_base64(Limits::none()).unwrap(),
            "minResourceFee": MIN_RESOURCE_FEE.to_string(),
            "latestLedger": 100,
            "results": [{ "auth": auth, "xdr": "" }],
        })
    }

    fn rpc_response(result: Value) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": result,
        }))
    }

    async fn mount_rpc(server: &MockServer, method_name: &str, result: Value) {
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "method": method_name })))
            .respond_with(rpc_response(result))
            .mount(server)
            .await;
    }

    fn test_args(source: &StellarKeypair) -> Vec<ScVal> {
        collect_fee_args(&source.public(), 6_000_000, [3u8; 16]).unwrap()
    }

    async fn request_bodies(server: &MockServer, method_name: &str) -> Vec<Value> {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
            .filter(|body| body.get("method").and_then(Value::as_str) == Some(method_name))
            .collect()
    }

    #[tokio::test]
    async fn success_path_returns_hash_status_and_ledger() {
        let server = MockServer::start().await;
        let transaction_data = sample_transaction_data();
        mount_rpc(
            &server,
            "simulateTransaction",
            simulate_payload(&transaction_data, vec![]),
        )
        .await;
        mount_rpc(
            &server,
            "sendTransaction",
            json!({ "hash": TRANSACTION_HASH, "status": "PENDING", "latestLedger": 101 }),
        )
        .await;
        mount_rpc(
            &server,
            "getTransaction",
            json!({ "status": "SUCCESS", "ledger": 4321, "latestLedger": 102 }),
        )
        .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let outcome = provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                test_args(&source),
                &source,
                41,
                100,
            )
            .await
            .unwrap();

        assert_eq!(outcome.hash, TRANSACTION_HASH);
        assert_eq!(outcome.status, "SUCCESS");
        assert_eq!(outcome.ledger, Some(4321));
    }

    #[tokio::test]
    async fn sent_transaction_carries_resources_auth_and_combined_fee() {
        let server = MockServer::start().await;
        let transaction_data = sample_transaction_data();
        let auth_entry = sample_auth_entry();
        mount_rpc(
            &server,
            "simulateTransaction",
            simulate_payload(&transaction_data, vec![auth_entry.clone()]),
        )
        .await;
        mount_rpc(
            &server,
            "sendTransaction",
            json!({ "hash": TRANSACTION_HASH, "status": "PENDING" }),
        )
        .await;
        mount_rpc(
            &server,
            "getTransaction",
            json!({ "status": "SUCCESS", "ledger": 99 }),
        )
        .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let args = test_args(&source);
        provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                args.clone(),
                &source,
                41,
                100,
            )
            .await
            .unwrap();

        let send_requests = request_bodies(&server, "sendTransaction").await;
        assert_eq!(send_requests.len(), 1);
        let encoded = send_requests[0]["params"]["transaction"].as_str().unwrap();
        let envelope = TransactionEnvelope::from_xdr_base64(encoded, Limits::none()).unwrap();

        let TransactionEnvelope::Tx(TransactionV1Envelope { tx, signatures }) = envelope else {
            panic!("expected a v1 transaction envelope");
        };
        assert_eq!(signatures.len(), 1);
        assert_eq!(tx.seq_num, SequenceNumber(42));
        assert_eq!(tx.fee, 100 + MIN_RESOURCE_FEE as u32);
        assert_eq!(tx.ext, TransactionExt::V1(transaction_data));

        assert_eq!(tx.operations.len(), 1);
        let OperationBody::InvokeHostFunction(invoke) = &tx.operations[0].body else {
            panic!("expected an invoke host function operation");
        };
        let HostFunction::InvokeContract(invocation) = &invoke.host_function else {
            panic!("expected an invoke contract host function");
        };
        assert_eq!(
            invocation.contract_address,
            ScAddress::Contract(ContractId(Hash(CONTRACT_BYTES)))
        );
        let function_name: &[u8] = invocation.function_name.as_ref();
        assert_eq!(function_name, b"collect_fee");
        assert_eq!(invocation.args, VecM::try_from(args).unwrap());
        assert_eq!(invoke.auth, VecM::try_from(vec![auth_entry]).unwrap());
    }

    #[tokio::test]
    async fn simulate_contract_error_is_external_service_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_partial_json(
                json!({ "method": "simulateTransaction" }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "error": "HostError: Error(WasmVm, InvalidAction)",
                    "latestLedger": 100,
                    "results": [],
                },
            })))
            .mount(&server)
            .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let error = provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                test_args(&source),
                &source,
                41,
                100,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert!(error.details["result"]["error"]
            .as_str()
            .unwrap()
            .contains("HostError"));
    }

    #[tokio::test]
    async fn failed_ledger_transaction_reports_result_xdr() {
        let server = MockServer::start().await;
        let transaction_data = sample_transaction_data();
        mount_rpc(
            &server,
            "simulateTransaction",
            simulate_payload(&transaction_data, vec![]),
        )
        .await;
        mount_rpc(
            &server,
            "sendTransaction",
            json!({ "hash": TRANSACTION_HASH, "status": "PENDING" }),
        )
        .await;
        mount_rpc(
            &server,
            "getTransaction",
            json!({
                "status": "FAILED",
                "resultXdr": "deadbeef",
                "resultMetaXdr": "cafebabe",
            }),
        )
        .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let error = provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                test_args(&source),
                &source,
                41,
                100,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert_eq!(error.details["resultXdr"], json!("deadbeef"));
        assert_eq!(error.details["resultMetaXdr"], json!("cafebabe"));
    }

    #[tokio::test]
    async fn send_transaction_top_level_error_is_external_service_error() {
        let server = MockServer::start().await;
        let transaction_data = sample_transaction_data();
        mount_rpc(
            &server,
            "simulateTransaction",
            simulate_payload(&transaction_data, vec![]),
        )
        .await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "method": "sendTransaction" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "error": { "code": -32602, "message": "invalid transaction" },
            })))
            .mount(&server)
            .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let error = provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                test_args(&source),
                &source,
                41,
                100,
            )
            .await
            .unwrap_err();

        assert_eq!(error.code, ErrorCode::ExternalService);
        assert_eq!(
            error.details["error"]["message"],
            json!("invalid transaction")
        );
    }

    #[tokio::test]
    async fn not_found_is_retried_until_success() {
        let server = MockServer::start().await;
        let transaction_data = sample_transaction_data();
        mount_rpc(
            &server,
            "simulateTransaction",
            simulate_payload(&transaction_data, vec![]),
        )
        .await;
        mount_rpc(
            &server,
            "sendTransaction",
            json!({ "hash": TRANSACTION_HASH, "status": "PENDING" }),
        )
        .await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "method": "getTransaction" })))
            .respond_with(rpc_response(
                json!({ "status": "NOT_FOUND", "latestLedger": 101 }),
            ))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_rpc(
            &server,
            "getTransaction",
            json!({ "status": "SUCCESS", "ledger": 77 }),
        )
        .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let outcome = provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                test_args(&source),
                &source,
                41,
                100,
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, "SUCCESS");
        assert_eq!(outcome.ledger, Some(77));
        assert_eq!(request_bodies(&server, "getTransaction").await.len(), 2);
    }

    #[tokio::test]
    async fn try_again_later_is_retried_once_then_confirmed() {
        let server = MockServer::start().await;
        let transaction_data = sample_transaction_data();
        mount_rpc(
            &server,
            "simulateTransaction",
            simulate_payload(&transaction_data, vec![]),
        )
        .await;
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "method": "sendTransaction" })))
            .respond_with(rpc_response(
                json!({ "status": "TRY_AGAIN_LATER", "latestLedger": 101 }),
            ))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        mount_rpc(
            &server,
            "sendTransaction",
            json!({ "hash": TRANSACTION_HASH, "status": "PENDING" }),
        )
        .await;
        mount_rpc(
            &server,
            "getTransaction",
            json!({ "status": "SUCCESS", "ledger": 55 }),
        )
        .await;

        let provider = SorobanProvider::new(server.uri(), NETWORK_PASSPHRASE);
        let source = StellarKeypair::generate();
        let outcome = provider
            .invoke_contract(
                &contract_id(),
                "collect_fee",
                test_args(&source),
                &source,
                41,
                100,
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, "SUCCESS");
        assert_eq!(outcome.ledger, Some(55));
        assert_eq!(request_bodies(&server, "sendTransaction").await.len(), 2);
    }
}
