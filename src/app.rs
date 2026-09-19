use crate::anchor::AnchorProvider;
use crate::cli::{
    Asset, AuthCommands, CardCommands, Cli, Commands, ConfigCommands, DebugCommands,
    DepositCommands, Network, OnrampCommands, OutputFormat,
};
use crate::error::{AppError, ErrorCode};
use crate::fee_contract::{
    card_fee_cents, collect_fee_args, parse_contract_id, usd_cents_to_stroops,
};
use crate::horizon::HorizonProvider;
use crate::models::{
    AuditEntry, CardRecord, CardState, Config, Credentials, Deposit, DepositState, FeePayment,
    IdempotencyRecord, OnrampRecord, State, BASE_RESERVE_STROOPS,
};
use crate::output::{emit_error, emit_success};
use crate::providers::StripeProvider;
use crate::soroban::SorobanProvider;
use crate::store::{
    load_config, load_credentials, load_state, save_config, save_credentials, save_state, AppPaths,
};
use crate::tx::{
    build_change_trust_tx, envelope_base64, parse_account_id, sign_transaction, StellarKeypair,
    BASE_FEE_STROOPS,
};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::Digest;
use uuid::Uuid;

pub async fn run(cli: Cli) -> i32 {
    match run_inner(cli).await {
        Ok(code) => code,
        Err((format, error)) => emit_error(&format, &error),
    }
}

async fn run_inner(cli: Cli) -> Result<i32, (OutputFormat, AppError)> {
    let paths = AppPaths::discover().map_err(|e| (OutputFormat::Json, e))?;
    let mut config = load_config(&paths).map_err(|e| (OutputFormat::Json, e))?;
    if let Some(network) = cli.network.clone() {
        config.network = network;
    }
    if let Some(format) = cli.format.clone() {
        config.format = format;
    }
    if cli.quiet {
        config.quiet = true;
    }
    let format = config.format.clone();

    let credentials = load_credentials(&paths).map_err(|e| (format.clone(), e))?;
    let state = load_state(&paths).map_err(|e| (format.clone(), e))?;

    let mut ctx = AppContext {
        cli,
        format: format.clone(),
        config,
        credentials,
        state,
        paths,
    };

    let command = ctx.cli.command.clone();
    let data = match command {
        Commands::Auth { command } => ctx.handle_auth(&command).await,
        Commands::Deposit { command } => ctx.handle_deposit(&command).await,
        Commands::Card { command } => ctx.handle_card(&command).await,
        Commands::Onramp { command } => ctx.handle_onramp(&command).await,
        Commands::Balance => ctx.handle_balance().await,
        Commands::Config { command } => ctx.handle_config(&command).await,
        Commands::Debug { command } => ctx.handle_debug(&command).await,
    }
    .map_err(|e| (ctx.format.clone(), e))?;

    emit_success(&ctx.format, &data).map_err(|e| (ctx.format.clone(), e))?;
    Ok(0)
}

struct AppContext {
    cli: Cli,
    format: OutputFormat,
    config: Config,
    credentials: Option<Credentials>,
    state: State,
    paths: AppPaths,
}

impl AppContext {
    async fn handle_auth(&mut self, command: &AuthCommands) -> Result<Value, AppError> {
        match command {
            AuthCommands::Login(args) => {
                let api_key = args
                    .api_key
                    .clone()
                    .or_else(|| self.cli.api_key.clone())
                    .ok_or_else(|| {
                        AppError::new(ErrorCode::Usage, "Missing API key")
                            .with_suggestion("Run: stellar-card auth login --api-key sk_test_...")
                    })?;
                validate_api_key(&api_key, &self.config.network)?;
                let stripe = StripeProvider::new(self.config.stripe_base_url(), api_key.clone())?;
                let account = stripe.validate_key().await?;
                let credentials = Credentials {
                    api_key: api_key.clone(),
                    created_at: now(),
                };
                save_credentials(&self.paths, &credentials)?;
                self.credentials = Some(credentials);
                Ok(json!({
                    "authenticated": true,
                    "network": network_name(&self.config.network),
                    "api_key_masked": mask_api_key(&api_key),
                    "stripe_account_id": account.id,
                    "config_dir": self.paths.base_dir,
                }))
            }
            AuthCommands::Status => Ok(json!({
                "authenticated": self.resolve_api_key().is_ok(),
                "network": network_name(&self.config.network),
                "api_key_masked": self.resolve_api_key().ok().map(|v| mask_api_key(&v)),
                "stellar_wallet_pubkey": self.configured_stellar_pubkey().ok(),
                "config_dir": self.paths.base_dir,
                "stripe_base_url": self.config.stripe_base_url(),
                "coinbase_base_url": self.config.coinbase_base_url(),
                "horizon_url": self.config.horizon_url(),
                "rpc_url": self.config.rpc_url(),
                "network_passphrase": self.config.network_passphrase(),
                "fee_contract_id": self.config.fee_contract_id.clone(),
                "onchain_fee_collection_enabled": self.config.onchain_fee_collection_enabled,
                "fee_fixed_cents": self.config.fee_fixed_cents,
                "fee_variable_bps": self.config.fee_variable_bps,
                "usdc_issuer": self.config.usdc_issuer(),
                "anchor_home_domain": self.config.anchor_home_domain(),
            })),
        }
    }

