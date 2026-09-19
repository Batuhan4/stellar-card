use crate::error::{AppError, ErrorCode};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};
use stellar_strkey::ed25519::{PrivateKey, PublicKey as StrkeyPublicKey};
use stellar_xdr::{
    AccountId, AlphaNum4, Asset, AssetCode4, ChangeTrustAsset, ChangeTrustOp, DecoratedSignature,
    Hash, Limits, Memo, MuxedAccount, Operation, OperationBody, PaymentOp, Preconditions,
    PublicKey, SequenceNumber, Signature, SignatureHint, Transaction, TransactionEnvelope,
    TransactionExt, TransactionV1Envelope, Uint256, WriteXdr,
};

pub const BASE_FEE_STROOPS: u32 = 100;

pub struct StellarKeypair {
    signing: SigningKey,
}

impl StellarKeypair {
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).expect("operating system randomness must be available");
        Self {
            signing: SigningKey::from_bytes(&seed),
        }
    }

    pub fn from_secret(secret: &str) -> Result<Self, AppError> {
        let private = PrivateKey::from_string(secret)
            .map_err(|_| AppError::new(ErrorCode::Usage, "Invalid Stellar secret key (S...)"))?;
        Ok(Self {
            signing: SigningKey::from_bytes(&private.0),
        })
    }

    pub fn secret(&self) -> String {
        format!("{}", PrivateKey(self.signing.to_bytes()).as_unredacted())
    }

    pub fn public(&self) -> String {
        format!("{}", StrkeyPublicKey(self.public_bytes()))
    }

    pub fn public_bytes(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }
}

pub fn network_id(passphrase: &str) -> [u8; 32] {
    let digest = Sha256::digest(passphrase.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

pub fn parse_account_id(address: &str) -> Result<[u8; 32], AppError> {
    StrkeyPublicKey::from_string(address)
        .map(|key| key.0)
        .map_err(|_| {
            AppError::new(
                ErrorCode::Usage,
                format!("Invalid Stellar account id: {address}"),
            )
        })
}

pub fn build_payment_tx(
    source: &StellarKeypair,
    sequence: i64,
    destination: &str,
    amount_stroops: i64,
) -> Result<Transaction, AppError> {
    if amount_stroops <= 0 {
        return Err(AppError::new(
            ErrorCode::Usage,
            "Payment amount must be greater than zero",
        ));
    }
    let destination_bytes = parse_account_id(destination)?;
    let operation = Operation {
        source_account: None,
        body: OperationBody::Payment(PaymentOp {
            destination: MuxedAccount::Ed25519(Uint256(destination_bytes)),
            asset: Asset::Native,
            amount: amount_stroops,
        }),
    };
    Ok(base_transaction(source, sequence, vec![operation]))
}

pub fn build_change_trust_tx(
    source: &StellarKeypair,
    sequence: i64,
    asset_code: &str,
    issuer: &str,
) -> Result<Transaction, AppError> {
    let code = asset_code.as_bytes();
    if code.is_empty() || code.len() > 4 {
        return Err(AppError::new(
            ErrorCode::Usage,
            format!("Asset code must be 1-4 characters for credit_alphanum4: {asset_code}"),
        ));
    }
    let mut padded = [0u8; 4];
    padded[..code.len()].copy_from_slice(code);
    let issuer_bytes = parse_account_id(issuer)?;
    let asset = ChangeTrustAsset::CreditAlphanum4(AlphaNum4 {
        asset_code: AssetCode4(padded),
        issuer: AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(issuer_bytes))),
    });
    let operation = Operation {
        source_account: None,
        body: OperationBody::ChangeTrust(ChangeTrustOp {
            line: asset,
            limit: i64::MAX,
        }),
    };
    Ok(base_transaction(source, sequence, vec![operation]))
}

pub fn sign_transaction(
    transaction: Transaction,
    signer: &StellarKeypair,
    passphrase: &str,
) -> Result<TransactionEnvelope, AppError> {
    let id = network_id(passphrase);
    let tx_hash = transaction.hash(id).map_err(|error| {
        AppError::new(
            ErrorCode::General,
            format!("Failed to hash transaction: {error}"),
        )
    })?;
    let decorated = make_signature(signer, &tx_hash)?;
    let envelope = TransactionV1Envelope {
        tx: transaction,
        signatures: vec![decorated].try_into().map_err(|_| {
            AppError::new(ErrorCode::General, "Signature count exceeds envelope limit")
        })?,
    };
    Ok(TransactionEnvelope::Tx(envelope))
}

