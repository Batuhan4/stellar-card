#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, symbol_short, token,
    Address, BytesN, Env, Symbol,
};

const STATE_KEY: Symbol = symbol_short!("STATE");

const MAX_FEE_BPS: u32 = 10_000;

const LEDGERS_PER_DAY: u32 = 17_280;
const TTL_THRESHOLD: u32 = LEDGERS_PER_DAY * 7;
const TTL_EXTEND_TO: u32 = LEDGERS_PER_DAY * 30;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultState {
    pub authority: Address,
    pub native_token: Address,
    pub fee_bps: u32,
    pub fixed_fee_cents: u32,
    pub total_collected: i128,
    pub total_withdrawn: i128,
    pub payment_count: u64,
    pub created_at: u64,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidFeeBps = 3,
    InvalidAmount = 4,
    MathOverflow = 5,
    InsufficientVault = 6,
    Unauthorized = 7,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    pub authority: Address,
    pub native_token: Address,
    pub fee_bps: u32,
    pub fixed_fee_cents: u32,
    pub created_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollected {
    #[topic]
    pub payer: Address,
    pub amount: i128,
    pub fee_reference: BytesN<16>,
    pub payment_count: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdrawn {
    #[topic]
    pub to: Address,
    pub amount: i128,
    pub total_withdrawn: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricingUpdated {
    pub fee_bps: u32,
    pub fixed_fee_cents: u32,
}

#[contract]
pub struct FeeVault;

#[contractimpl]
impl FeeVault {
    pub fn initialize(
        env: Env,
        authority: Address,
        native_token: Address,
        fee_bps: u32,
        fixed_fee_cents: u32,
    ) -> Result<(), Error> {
        authority.require_auth();

        if env.storage().instance().has(&STATE_KEY) {
            return Err(Error::AlreadyInitialized);
        }
        if fee_bps > MAX_FEE_BPS {
            return Err(Error::InvalidFeeBps);
        }

        let created_at = env.ledger().timestamp();
        let state = VaultState {
            authority: authority.clone(),
            native_token: native_token.clone(),
            fee_bps,
            fixed_fee_cents,
            total_collected: 0,
            total_withdrawn: 0,
            payment_count: 0,
            created_at,
        };
        Self::save_state(&env, &state);

        Initialized {
            authority,
            native_token,
            fee_bps,
            fixed_fee_cents,
            created_at,
        }
        .publish(&env);

        Ok(())
    }

    pub fn collect_fee(
        env: Env,
        payer: Address,
        amount: i128,
        fee_reference: BytesN<16>,
    ) -> Result<(), Error> {
        let mut state = Self::load_state(&env)?;

        payer.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        state.total_collected = state
            .total_collected
            .checked_add(amount)
            .ok_or(Error::MathOverflow)?;
        state.payment_count = state
            .payment_count
            .checked_add(1)
            .ok_or(Error::MathOverflow)?;

        token::Client::new(&env, &state.native_token).transfer(
            &payer,
            env.current_contract_address(),
            &amount,
        );
        Self::save_state(&env, &state);

        FeeCollected {
            payer,
            amount,
            fee_reference,
            payment_count: state.payment_count,
        }
        .publish(&env);

        Ok(())
    }

    pub fn withdraw(env: Env, to: Address, amount: i128) -> Result<(), Error> {
        let mut state = Self::load_state(&env)?;

        state.authority.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let available = state
            .total_collected
            .checked_sub(state.total_withdrawn)
            .ok_or(Error::MathOverflow)?;
        if amount > available {
            return Err(Error::InsufficientVault);
        }

        state.total_withdrawn = state
            .total_withdrawn
            .checked_add(amount)
            .ok_or(Error::MathOverflow)?;

        token::Client::new(&env, &state.native_token).transfer(
            &env.current_contract_address(),
            &to,
            &amount,
        );
        Self::save_state(&env, &state);

        Withdrawn {
            to,
            amount,
            total_withdrawn: state.total_withdrawn,
        }
        .publish(&env);

        Ok(())
    }

    pub fn update_pricing(env: Env, fee_bps: u32, fixed_fee_cents: u32) -> Result<(), Error> {
        let mut state = Self::load_state(&env)?;

        state.authority.require_auth();

        if fee_bps > MAX_FEE_BPS {
            return Err(Error::InvalidFeeBps);
        }

        state.fee_bps = fee_bps;
        state.fixed_fee_cents = fixed_fee_cents;
        Self::save_state(&env, &state);

        PricingUpdated {
            fee_bps,
            fixed_fee_cents,
        }
        .publish(&env);

        Ok(())
    }

    pub fn get_state(env: Env) -> Result<VaultState, Error> {
        Self::load_state(&env)
    }
}

impl FeeVault {
    fn load_state(env: &Env) -> Result<VaultState, Error> {
        env.storage()
            .instance()
            .get(&STATE_KEY)
            .ok_or(Error::NotInitialized)
    }

    fn save_state(env: &Env, state: &VaultState) {
        env.storage().instance().set(&STATE_KEY, state);
        env.storage()
            .instance()
            .extend_ttl(TTL_THRESHOLD, TTL_EXTEND_TO);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _, MockAuth, MockAuthInvoke},
        token::{StellarAssetClient, TokenClient},
        IntoVal,
    };

    fn setup<'a>(
        env: &'a Env,
    ) -> (
        FeeVaultClient<'a>,
        Address,
        Address,
        Address,
        StellarAssetClient<'a>,
    ) {
        let contract_id = env.register(FeeVault, ());
        let client = FeeVaultClient::new(env, &contract_id);
        let authority = Address::generate(env);
        let payer = Address::generate(env);
        let token_id = env
            .register_stellar_asset_contract_v2(authority.clone())
            .address();
        let token = StellarAssetClient::new(env, &token_id);
        (client, authority, payer, token_id, token)
    }

    fn fee_ref(env: &Env, tag: u8) -> BytesN<16> {
        BytesN::from_array(env, &[tag; 16])
    }

    #[test]
    fn initialize_stores_state() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_timestamp(1_700_000_000);
        let (client, authority, _payer, token_id, _token) = setup(&env);

        client.initialize(&authority, &token_id, &250, &50);

        let state = client.get_state();
        assert_eq!(state.authority, authority);
        assert_eq!(state.native_token, token_id);
        assert_eq!(state.fee_bps, 250);
        assert_eq!(state.fixed_fee_cents, 50);
        assert_eq!(state.total_collected, 0);
        assert_eq!(state.total_withdrawn, 0);
        assert_eq!(state.payment_count, 0);
        assert_eq!(state.created_at, 1_700_000_000);
    }

    #[test]
    fn initialize_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, _payer, token_id, _token) = setup(&env);

        client.initialize(&authority, &token_id, &250, &50);

        assert_eq!(
            client.try_initialize(&authority, &token_id, &250, &50),
            Err(Ok(Error::AlreadyInitialized))
        );
    }

    #[test]
    fn initialize_invalid_fee_bps_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, _payer, token_id, _token) = setup(&env);

        assert_eq!(
            client.try_initialize(&authority, &token_id, &10_001, &0),
            Err(Ok(Error::InvalidFeeBps))
        );

        client.initialize(&authority, &token_id, &10_000, &0);
        assert_eq!(client.get_state().fee_bps, 10_000);
    }

    #[test]
    fn get_state_uninitialized_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _authority, _payer, _token_id, _token) = setup(&env);

        assert_eq!(client.try_get_state(), Err(Ok(Error::NotInitialized)));
    }

    #[test]
    fn collect_fee_accumulates() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);

        client.collect_fee(&payer, &300, &fee_ref(&env, 1));
        let state = client.get_state();
        assert_eq!(state.total_collected, 300);
        assert_eq!(state.payment_count, 1);
        assert_eq!(
            TokenClient::new(&env, &token_id).balance(&client.address),
            300
        );
        assert_eq!(TokenClient::new(&env, &token_id).balance(&payer), 700);

        client.collect_fee(&payer, &700, &fee_ref(&env, 2));
        let state = client.get_state();
        assert_eq!(state.total_collected, 1_000);
        assert_eq!(state.payment_count, 2);
        assert_eq!(
            TokenClient::new(&env, &token_id).balance(&client.address),
            1_000
        );
        assert_eq!(TokenClient::new(&env, &token_id).balance(&payer), 0);
    }

    #[test]
    fn collect_fee_requires_payer_auth() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);

        env.mock_auths(&[]);
        assert!(client
            .try_collect_fee(&payer, &100, &fee_ref(&env, 1))
            .is_err());

        let state = client.get_state();
        assert_eq!(state.total_collected, 0);
        assert_eq!(state.payment_count, 0);
        assert_eq!(
            TokenClient::new(&env, &token_id).balance(&client.address),
            0
        );
    }

    #[test]
    fn collect_fee_uninitialized_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _authority, payer, _token_id, token) = setup(&env);
        token.mint(&payer, &1_000);

        assert_eq!(
            client.try_collect_fee(&payer, &100, &fee_ref(&env, 1)),
            Err(Ok(Error::NotInitialized))
        );
    }

    #[test]
    fn collect_fee_invalid_amount_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);

        assert_eq!(
            client.try_collect_fee(&payer, &0, &fee_ref(&env, 1)),
            Err(Ok(Error::InvalidAmount))
        );
        assert_eq!(
            client.try_collect_fee(&payer, &-1, &fee_ref(&env, 1)),
            Err(Ok(Error::InvalidAmount))
        );
        assert_eq!(client.get_state().payment_count, 0);
    }

    #[test]
    fn withdraw_updates_state_and_balance() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);
        client.collect_fee(&payer, &1_000, &fee_ref(&env, 1));

        let recipient = Address::generate(&env);
        client.withdraw(&recipient, &400);

        let state = client.get_state();
        assert_eq!(state.total_collected, 1_000);
        assert_eq!(state.total_withdrawn, 400);
        assert_eq!(TokenClient::new(&env, &token_id).balance(&recipient), 400);
        assert_eq!(
            TokenClient::new(&env, &token_id).balance(&client.address),
            600
        );

        client.withdraw(&recipient, &600);
        assert_eq!(client.get_state().total_withdrawn, 1_000);
        assert_eq!(TokenClient::new(&env, &token_id).balance(&recipient), 1_000);
    }

    #[test]
    fn withdraw_requires_authority_auth() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);
        client.collect_fee(&payer, &1_000, &fee_ref(&env, 1));

        let attacker = Address::generate(&env);
        let result = client
            .mock_auths(&[MockAuth {
                address: &attacker,
                invoke: &MockAuthInvoke {
                    contract: &client.address,
                    fn_name: "withdraw",
                    args: (&attacker, 1_000i128).into_val(&env),
                    sub_invokes: &[],
                },
            }])
            .try_withdraw(&attacker, &1_000);

        assert!(result.is_err());
        assert_eq!(client.get_state().total_withdrawn, 0);
        assert_eq!(TokenClient::new(&env, &token_id).balance(&attacker), 0);
    }

    #[test]
    fn withdraw_rejects_over_withdraw() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);
        client.collect_fee(&payer, &500, &fee_ref(&env, 1));

        assert_eq!(
            client.try_withdraw(&payer, &501),
            Err(Ok(Error::InsufficientVault))
        );
        assert_eq!(client.get_state().total_withdrawn, 0);

        client.withdraw(&payer, &200);
        assert_eq!(client.get_state().total_withdrawn, 200);
        assert_eq!(
            client.try_withdraw(&payer, &301),
            Err(Ok(Error::InsufficientVault))
        );
    }

    #[test]
    fn withdraw_invalid_amount_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, payer, token_id, token) = setup(&env);
        token.mint(&payer, &1_000);
        client.initialize(&authority, &token_id, &250, &50);
        client.collect_fee(&payer, &1_000, &fee_ref(&env, 1));

        assert_eq!(
            client.try_withdraw(&payer, &0),
            Err(Ok(Error::InvalidAmount))
        );
        assert_eq!(
            client.try_withdraw(&payer, &-1),
            Err(Ok(Error::InvalidAmount))
        );
    }

    #[test]
    fn update_pricing_validates_and_applies() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, authority, _payer, token_id, _token) = setup(&env);
        client.initialize(&authority, &token_id, &100, &10);

        client.update_pricing(&500, &25);
        let state = client.get_state();
        assert_eq!(state.fee_bps, 500);
        assert_eq!(state.fixed_fee_cents, 25);

        assert_eq!(
            client.try_update_pricing(&10_001, &25),
            Err(Ok(Error::InvalidFeeBps))
        );
        let state = client.get_state();
        assert_eq!(state.fee_bps, 500);
        assert_eq!(state.fixed_fee_cents, 25);
    }
}