    async fn handle_deposit(&mut self, command: &DepositCommands) -> Result<Value, AppError> {
        match command {
            DepositCommands::Address(args) => {
                ensure_testnet(&self.config.network)?;
                let deposit = self.resolve_or_create_deposit(&args.asset)?;
                Ok(json!(deposit_response(&deposit)))
            }
            DepositCommands::Status(args) => {
                let deposit = self
                    .refresh_deposit(&args.id, args.wait, args.timeout)
                    .await?;
                let response = deposit_response(deposit);
                if deposit.status == DepositState::Pending {
                    return Err(AppError::new(
                        ErrorCode::DepositPending,
                        format!("Deposit {} is still pending", deposit.id),
                    )
                    .with_details(json!(response))
                    .with_suggestion(
                        "Retry later or run: stellar-card deposit status <id> --wait",
                    ));
                }
                Ok(json!(response))
            }
            DepositCommands::Fund(args) => {
                ensure_testnet(&self.config.network)?;
                let address = self.find_deposit(&args.id)?.address.clone();
                let horizon = self.horizon_provider();
                let transaction_hash = horizon
                    .fund_with_friendbot(self.config.friendbot_url(), &address)
                    .await?;
                self.record_audit(
                    "deposit.funded",
                    "deposit",
                    &args.id,
                    json!({"address": address, "transaction_hash": transaction_hash}),
                );
                self.save_state()?;
                Ok(json!({
                    "id": args.id,
                    "address": address,
                    "funded": true,
                    "transaction_hash": transaction_hash,
                }))
            }
            DepositCommands::Trustline(args) => {
                ensure_testnet(&self.config.network)?;
                let (address, issuer, secret) = {
                    let deposit = self.find_deposit(&args.id)?;
                    if deposit.asset != Asset::Usdc {
                        return Err(AppError::new(
                            ErrorCode::Usage,
                            "Trustlines are only required for USDC deposits",
                        )
                        .with_details(json!({"asset": asset_name(&deposit.asset)})));
                    }
                    (
                        deposit.address.clone(),
                        deposit
                            .token_issuer
                            .clone()
                            .unwrap_or_else(|| self.config.usdc_issuer().to_string()),
                        deposit.secret_key.clone(),
                    )
                };
                let horizon = self.horizon_provider();
                let account = horizon.get_account(&address).await?;
                if !account.exists {
                    return Err(AppError::new(
                        ErrorCode::Usage,
                        "Deposit account is not funded yet",
                    )
                    .with_suggestion(format!("Run: stellar-card deposit fund {}", args.id)));
                }
                let keypair = StellarKeypair::from_secret(&secret)?;
                let transaction =
                    build_change_trust_tx(&keypair, account.sequence, "USDC", &issuer)?;
                let envelope =
                    sign_transaction(transaction, &keypair, self.config.network_passphrase())?;
                let encoded = envelope_base64(&envelope)?;
                let submitted = horizon.submit_transaction(&encoded).await?;
                self.record_audit(
                    "deposit.trustline",
                    "deposit",
                    &args.id,
                    json!({
                        "address": address,
                        "asset_issuer": issuer,
                        "transaction_hash": submitted.hash,
                    }),
                );
                self.save_state()?;
                Ok(json!({
                    "id": args.id,
                    "address": address,
                    "asset": "usdc",
                    "asset_issuer": issuer,
                    "successful": submitted.successful,
                    "transaction_hash": submitted.hash,
                    "ledger": submitted.ledger,
                }))
            }
            DepositCommands::List => {
                let items: Vec<Value> = self
                    .state
                    .deposits
                    .iter()
                    .map(|deposit| json!(deposit_response(deposit)))
                    .collect();
                Ok(json!({"items": items}))
            }
        }
    }

