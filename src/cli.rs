use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Table,
    Plain,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Testnet,
    Mainnet,
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Asset {
    #[default]
    Xlm,
    Usdc,
}

#[derive(Debug, Parser, Clone)]
#[command(
    name = "stellar-card",
    version,
    about = "Agent-first virtual card CLI funded by Stellar"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub format: Option<OutputFormat>,
    #[arg(long, global = true)]
    pub network: Option<Network>,
    #[arg(long, global = true)]
    pub api_key: Option<String>,
    #[arg(long, global = true, default_value_t = false)]
    pub quiet: bool,
    #[arg(long, global = true)]
    pub idempotency_key: Option<String>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    Deposit {
        #[command(subcommand)]
        command: DepositCommands,
    },
    Card {
        #[command(subcommand)]
        command: CardCommands,
    },
    Onramp {
        #[command(subcommand)]
        command: OnrampCommands,
    },
    Balance,
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    #[command(hide = true)]
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum AuthCommands {
    Login(AuthLoginArgs),
    Status,
}

#[derive(Debug, Args, Clone)]
pub struct AuthLoginArgs {
    #[arg(long)]
    pub api_key: Option<String>,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DepositCommands {
    Address(DepositAddressArgs),
    Status(DepositStatusArgs),
    Fund(DepositIdArgs),
    Trustline(DepositIdArgs),
    List,
}

#[derive(Debug, Args, Clone)]
pub struct DepositAddressArgs {
    #[arg(long, value_enum, default_value_t = Asset::Xlm)]
    pub asset: Asset,
}

#[derive(Debug, Args, Clone)]
pub struct DepositIdArgs {
    pub id: String,
}

#[derive(Debug, Args, Clone)]
pub struct DepositStatusArgs {
    pub id: String,
    #[arg(long, default_value_t = false)]
    pub wait: bool,
    #[arg(long, default_value_t = 300)]
    pub timeout: u64,
}

#[derive(Debug, Subcommand, Clone)]
pub enum CardCommands {
    Buy(CardBuyArgs),
    Show(CardShowArgs),
    List,
    Freeze(CardFreezeArgs),
}

#[derive(Debug, Args, Clone)]
pub struct CardBuyArgs {
    #[arg(long)]
    pub amount: String,
}

#[derive(Debug, Args, Clone)]
pub struct CardShowArgs {
    pub id: String,
}

#[derive(Debug, Args, Clone)]
pub struct CardFreezeArgs {
    pub id: String,
    #[arg(long, default_value_t = false)]
    pub confirm: bool,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum OnrampCommands {
    Info,
    Start(OnrampStartArgs),
    Status(OnrampStatusArgs),
}

#[derive(Debug, Args, Clone)]
pub struct OnrampStartArgs {
    #[arg(long, value_enum, default_value_t = Asset::Usdc)]
    pub asset: Asset,
    #[arg(long)]
    pub amount: Option<String>,
    #[arg(long)]
    pub deposit: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct OnrampStatusArgs {
    pub id: String,
}

#[derive(Debug, Subcommand, Clone)]
pub enum ConfigCommands {
    Set(ConfigSetArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ConfigSetArgs {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Subcommand, Clone)]
pub enum DebugCommands {
    ConfirmDeposit(DebugConfirmDepositArgs),
}

#[derive(Debug, Args, Clone)]
pub struct DebugConfirmDepositArgs {
    pub id: String,
    #[arg(long)]
    pub amount_usd: String,
    #[arg(long)]
    pub amount_native: Option<String>,
}
