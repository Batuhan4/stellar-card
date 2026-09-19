use crate::cli::{Asset, Network, OutputFormat};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const TESTNET_HORIZON_URL: &str = "https://horizon-testnet.stellar.org";
pub const MAINNET_HORIZON_URL: &str = "https://horizon.stellar.org";
pub const TESTNET_RPC_URL: &str = "https://soroban-testnet.stellar.org";
pub const TESTNET_FRIENDBOT_URL: &str = "https://friendbot.stellar.org";
pub const TESTNET_PASSPHRASE: &str = "Test SDF Network ; September 2015";
pub const MAINNET_PASSPHRASE: &str = "Public Global Stellar Network ; September 2015";
pub const TESTNET_USDC_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
pub const COINBASE_API_URL: &str = "https://api.coinbase.com";
pub const BASE_RESERVE_STROOPS: i64 = 500_000;
pub const DEFAULT_ANCHOR_HOME_DOMAIN: &str = "testanchor.stellar.org";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub network: Network,
    pub format: OutputFormat,
    pub quiet: bool,
    pub horizon_url: Option<String>,
    pub rpc_url: Option<String>,
    pub network_passphrase: Option<String>,
    pub stripe_base_url: Option<String>,
    pub coinbase_base_url: Option<String>,
    pub stellar_private_key: Option<String>,
    pub fee_contract_id: Option<String>,
    pub onchain_fee_collection_enabled: bool,
    pub fee_fixed_cents: u64,
    pub fee_variable_bps: u16,
    pub xlm_price_usd: String,
    pub usdc_issuer: Option<String>,
    pub cardholder_name: String,
    pub cardholder_email: String,
    pub anchor_home_domain: Option<String>,
    pub anchor_sep24_url: Option<String>,
    pub anchor_web_auth_endpoint: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: Network::Testnet,
            format: OutputFormat::Json,
            quiet: false,
            horizon_url: None,
            rpc_url: None,
            network_passphrase: None,
            stripe_base_url: None,
            coinbase_base_url: None,
            stellar_private_key: None,
            fee_contract_id: None,
            onchain_fee_collection_enabled: false,
            fee_fixed_cents: 10,
            fee_variable_bps: 20,
            xlm_price_usd: "0.20".to_string(),
            usdc_issuer: None,
            cardholder_name: "StellarCard Demo User".to_string(),
            cardholder_email: "demo@stellar-card.dev".to_string(),
            anchor_home_domain: None,
            anchor_sep24_url: None,
            anchor_web_auth_endpoint: None,
        }
    }
}

impl Config {
    pub fn horizon_url(&self) -> &str {
        self.horizon_url.as_deref().unwrap_or(match self.network {
            Network::Testnet => TESTNET_HORIZON_URL,
            Network::Mainnet => MAINNET_HORIZON_URL,
        })
    }

    pub fn rpc_url(&self) -> &str {
        self.rpc_url.as_deref().unwrap_or(TESTNET_RPC_URL)
    }

    pub fn network_passphrase(&self) -> &str {
        self.network_passphrase
            .as_deref()
            .unwrap_or(match self.network {
                Network::Testnet => TESTNET_PASSPHRASE,
                Network::Mainnet => MAINNET_PASSPHRASE,
            })
    }

    pub fn friendbot_url(&self) -> &str {
        TESTNET_FRIENDBOT_URL
    }

    pub fn usdc_issuer(&self) -> &str {
        self.usdc_issuer.as_deref().unwrap_or(TESTNET_USDC_ISSUER)
    }

    pub fn stripe_base_url(&self) -> &str {
        self.stripe_base_url
            .as_deref()
            .unwrap_or("https://api.stripe.com")
    }

    pub fn coinbase_base_url(&self) -> &str {
        self.coinbase_base_url
            .as_deref()
            .unwrap_or(COINBASE_API_URL)
    }

    pub fn anchor_home_domain(&self) -> &str {
        self.anchor_home_domain
            .as_deref()
            .unwrap_or(DEFAULT_ANCHOR_HOME_DOMAIN)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub api_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct State {
    pub stripe_cardholder_id: Option<String>,
    pub deposits: Vec<Deposit>,
    pub cards: Vec<CardRecord>,
    pub fee_payments: Vec<FeePayment>,
    pub audit_log: Vec<AuditEntry>,
    pub idempotency_keys: Vec<IdempotencyRecord>,
    #[serde(default)]
    pub onramp_transactions: Vec<OnrampRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deposit {
    pub id: String,
    pub asset: Asset,
    pub network: Network,
    pub address: String,
    pub token_issuer: Option<String>,
    pub secret_key: String,
    pub created_at: String,
    pub status: DepositState,
    pub amount_native: String,
    pub amount_usd: String,
    #[serde(default)]
    pub fee_paid_stroops: i64,
    pub confirmed_at: Option<String>,
    pub last_transaction_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DepositState {
    Pending,
    Confirmed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRecord {
    pub id: String,
    pub stripe_card_id: String,
    pub stripe_cardholder_id: String,
    pub amount_usd: String,
    pub currency: String,
    pub status: CardState,
    pub last4: Option<String>,
    pub brand: Option<String>,
    pub exp_month: Option<u32>,
    pub exp_year: Option<u32>,
    #[serde(default)]
    pub fee_fixed_usd: String,
    #[serde(default)]
    pub fee_variable_usd: String,
    #[serde(default)]
    pub fee_total_usd: String,
    #[serde(default)]
    pub total_debited_usd: String,
    #[serde(default)]
    pub fee_stroops: i64,
    #[serde(default)]
    pub fee_transaction_hash: Option<String>,
    #[serde(default)]
    pub fee_reference: Option<String>,
    pub number: Option<String>,
    pub cvc: Option<String>,
    pub reveal_unavailable_reason: Option<String>,
    pub created_at: String,
    pub frozen_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeePayment {
    pub id: String,
    pub source_deposit_id: String,
    pub contract_id: String,
    pub fee_transaction_hash: String,
    pub fee_usd: String,
    pub fee_stroops: i64,
    pub card_amount_usd: String,
    pub xlm_price_usd: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CardState {
    Pending,
    Active,
    Frozen,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    pub key: String,
    pub operation: String,
    pub resource_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnrampRecord {
    pub id: String,
    pub deposit_id: Option<String>,
    pub account: String,
    pub anchor_home_domain: String,
    pub anchor_sep24_url: String,
    pub web_auth_endpoint: String,
    pub asset_code: String,
    pub amount: Option<String>,
    pub anchor_transaction_id: String,
    pub interactive_url: String,
    pub status: String,
    pub stellar_transaction_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