    async fn handle_card(&mut self, command: &CardCommands) -> Result<Value, AppError> {
        match command {
            CardCommands::Buy(args) => {
                ensure_testnet(&self.config.network)?;
                let amount_cents = parse_money_to_cents(&args.amount)?;
                if !(500..=50_000).contains(&amount_cents) {
                    return Err(AppError::new(
                        ErrorCode::Usage,
                        "Card amount must be between $5.00 and $500.00",
                    )
                    .with_details(json!({"amount": args.amount})));
                }
                if let Some(existing_id) = self.find_idempotent_resource("card_buy") {
                    let card = self.find_card(&existing_id)?;
                    return Ok(json!(card_summary(card)));
                }
                self.refresh_all_pending_deposits().await?;
                let fee_cents = self.card_fee_cents(amount_cents as u64) as i64;
                let total_required_cents = amount_cents + fee_cents;
                let balance_cents = self.available_balance_cents();
                if balance_cents < total_required_cents {
                    return Err(AppError::new(
                        ErrorCode::InsufficientBalance,
                        format!(
                            "Account balance (${:.2}) is less than requested total (${:.2}) including fees",
                            balance_cents as f64 / 100.0,
                            total_required_cents as f64 / 100.0
                        ),
                    )
                    .with_details(json!({
                        "balance": format_cents(balance_cents),
                        "requested": format_cents(amount_cents),
                        "fee": format_cents(fee_cents),
                        "total_required": format_cents(total_required_cents),
                        "currency": "usd",
                    }))
                    .with_suggestion(format!(
                        "Deposit at least ${:.2} more. Run: stellar-card deposit address",
                        (total_required_cents - balance_cents) as f64 / 100.0
                    )));
                }
                let fee_payment = self.maybe_collect_onchain_fee(amount_cents as u64).await?;
                let stripe = self.stripe_provider()?;
                let cardholder_id = if let Some(id) = self.state.stripe_cardholder_id.clone() {
                    id
                } else {
                    let cardholder = stripe
                        .create_cardholder(
                            &self.config.cardholder_name,
                            &self.config.cardholder_email,
                        )
                        .await?;
                    self.state.stripe_cardholder_id = Some(cardholder.id.clone());
                    cardholder.id
                };
                let created = stripe.create_virtual_card(&cardholder_id).await?;
                let fixed_fee_cents = self.config.fee_fixed_cents as i64;
                let variable_fee_cents = fee_cents - fixed_fee_cents;
                let card = CardRecord {
                    id: format!("crd_{}", short_id()),
                    stripe_card_id: created.id,
                    stripe_cardholder_id: cardholder_id,
                    amount_usd: format_cents(amount_cents),
                    currency: "usd".to_string(),
                    status: map_card_status(created.status.as_deref()),
                    last4: created.last4,
                    brand: created.brand,
                    exp_month: created.exp_month,
                    exp_year: created.exp_year,
                    fee_fixed_usd: format_cents(fixed_fee_cents),
                    fee_variable_usd: format_cents(variable_fee_cents),
                    fee_total_usd: format_cents(fee_cents),
                    total_debited_usd: format_cents(total_required_cents),
                    fee_stroops: fee_payment.as_ref().map(|p| p.fee_stroops).unwrap_or(0),
                    fee_transaction_hash: fee_payment
                        .as_ref()
                        .map(|payment| payment.fee_transaction_hash.clone()),
                    fee_reference: fee_payment.as_ref().map(|payment| payment.id.clone()),
                    number: created.number,
                    cvc: created.cvc,
                    reveal_unavailable_reason: None,
                    created_at: now(),
                    frozen_at: None,
                };
                let card_id = card.id.clone();
                self.state.cards.push(card);
                self.record_audit(
                    "card.created",
                    "card",
                    &card_id,
                    json!({
                        "amount_usd": format_cents(amount_cents),
                        "fee_usd": format_cents(fee_cents),
                        "fee_transaction_hash": fee_payment.as_ref().map(|payment| payment.fee_transaction_hash.clone())
                    }),
                );
                self.record_idempotency("card_buy", &card_id);
                self.save_state()?;
                let mut response = card_summary(self.find_card(&card_id)?);
                if let Some(payment) = fee_payment {
                    response["fee_payment"] = json!({
                        "transaction_hash": payment.fee_transaction_hash,
                        "fee_usd": payment.fee_usd,
                        "fee_stroops": payment.fee_stroops,
                        "contract_id": payment.contract_id,
                    });
                }
                Ok(json!(response))
            }
            CardCommands::Show(args) => {
                let stripe = self.stripe_provider()?;
                let stripe_id = self.find_card(&args.id)?.stripe_card_id.clone();
                let retrieved = stripe.retrieve_card(&stripe_id).await?;
                let card = self.find_card_mut(&args.id)?;
                card.last4 = retrieved.last4.or(card.last4.clone());
                card.brand = retrieved.brand.or(card.brand.clone());
                card.exp_month = retrieved.exp_month.or(card.exp_month);
                card.exp_year = retrieved.exp_year.or(card.exp_year);
                card.number = retrieved.number.clone();
                card.cvc = retrieved.cvc.clone();
                if card.number.is_none() || card.cvc.is_none() {
                    card.reveal_unavailable_reason = Some("Stripe Issuing test mode does not expose PAN/CVC for virtual cards; summary fields are returned instead.".to_string());
                }
                let response = card_detail(card);
                self.record_audit(
                    "card.revealed",
                    "card",
                    &args.id,
                    json!({"stripe_card_id": stripe_id}),
                );
                self.save_state()?;
                Ok(json!(response))
            }
            CardCommands::List => {
                let items: Vec<Value> = self
                    .state
                    .cards
                    .iter()
                    .map(|card| json!(card_summary(card)))
                    .collect();
                Ok(json!({"items": items}))
            }
            CardCommands::Freeze(args) => {
                if !args.confirm {
                    return Err(AppError::new(
                        ErrorCode::Usage,
                        "Refusing to freeze card without --confirm",
                    )
                    .with_suggestion(format!(
                        "Run: stellar-card card freeze {} --confirm",
                        args.id
                    )));
                }
                if args.dry_run {
                    return Ok(json!({"id": args.id, "action": "freeze", "dry_run": true}));
                }
                let stripe = self.stripe_provider()?;
                let stripe_id = self.find_card(&args.id)?.stripe_card_id.clone();
                let updated = stripe.freeze_card(&stripe_id).await?;
                let card = self.find_card_mut(&args.id)?;
                card.status = CardState::Frozen;
                card.frozen_at = Some(now());
                card.last4 = updated.last4.or(card.last4.clone());
                let response = card_summary(card);
                self.record_audit(
                    "card.frozen",
                    "card",
                    &args.id,
                    json!({"stripe_card_id": stripe_id}),
                );
                self.save_state()?;
                Ok(json!(response))
            }
        }
    }