pub fn sign_envelope(
    envelope: TransactionEnvelope,
    signer: &StellarKeypair,
    passphrase: &str,
) -> Result<TransactionEnvelope, AppError> {
    match envelope {
        TransactionEnvelope::Tx(mut inner) => {
            let id = network_id(passphrase);
            let tx_hash = inner.tx.hash(id).map_err(|error| {
                AppError::new(
                    ErrorCode::General,
                    format!("Failed to hash transaction: {error}"),
                )
            })?;
            let decorated = make_signature(signer, &tx_hash)?;
            let mut signatures = inner.signatures.to_vec();
            signatures.push(decorated);
            inner.signatures = signatures.try_into().map_err(|_| {
                AppError::new(ErrorCode::General, "Signature count exceeds envelope limit")
            })?;
            Ok(TransactionEnvelope::Tx(inner))
        }
        _ => Err(AppError::new(
            ErrorCode::Usage,
            "Only v1 transaction envelopes can be signed",
        )),
    }
}

fn make_signature(
    signer: &StellarKeypair,
    tx_hash: &[u8; 32],
) -> Result<DecoratedSignature, AppError> {
    let signature = signer.signing.sign(tx_hash);
    let mut hint = [0u8; 4];
    hint.copy_from_slice(&signer.public_bytes()[28..32]);
    Ok(DecoratedSignature {
        hint: SignatureHint(hint),
        signature: Signature(
            signature
                .to_bytes()
                .to_vec()
                .try_into()
                .map_err(|_| AppError::new(ErrorCode::General, "Signature length is invalid"))?,
        ),
    })
}

pub fn envelope_base64(envelope: &TransactionEnvelope) -> Result<String, AppError> {
    envelope.to_xdr_base64(Limits::none()).map_err(|error| {
        AppError::new(
            ErrorCode::General,
            format!("Failed to encode envelope: {error}"),
        )
    })
}

fn base_transaction(
    source: &StellarKeypair,
    sequence: i64,
    operations: Vec<Operation>,
) -> Transaction {
    let operation_count = operations.len() as u32;
    Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(source.public_bytes())),
        fee: BASE_FEE_STROOPS.saturating_mul(operation_count.max(1)),
        seq_num: SequenceNumber(sequence.saturating_add(1)),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: operations
            .try_into()
            .expect("operation count is within the XDR limit"),
        ext: TransactionExt::V0,
    }
}

#[allow(dead_code)]
fn account_id_from_bytes(bytes: [u8; 32]) -> AccountId {
    AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(bytes)))
}

#[allow(dead_code)]
fn transaction_hash(transaction: &Transaction, passphrase: &str) -> Result<Hash, AppError> {
    let id = network_id(passphrase);
    transaction.hash(id).map(Hash).map_err(|error| {
        AppError::new(
            ErrorCode::General,
            format!("Failed to hash transaction: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;
    use stellar_xdr::ReadXdr;

    const TEST_PASSPHRASE: &str = "Test SDF Network ; September 2015";

    #[test]
    fn generated_keypair_round_trips_strkey() {
        let keypair = StellarKeypair::generate();
        let secret = keypair.secret();
        let restored = StellarKeypair::from_secret(&secret).unwrap();
        assert_eq!(keypair.public(), restored.public());
        assert!(secret.starts_with('S'));
        assert!(keypair.public().starts_with('G'));
    }

    #[test]
    fn signature_verifies_against_transaction_hash() {
        let keypair = StellarKeypair::generate();
        let tx = build_payment_tx(&keypair, 41, &keypair.public(), 1_000_000).unwrap();
        let envelope = sign_transaction(tx.clone(), &keypair, TEST_PASSPHRASE).unwrap();
        let signatures = match &envelope {
            TransactionEnvelope::Tx(env) => env.signatures.clone(),
            _ => panic!("expected v1 envelope"),
        };
        assert_eq!(signatures.len(), 1);
        let id = network_id(TEST_PASSPHRASE);
        let tx_hash = tx.hash(id).unwrap();
        let signature =
            ed25519_dalek::Signature::from_slice(signatures[0].signature.0.as_ref()).unwrap();
        keypair
            .signing
            .verifying_key()
            .verify(&tx_hash, &signature)
            .unwrap();
    }

    #[test]
    fn envelope_is_base64_xdr_and_parses_back() {
        let keypair = StellarKeypair::generate();
        let tx = build_change_trust_tx(&keypair, 7, "USDC", &keypair.public()).unwrap();
        let envelope = sign_transaction(tx, &keypair, TEST_PASSPHRASE).unwrap();
        let encoded = envelope_base64(&envelope).unwrap();
        let decoded = TransactionEnvelope::from_xdr_base64(&encoded, Limits::none()).unwrap();
        assert_eq!(decoded, envelope);
    }

    #[test]
    fn invalid_secret_is_rejected() {
        let result = StellarKeypair::from_secret("not-a-key");
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, ErrorCode::Usage);
    }
}
