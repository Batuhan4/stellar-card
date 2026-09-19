use crate::error::{AppError, ErrorCode};
use stellar_strkey::Contract;
use stellar_xdr::{AccountId, BytesM, Int128Parts, PublicKey, ScAddress, ScBytes, ScVal, Uint256};

pub const DEFAULT_FIXED_FEE_CENTS: u64 = 10;
pub const DEFAULT_VARIABLE_FEE_BPS: u16 = 20;
const STROOPS_PER_XLM: u128 = 10_000_000;
const BPS_DENOMINATOR: u64 = 10_000;

pub fn card_fee_cents(card_amount_cents: u64, fixed_fee_cents: u64, fee_bps: u16) -> u64 {
    let variable_fee = (card_amount_cents * fee_bps as u64).div_ceil(BPS_DENOMINATOR);
    fixed_fee_cents + variable_fee
}

pub fn usd_cents_to_stroops(fee_usd_cents: u64, xlm_price_usd: &str) -> Result<i64, AppError> {
    let price_cents = parse_money_to_cents(xlm_price_usd)?;
    if price_cents <= 0 {
        return Err(AppError::new(
            ErrorCode::Usage,
            "XLM spot price must be greater than zero",
        ));
    }
    let stroops = (fee_usd_cents as u128 * STROOPS_PER_XLM).div_ceil(price_cents as u128);
    i64::try_from(stroops)
        .map(|value| value.max(1))
        .map_err(|_| AppError::new(ErrorCode::Usage, "Fee amount overflows i64 stroops"))
}

pub fn parse_contract_id(value: &str) -> Result<[u8; 32], AppError> {
    Contract::from_string(value)
        .map(|contract| contract.0)
        .map_err(|_| {
            AppError::new(
                ErrorCode::Usage,
                format!("Invalid Stellar contract id (C...): {value}"),
            )
        })
}

pub fn scval_account(address: &str) -> Result<ScVal, AppError> {
    let public = stellar_strkey::ed25519::PublicKey::from_string(address).map_err(|_| {
        AppError::new(
            ErrorCode::Usage,
            format!("Invalid Stellar account id: {address}"),
        )
    })?;
    Ok(ScVal::Address(ScAddress::Account(AccountId(
        PublicKey::PublicKeyTypeEd25519(Uint256(public.0)),
    ))))
}

pub fn scval_contract_address(contract_id: [u8; 32]) -> ScVal {
    ScVal::Address(ScAddress::Contract(stellar_xdr::ContractId(
        stellar_xdr::Hash(contract_id),
    )))
}

pub fn scval_u32(value: u32) -> ScVal {
    ScVal::U32(value)
}

pub fn scval_i128(value: i128) -> ScVal {
    ScVal::I128(Int128Parts {
        hi: (value >> 64) as i64,
        lo: value as u64,
    })
}

pub fn scval_bytes16(bytes: [u8; 16]) -> ScVal {
    ScVal::Bytes(ScBytes(
        BytesM::try_from(bytes.to_vec()).expect("16 bytes fit within the XDR bytes limit"),
    ))
}

pub fn collect_fee_args(
    payer: &str,
    amount_stroops: i64,
    reference: [u8; 16],
) -> Result<Vec<ScVal>, AppError> {
    Ok(vec![
        scval_account(payer)?,
        scval_i128(amount_stroops as i128),
        scval_bytes16(reference),
    ])
}

fn parse_money_to_cents(value: &str) -> Result<i64, AppError> {
    let parsed = value.trim().parse::<f64>().map_err(|_| {
        AppError::new(
            ErrorCode::Usage,
            format!("Invalid money value for XLM price: {value}"),
        )
    })?;
    Ok((parsed * 100.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_fee_formula_is_stable() {
        assert_eq!(
            card_fee_cents(1_000, DEFAULT_FIXED_FEE_CENTS, DEFAULT_VARIABLE_FEE_BPS),
            12
        );
    }

    #[test]
    fn fee_stroops_round_up() {
        assert_eq!(usd_cents_to_stroops(12, "0.20").unwrap(), 6_000_000);
    }

    #[test]
    fn fee_stroops_reject_zero_price() {
        assert!(usd_cents_to_stroops(12, "0.00").is_err());
    }

    #[test]
    fn account_scval_parses_valid_address() {
        let scval = scval_account(TESTNET_USDC_ISSUER).unwrap();
        match scval {
            ScVal::Address(ScAddress::Account(account)) => match account {
                AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(bytes))) => {
                    assert_eq!(bytes.len(), 32);
                }
            },
            _ => panic!("expected account address scval"),
        }
    }

    #[test]
    fn i128_scval_splits_high_and_low() {
        match scval_i128(6_000_000) {
            ScVal::I128(parts) => {
                assert_eq!(parts.hi, 0);
                assert_eq!(parts.lo, 6_000_000);
            }
            _ => panic!("expected i128 scval"),
        }
    }

    #[test]
    fn contract_id_round_trips() {
        let encoded = format!("{}", Contract([7u8; 32]));
        let bytes = parse_contract_id(&encoded).unwrap();
        assert_eq!(bytes, [7u8; 32]);
        assert!(parse_contract_id("not-a-contract").is_err());
    }

    const TESTNET_USDC_ISSUER: &str = "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
}