    async fn handle_onramp(&mut self, command: &OnrampCommands) -> Result<Value, AppError> {
        match command {
            OnrampCommands::Info => {
                ensure_testnet(&self.config.network)?;
                let info = self.anchor_provider().fetch_info().await?;
                Ok(json!(info))
            }
            OnrampCommands::Start(args) => {
                ensure_testnet(&self.config.network)?;
                let deposit = match &args.deposit {
                    Some(id) => self.find_deposit(id)?.clone(),
                    None => self.resolve_or_create_deposit(&args.asset)?,
                };
                if deposit.asset != args.asset {
                    return Err(AppError::new(
                        ErrorCode::Usage,
                        format!(
                            "Deposit {} holds {} but --asset {} was requested",
                            deposit.id,
                            asset_name(&deposit.asset),
                            asset_name(&args.asset)
                        ),
                    )
                    .with_details(json!({
                        "deposit_id": deposit.id,
                        "deposit_asset": asset_name(&deposit.asset),
                        "requested_asset": asset_name(&args.asset),
                    }))
                    .with_suggestion(format!(
                        "Run: stellar-card deposit address --asset {}",
                        asset_name(&args.asset)
                    )));
                }
                let keypair = StellarKeypair::from_secret(&deposit.secret_key)?;
                let info = self.anchor_provider().fetch_info().await?;
                let provider = AnchorProvider::new(
                    info.home_domain.clone(),
                    Some(info.transfer_server_sep24.clone()),
                    Some(info.web_auth_endpoint.clone()),
                );
                let jwt = provider
                    .fetch_jwt(&deposit.address, &keypair, self.config.network_passphrase())
                    .await?;
                let asset_code = anchor_asset_code(&args.asset);
                let started = provider
                    .start_deposit(&jwt, asset_code, &deposit.address, args.amount.as_deref())
                    .await?;
                let record = OnrampRecord {
                    id: format!("ramp_{}", short_id()),
                    deposit_id: Some(deposit.id.clone()),
                    account: deposit.address.clone(),
                    anchor_home_domain: info.home_domain.clone(),
                    anchor_sep24_url: info.transfer_server_sep24.clone(),
                    web_auth_endpoint: info.web_auth_endpoint.clone(),
                    asset_code: asset_code.to_string(),
                    amount: args.amount.clone(),
                    anchor_transaction_id: started.id.clone(),
                    interactive_url: started.url.clone(),
                    status: "incomplete".to_string(),
                    stellar_transaction_id: None,
                    created_at: now(),
                    updated_at: now(),
                };
                let record_id = record.id.clone();
                self.state.onramp_transactions.push(record);
                self.record_audit(
                    "onramp.started",
                    "onramp",
                    &record_id,
                    json!({
                        "deposit_id": deposit.id,
                        "account": deposit.address,
                        "asset_code": asset_code,
                        "amount": args.amount,
                        "anchor_home_domain": info.home_domain,
                        "anchor_transaction_id": started.id,
                        "interactive_url": started.url,
                    }),
                );
                self.save_state()?;
                Ok(json!({
                    "id": record_id,
                    "deposit_id": deposit.id,
                    "account": deposit.address,
                    "asset_code": asset_code,
                    "amount": args.amount,
                    "anchor_transaction_id": started.id,
                    "interactive_url": started.url,
                    "status": "incomplete",
                    "home_domain": info.home_domain,
                    "hint": format!("Complete KYC and funding in the interactive URL, then run: stellar-card onramp status {record_id}"),
                }))
            }
            OnrampCommands::Status(args) => {
                ensure_testnet(&self.config.network)?;
                let record = self.find_onramp(&args.id)?.clone();
                let deposit = match &record.deposit_id {
                    Some(id) => self.find_deposit(id)?.clone(),
                    None => {
                        return Err(AppError::new(
                            ErrorCode::Usage,
                            "On-ramp record has no linked deposit account to re-authenticate with SEP-10",
                        )
                        .with_details(json!({"id": record.id})));
                    }
                };
                let keypair = StellarKeypair::from_secret(&deposit.secret_key)?;
                let provider = AnchorProvider::new(
                    record.anchor_home_domain.clone(),
                    Some(record.anchor_sep24_url.clone()),
                    Some(record.web_auth_endpoint.clone()),
                );
                let jwt = provider
                    .fetch_jwt(&deposit.address, &keypair, self.config.network_passphrase())
                    .await?;
                let transaction = provider
                    .transaction_status(&jwt, &record.anchor_transaction_id)
                    .await?;
                {
                    let stored = self.find_onramp_mut(&args.id)?;
                    stored.status = transaction.status.clone();
                    if let Some(stellar_transaction_id) = &transaction.stellar_transaction_id {
                        stored.stellar_transaction_id = Some(stellar_transaction_id.clone());
                    }
                    stored.updated_at = now();
                }
                if transaction.status == "completed" {
                    self.refresh_deposit(&deposit.id, false, 0).await?;
                }
                self.save_state()?;
                let deposit_json = self.find_deposit(&deposit.id).ok().map(deposit_response);
                let record = self.find_onramp(&args.id)?;
                Ok(json!({
                    "id": record.id,
                    "deposit_id": record.deposit_id,
                    "account": record.account,
                    "asset_code": record.asset_code,
                    "amount": record.amount,
                    "anchor_transaction_id": record.anchor_transaction_id,
                    "interactive_url": record.interactive_url,
                    "status": transaction.status,
                    "amount_in": transaction.amount_in,
                    "amount_out": transaction.amount_out,
                    "stellar_transaction_id": transaction.stellar_transaction_id,
                    "external_transaction_id": transaction.external_transaction_id,
                    "message": transaction.message,
                    "home_domain": record.anchor_home_domain,
                    "updated_at": record.updated_at,
                    "deposit": deposit_json,
                }))
            }
        }
    }

    async fn handle_balance(&mut self) -> Result<Value, AppError> {
        self.refresh_all_pending_deposits().await?;
        Ok(json!({
            "available_usd": format_cents(self.available_balance_cents()),
            "network": network_name(&self.config.network),
            "deposits_confirmed": self.state.deposits.iter().filter(|d| d.status == DepositState::Confirmed).count(),
            "cards_issued": self.state.cards.len(),
            "fees_paid_usd": format_cents(self.total_fee_cents()),
            "onchain_fee_collection_enabled": self.config.onchain_fee_collection_enabled,
        }))
    }

    async fn handle_config(&mut self, command: &ConfigCommands) -> Result<Value, AppError> {
        match command {
            ConfigCommands::Set(args) => {
                match args.key.as_str() {
                    "network" => {
                        self.config.network = match args.value.as_str() {
                            "testnet" => Network::Testnet,
                            "mainnet" => Network::Mainnet,
                            _ => return Err(AppError::new(ErrorCode::Usage, "network must be testnet or mainnet")),
                        }
                    }
                    "format" => {
                        self.config.format = match args.value.as_str() {
                            "json" => OutputFormat::Json,
                            "table" => OutputFormat::Table,
                            "plain" => OutputFormat::Plain,
                            _ => return Err(AppError::new(ErrorCode::Usage, "format must be json, table, or plain")),
                        };
                        self.format = self.config.format.clone();
                    }
                    "horizon_url" => self.config.horizon_url = Some(args.value.clone()),
                    "rpc_url" => self.config.rpc_url = Some(args.value.clone()),
                    "network_passphrase" => {
                        self.config.network_passphrase = Some(args.value.clone())
                    }
                    "stripe_base_url" => self.config.stripe_base_url = Some(args.value.clone()),
                    "coinbase_base_url" => self.config.coinbase_base_url = Some(args.value.clone()),
                    "stellar_private_key" => {
                        StellarKeypair::from_secret(&args.value)?;
                        self.config.stellar_private_key = Some(args.value.clone());
                    }
                    "fee_contract_id" => {
                        parse_contract_id(&args.value)?;
                        self.config.fee_contract_id = Some(args.value.clone());
                    }
                    "onchain_fee_collection_enabled" => {
                        self.config.onchain_fee_collection_enabled = parse_bool(&args.value)?
                    }
                    "fee_fixed_cents" => {
                        self.config.fee_fixed_cents = args.value.parse().map_err(|_| {
                            AppError::new(ErrorCode::Usage, "fee_fixed_cents must be an integer")
                        })?
                    }
                    "fee_variable_bps" => {
                        let bps: u16 = args.value.parse().map_err(|_| {
                            AppError::new(ErrorCode::Usage, "fee_variable_bps must be an integer")
                        })?;
                        if bps > 10_000 {
                            return Err(AppError::new(
                                ErrorCode::Usage,
                                "fee_variable_bps must be between 0 and 10000",
                            ));
                        }
                        self.config.fee_variable_bps = bps;
                    }
                    "xlm_price_usd" => {
                        parse_money_to_cents(&args.value)?;
                        self.config.xlm_price_usd = args.value.clone();
                    }
                    "usdc_issuer" => {
                        parse_account_id(&args.value)?;
                        self.config.usdc_issuer = Some(args.value.clone());
                    }
                    "cardholder_name" => self.config.cardholder_name = args.value.clone(),
                    "cardholder_email" => self.config.cardholder_email = args.value.clone(),
                    "anchor_home_domain" => {
                        let value = args.value.trim();
                        if value.is_empty() {
                            return Err(AppError::new(
                                ErrorCode::Usage,
                                "anchor_home_domain must not be empty",
                            ));
                        }
                        self.config.anchor_home_domain = Some(value.to_string());
                    }
                    "anchor_sep24_url" => {
                        validate_http_url(&args.value)?;
                        self.config.anchor_sep24_url = Some(args.value.clone());
                    }
                    "anchor_web_auth_endpoint" => {
                        validate_http_url(&args.value)?;
                        self.config.anchor_web_auth_endpoint = Some(args.value.clone());
                    }
                    _ => {
                        return Err(AppError::new(ErrorCode::Usage, "Unsupported config key")
                            .with_details(json!({"supported_keys": ["network", "format", "horizon_url", "rpc_url", "network_passphrase", "stripe_base_url", "coinbase_base_url", "stellar_private_key", "fee_contract_id", "onchain_fee_collection_enabled", "fee_fixed_cents", "fee_variable_bps", "xlm_price_usd", "usdc_issuer", "cardholder_name", "cardholder_email", "anchor_home_domain", "anchor_sep24_url", "anchor_web_auth_endpoint"]})))
                    }
                }
                save_config(&self.paths, &self.config)?;
                Ok(
                    json!({"key": args.key, "value": args.value, "config_path": self.paths.config_path}),
                )
            }
        }
    }

    async fn handle_debug(&mut self, command: &DebugCommands) -> Result<Value, AppError> {
        match command {
            DebugCommands::ConfirmDeposit(args) => {
                let response = {
                    let deposit = self.find_deposit_mut(&args.id)?;
                    deposit.status = DepositState::Confirmed;
                    deposit.amount_usd = normalize_money(&args.amount_usd)?;
                    deposit.amount_native = args
                        .amount_native
                        .clone()
                        .unwrap_or_else(|| deposit.amount_usd.clone());
                    deposit.confirmed_at = Some(now());
                    deposit_response(deposit)
                };
                self.save_state()?;
                Ok(json!(response))
            }
        }
    }

    fn horizon_provider(&self) -> HorizonProvider {
        HorizonProvider::new(self.config.horizon_url(), self.config.coinbase_base_url())
    }

    fn soroban_provider(&self) -> SorobanProvider {
        SorobanProvider::new(self.config.rpc_url(), self.config.network_passphrase())
    }

    fn anchor_provider(&self) -> AnchorProvider {
        AnchorProvider::new(
            self.config.anchor_home_domain().to_string(),
            self.config.anchor_sep24_url.clone(),
            self.config.anchor_web_auth_endpoint.clone(),
        )
    }

    fn resolve_api_key(&self) -> Result<String, AppError> {
        if let Some(key) = self.cli.api_key.clone() {
            return Ok(key);
        }
        if let Ok(key) = std::env::var("STELLAR_CARD_API_KEY") {
            return Ok(key);
        }
        if let Some(credentials) = &self.credentials {
            return Ok(credentials.api_key.clone());
        }
        Err(
            AppError::new(ErrorCode::AuthenticationFailure, "No API key configured")
                .with_suggestion("Run: stellar-card auth login --api-key sk_test_..."),
        )
    }

    fn stripe_provider(&self) -> Result<StripeProvider, AppError> {
        let key = self.resolve_api_key()?;
        validate_api_key(&key, &self.config.network)?;
        StripeProvider::new(self.config.stripe_base_url(), key)
    }

    fn configured_stellar_keypair(&self) -> Result<StellarKeypair, AppError> {
        let key = self.config.stellar_private_key.as_deref().ok_or_else(|| {
            AppError::new(ErrorCode::Usage, "No Stellar private key configured")
                .with_suggestion("Run: stellar-card config set stellar_private_key <S...>")
        })?;
        StellarKeypair::from_secret(key)
    }

    fn configured_stellar_pubkey(&self) -> Result<String, AppError> {
        Ok(self.configured_stellar_keypair()?.public())
    }

    fn card_fee_cents(&self, card_amount_cents: u64) -> u64 {
        card_fee_cents(
            card_amount_cents,
            self.config.fee_fixed_cents,
            self.config.fee_variable_bps,
        )
    }

    async fn maybe_collect_onchain_fee(
        &mut self,
        card_amount_cents: u64,
    ) -> Result<Option<FeePayment>, AppError> {
        if !self.config.onchain_fee_collection_enabled {
            return Ok(None);
        }
        let contract_id = self.config.fee_contract_id.clone().ok_or_else(|| {
            AppError::new(
                ErrorCode::Usage,
                "On-chain fee collection is enabled but fee_contract_id is not configured",
            )
            .with_suggestion("Run: stellar-card config set fee_contract_id <deployed-contract-id>")
        })?;
        parse_contract_id(&contract_id)?;

        let fee_usd_cents = self.card_fee_cents(card_amount_cents);
        let horizon = self.horizon_provider();
        let live_price = horizon
            .fetch_xlm_usd_spot_price()
            .await
            .unwrap_or_else(|_| self.config.xlm_price_usd.clone());
        let fee_stroops = usd_cents_to_stroops(fee_usd_cents, &live_price)?;
        let (deposit_id, payer, sequence) = self.find_fee_payer(&horizon, fee_stroops).await?;

        let reference = self
            .cli
            .idempotency_key
            .clone()
            .map(reference_from_text)
            .unwrap_or_else(|| *Uuid::new_v4().as_bytes());
        let args = collect_fee_args(&payer.public(), fee_stroops, reference)?;
        let soroban = self.soroban_provider();
        let outcome = soroban
            .invoke_contract(
                &contract_id,
                "collect_fee",
                args,
                &payer,
                sequence,
                BASE_FEE_STROOPS,
            )
            .await?;

        let payment = FeePayment {
            id: format!("fee_{}", short_id()),
            source_deposit_id: deposit_id.clone(),
            contract_id: contract_id.clone(),
            fee_transaction_hash: outcome.hash.clone(),
            fee_usd: format_cents(fee_usd_cents as i64),
            fee_stroops,
            card_amount_usd: format_cents(card_amount_cents as i64),
            xlm_price_usd: live_price,
            created_at: now(),
        };
        if let Some(deposit) = self
            .state
            .deposits
            .iter_mut()
            .find(|deposit| deposit.id == deposit_id)
        {
            deposit.fee_paid_stroops = deposit.fee_paid_stroops.saturating_add(payment.fee_stroops);
        }
        self.record_audit(
            "fee.collected",
            "fee_payment",
            &payment.id,
            json!({
                "source_deposit_id": payment.source_deposit_id,
                "fee_transaction_hash": payment.fee_transaction_hash,
                "fee_usd": payment.fee_usd,
                "fee_stroops": payment.fee_stroops,
                "contract_id": payment.contract_id,
            }),
        );
        self.state.fee_payments.push(payment.clone());
        self.save_state()?;
        Ok(Some(payment))
    }

    async fn find_fee_payer(
        &self,
        horizon: &HorizonProvider,
        fee_stroops: i64,
    ) -> Result<(String, StellarKeypair, i64), AppError> {
        if let Ok(keypair) = self.configured_stellar_keypair() {
            let public = keypair.public();
            let account = horizon.get_account(&public).await?;
            if account.exists {
                let required =
                    fee_stroops.saturating_add(min_reserve_stroops(account.subentry_count));
                if xlm_to_stroops(&account.native_balance) >= required {
                    return Ok(("configured_wallet".to_string(), keypair, account.sequence));
                }
            }
        }
        for deposit in self.state.deposits.iter().filter(|deposit| {
            deposit.status == DepositState::Confirmed && deposit.asset == Asset::Xlm
        }) {
            let account = horizon.get_account(&deposit.address).await?;
            if !account.exists {
                continue;
            }
            let required = fee_stroops.saturating_add(min_reserve_stroops(account.subentry_count));
            if xlm_to_stroops(&account.native_balance) >= required {
                let keypair = StellarKeypair::from_secret(&deposit.secret_key)?;
                return Ok((deposit.id.clone(), keypair, account.sequence));
            }
        }
        Err(AppError::new(
            ErrorCode::InsufficientBalance,
            "On-chain fee collection requires a confirmed XLM deposit with enough balance for fees",
        )
        .with_details(json!({
            "required_fee_stroops": fee_stroops,
            "fee_contract_id": self.config.fee_contract_id,
        }))
        .with_suggestion(
            "Deposit XLM on testnet before calling card buy, or disable on-chain fee collection",
        ))
    }

    fn find_idempotent_resource(&self, operation: &str) -> Option<String> {
        let key = self.cli.idempotency_key.as_ref()?;
        self.state
            .idempotency_keys
            .iter()
            .find(|record| &record.key == key && record.operation == operation)
            .map(|record| record.resource_id.clone())
    }

    fn record_idempotency(&mut self, operation: &str, resource_id: &str) {
        if let Some(key) = self.cli.idempotency_key.clone() {
            self.state.idempotency_keys.push(IdempotencyRecord {
                key,
                operation: operation.to_string(),
                resource_id: resource_id.to_string(),
                created_at: now(),
            });
        }
    }

    fn record_audit(
        &mut self,
        action: &str,
        resource_type: &str,
        resource_id: &str,
        details: Value,
    ) {
        self.state.audit_log.push(AuditEntry {
            timestamp: now(),
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            details,
        });
    }

    async fn refresh_all_pending_deposits(&mut self) -> Result<(), AppError> {
        let pending_ids: Vec<String> = self
            .state
            .deposits
            .iter()
            .filter(|d| d.status == DepositState::Pending)
            .map(|d| d.id.clone())
            .collect();
        for id in pending_ids {
            let _ = self.refresh_deposit(&id, false, 0).await;
        }
        Ok(())
    }

    async fn refresh_deposit(
        &mut self,
        id: &str,
        wait: bool,
        timeout: u64,
    ) -> Result<&Deposit, AppError> {
        let mut remaining = timeout;
        loop {
            let (address, asset) = {
                let deposit = self.find_deposit(id)?;
                (deposit.address.clone(), deposit.asset.clone())
            };
            let horizon = self.horizon_provider();
            let usdc_issuer = self.config.usdc_issuer().to_string();
            let observed = horizon
                .observe_deposit(&address, &asset, &self.config.xlm_price_usd, &usdc_issuer)
                .await?;
            {
                let deposit = self.find_deposit_mut(id)?;
                let observed_usd_cents = money_to_cents(&observed.amount_usd);
                let current_usd_cents = money_to_cents(&deposit.amount_usd);
                if observed_usd_cents >= current_usd_cents {
                    deposit.amount_native = observed.amount_native;
                    deposit.amount_usd = observed.amount_usd;
                }
                deposit.last_transaction_hash = observed.last_signature;
                if money_to_cents(&deposit.amount_usd) > 0 {
                    deposit.status = DepositState::Confirmed;
                    if deposit.confirmed_at.is_none() {
                        deposit.confirmed_at = Some(now());
                    }
                }
            }
            self.save_state()?;
            if self.find_deposit(id)?.status == DepositState::Confirmed || !wait || remaining == 0 {
                break;
            }
            if !self.config.quiet {
                eprintln!("Waiting for deposit {} on {}...", id, address);
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            remaining = remaining.saturating_sub(5);
        }
        self.find_deposit(id)
    }

    fn resolve_or_create_deposit(&mut self, asset: &Asset) -> Result<Deposit, AppError> {
        if let Some(existing_id) = self.find_idempotent_resource("deposit_address") {
            return Ok(self.find_deposit(&existing_id)?.clone());
        }
        let deposit = self.new_deposit(asset);
        self.record_audit(
            "deposit.created",
            "deposit",
            &deposit.id,
            json!({"asset": asset_name(asset)}),
        );
        self.record_idempotency("deposit_address", &deposit.id);
        self.save_state()?;
        Ok(deposit)
    }

    fn new_deposit(&mut self, asset: &Asset) -> Deposit {
        let keypair = StellarKeypair::generate();
        let configured_keypair = self.configured_stellar_keypair().ok();
        let address = configured_keypair
            .as_ref()
            .map(StellarKeypair::public)
            .unwrap_or_else(|| keypair.public());
        let secret = configured_keypair
            .as_ref()
            .map(StellarKeypair::secret)
            .unwrap_or_else(|| keypair.secret());
        let token_issuer =
            matches!(asset, Asset::Usdc).then(|| self.config.usdc_issuer().to_string());
        let deposit = Deposit {
            id: format!("dep_{}", short_id()),
            asset: asset.clone(),
            network: self.config.network.clone(),
            address,
            token_issuer,
            secret_key: secret,
            created_at: now(),
            status: DepositState::Pending,
            amount_native: "0".to_string(),
            amount_usd: "0.00".to_string(),
            fee_paid_stroops: 0,
            confirmed_at: None,
            last_transaction_hash: None,
        };
        self.state.deposits.push(deposit.clone());
        deposit
    }

    fn available_balance_cents(&self) -> i64 {
        let credits: i64 = self
            .state
            .deposits
            .iter()
            .filter(|deposit| deposit.status == DepositState::Confirmed)
            .map(|deposit| money_to_cents(&deposit.amount_usd))
            .sum();
        let debits: i64 = self.state.cards.iter().map(card_total_debit_cents).sum();
        credits - debits
    }

    fn total_fee_cents(&self) -> i64 {
        self.state
            .cards
            .iter()
            .map(|card| money_to_cents(&card.fee_total_usd))
            .sum()
    }

    fn find_deposit(&self, id: &str) -> Result<&Deposit, AppError> {
        self.state
            .deposits
            .iter()
            .find(|deposit| deposit.id == id)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ResourceNotFound,
                    format!("Deposit {id} not found"),
                )
            })
    }

    fn find_deposit_mut(&mut self, id: &str) -> Result<&mut Deposit, AppError> {
        self.state
            .deposits
            .iter_mut()
            .find(|deposit| deposit.id == id)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ResourceNotFound,
                    format!("Deposit {id} not found"),
                )
            })
    }

    fn find_onramp(&self, id: &str) -> Result<&OnrampRecord, AppError> {
        self.state
            .onramp_transactions
            .iter()
            .find(|record| record.id == id || record.anchor_transaction_id == id)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ResourceNotFound,
                    format!("On-ramp transaction {id} not found"),
                )
                .with_suggestion("Run: stellar-card onramp start --asset usdc")
            })
    }

    fn find_onramp_mut(&mut self, id: &str) -> Result<&mut OnrampRecord, AppError> {
        self.state
            .onramp_transactions
            .iter_mut()
            .find(|record| record.id == id || record.anchor_transaction_id == id)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ResourceNotFound,
                    format!("On-ramp transaction {id} not found"),
                )
            })
    }

    fn find_card(&self, id: &str) -> Result<&CardRecord, AppError> {
        self.state
            .cards
            .iter()
            .find(|card| card.id == id)
            .ok_or_else(|| {
                AppError::new(ErrorCode::ResourceNotFound, format!("Card {id} not found"))
            })
    }

    fn find_card_mut(&mut self, id: &str) -> Result<&mut CardRecord, AppError> {
        self.state
            .cards
            .iter_mut()
            .find(|card| card.id == id)
            .ok_or_else(|| {
                AppError::new(ErrorCode::ResourceNotFound, format!("Card {id} not found"))
            })
    }

    fn save_state(&self) -> Result<(), AppError> {
        save_state(&self.paths, &self.state)
    }
}

fn validate_api_key(api_key: &str, network: &Network) -> Result<(), AppError> {
    if api_key.starts_with("sk_test_") {
        if matches!(network, Network::Mainnet) {
            return Err(AppError::new(
                ErrorCode::AuthenticationFailure,
                "Test-mode Stripe key cannot be used with mainnet",
            )
            .with_details(json!({"network": "mainnet"})));
        }
        return Ok(());
    }
    if api_key.starts_with("sk_live_") {
        return Ok(());
    }
    if api_key.starts_with("test_") && matches!(network, Network::Testnet) {
        return Ok(());
    }
    if api_key.starts_with("live_") && matches!(network, Network::Mainnet) {
        return Ok(());
    }
    Err(AppError::new(
        ErrorCode::AuthenticationFailure,
        "Unsupported API key format",
    )
    .with_details(json!({"expected": ["sk_test_*", "sk_live_*", "test_*", "live_*"]})))
}

fn ensure_testnet(network: &Network) -> Result<(), AppError> {
    if matches!(network, Network::Mainnet) {
        return Err(AppError::new(
            ErrorCode::Usage,
            "CLI MVP direct mode currently supports testnet only",
        )
        .with_suggestion("Run with --network testnet or set config network testnet"));
    }
    Ok(())
}

fn min_reserve_stroops(subentry_count: u32) -> i64 {
    (2 + subentry_count as i64).saturating_mul(BASE_RESERVE_STROOPS)
}

fn xlm_to_stroops(value: &str) -> i64 {
    let trimmed = value.trim();
    let (whole, fraction) = match trimmed.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (trimmed, ""),
    };
    let whole: i64 = whole.parse().unwrap_or(0);
    let mut digits = fraction.chars().take(7).collect::<String>();
    while digits.len() < 7 {
        digits.push('0');
    }
    let fraction: i64 = digits.parse().unwrap_or(0);
    whole.saturating_mul(10_000_000).saturating_add(fraction)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..12].to_string()
}

fn mask_api_key(value: &str) -> String {
    let len = value.len();
    if len <= 8 {
        return "***".to_string();
    }
    format!("{}***{}", &value[..4], &value[len - 4..])
}

fn asset_name(asset: &Asset) -> &'static str {
    match asset {
        Asset::Xlm => "xlm",
        Asset::Usdc => "usdc",
    }
}

fn anchor_asset_code(asset: &Asset) -> &'static str {
    match asset {
        Asset::Xlm => "native",
        Asset::Usdc => "USDC",
    }
}

fn validate_http_url(value: &str) -> Result<(), AppError> {
    let url = reqwest::Url::parse(value).map_err(|_| {
        AppError::new(ErrorCode::Usage, format!("Invalid URL: {value}"))
            .with_details(json!({"value": value}))
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(
            AppError::new(ErrorCode::Usage, "URL must use the http or https scheme")
                .with_details(json!({"value": value, "scheme": url.scheme()})),
        );
    }
    Ok(())
}

fn network_name(network: &Network) -> &'static str {
    match network {
        Network::Testnet => "testnet",
        Network::Mainnet => "mainnet",
    }
}

fn deposit_response(deposit: &Deposit) -> Value {
    json!({
        "id": deposit.id,
        "asset": asset_name(&deposit.asset),
        "status": match deposit.status {
            DepositState::Pending => "pending",
            DepositState::Confirmed => "confirmed",
            DepositState::Expired => "expired",
        },
        "network": network_name(&deposit.network),
        "address": deposit.address,
        "asset_issuer": deposit.token_issuer,
        "amount_native": deposit.amount_native,
        "amount_usd": deposit.amount_usd,
        "fee_paid_stroops": deposit.fee_paid_stroops,
        "created_at": deposit.created_at,
        "confirmed_at": deposit.confirmed_at,
        "last_transaction_hash": deposit.last_transaction_hash,
    })
}

fn card_summary(card: &CardRecord) -> Value {
    json!({
        "id": card.id,
        "stripe_card_id": card.stripe_card_id,
        "status": match card.status {
            CardState::Pending => "pending",
            CardState::Active => "active",
            CardState::Frozen => "frozen",
            CardState::Canceled => "canceled",
        },
        "amount_usd": card.amount_usd,
        "fee_fixed_usd": card.fee_fixed_usd,
        "fee_variable_usd": card.fee_variable_usd,
        "fee_total_usd": card.fee_total_usd,
        "total_debited_usd": card.total_debited_usd,
        "fee_stroops": card.fee_stroops,
        "fee_transaction_hash": card.fee_transaction_hash,
        "fee_reference": card.fee_reference,
        "currency": card.currency,
        "last4": card.last4,
        "brand": card.brand,
        "exp_month": card.exp_month,
        "exp_year": card.exp_year,
        "created_at": card.created_at,
        "frozen_at": card.frozen_at,
    })
}

fn card_detail(card: &CardRecord) -> Value {
    json!({
        "summary": card_summary(card),
        "number": card.number,
        "cvc": card.cvc,
        "reveal_unavailable_reason": card.reveal_unavailable_reason,
    })
}

fn parse_money_to_cents(value: &str) -> Result<i64, AppError> {
    let normalized = normalize_money(value)?;
    Ok(money_to_cents(&normalized))
}

fn normalize_money(value: &str) -> Result<String, AppError> {
    let parsed: f64 = value
        .trim()
        .parse()
        .map_err(|_| AppError::new(ErrorCode::Usage, format!("Invalid money value: {value}")))?;
    Ok(format!("{parsed:.2}"))
}

fn parse_bool(value: &str) -> Result<bool, AppError> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(AppError::new(
            ErrorCode::Usage,
            "Boolean config value must be one of: true, false, 1, 0, yes, no, on, off",
        )),
    }
}

fn money_to_cents(value: &str) -> i64 {
    let parsed = value.parse::<f64>().unwrap_or(0.0);
    (parsed * 100.0).round() as i64
}

fn format_cents(cents: i64) -> String {
    format!("{:.2}", cents as f64 / 100.0)
}

fn card_total_debit_cents(card: &CardRecord) -> i64 {
    if !card.total_debited_usd.is_empty() {
        money_to_cents(&card.total_debited_usd)
    } else {
        money_to_cents(&card.amount_usd) + money_to_cents(&card.fee_total_usd)
    }
}

fn map_card_status(status: Option<&str>) -> CardState {
    match status.unwrap_or("active") {
        "inactive" => CardState::Frozen,
        "canceled" => CardState::Canceled,
        "pending" => CardState::Pending,
        _ => CardState::Active,
    }
}

fn reference_from_text(value: String) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    let digest = sha2::Sha256::digest(value.as_bytes());
    bytes.copy_from_slice(&digest[..16]);
    bytes
}
