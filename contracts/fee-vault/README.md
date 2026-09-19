# stellar-card-fee-vault

Soroban fee vault for the Stellar Card CLI. The CLI quotes a card-purchase fee in
native XLM, pays it on-chain into this vault, and accounts for it here. The vault
stores pricing parameters (`fee_bps`, `fixed_fee_cents`) so the CLI can quote
consistently, but it does not compute fees itself. Fee math stays in the CLI.

The contract is deliberately minimal: one stored authority, no upgrade path, no
pause, no other admin functions.

## Public interface

Arguments are positional, in the order below. All functions except `get_state`
require the listed address to authorize the invocation.

| Function | Arguments | Returns | Authorization |
| --- | --- | --- | --- |
| `initialize` | `authority: Address`, `native_token: Address`, `fee_bps: u32`, `fixed_fee_cents: u32` | `Result<(), Error>` | `authority` |
| `collect_fee` | `payer: Address`, `amount: i128`, `fee_reference: BytesN<16>` | `Result<(), Error>` | `payer` |
| `withdraw` | `to: Address`, `amount: i128` | `Result<(), Error>` | stored `authority` |
| `update_pricing` | `fee_bps: u32`, `fixed_fee_cents: u32` | `Result<(), Error>` | stored `authority` |
| `get_state` | none | `Result<VaultState, Error>` | none |

`amount` is an `i128` in stroops (1 XLM = 10^7 stroops). `fee_reference` is a
16-byte client-side identifier (hex-encoded as 32 hex characters when invoked
through `stellar`).

### State

`VaultState` is stored in instance storage under the symbol `STATE`:

```rust
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
```

`native_token` is the Stellar Asset Contract address of native XLM for the
target network, passed at `initialize`. `created_at` is the ledger timestamp at
initialization. Instance storage keeps the accounting in the same entry as the
contract instance and is extended on every mutation (to 30 days when below 7
days); read-only calls do not extend it.

### Errors

`Error` is a `#[contracterror]` enum with explicit discriminants.

| Code | Variant | Raised when |
| --- | --- | --- |
| 1 | `AlreadyInitialized` | `initialize` called on an initialized vault |
| 2 | `NotInitialized` | `collect_fee`, `withdraw`, `update_pricing`, or `get_state` before `initialize` |
| 3 | `InvalidFeeBps` | `fee_bps > 10_000` in `initialize` or `update_pricing` |
| 4 | `InvalidAmount` | `collect_fee`/`withdraw` amount is zero or negative |
| 5 | `MathOverflow` | checked addition/subtraction overflow (not reachable with valid amounts) |
| 6 | `InsufficientVault` | `withdraw` amount exceeds `total_collected - total_withdrawn` |
| 7 | `Unauthorized` | reserved; see note below |

Authorization failures do not produce code 7. `Address::require_auth` aborts the
invocation at the host level, so RPC simulation/submission fails with an auth
error rather than a contract error. The variant is reserved to keep the
discriminants stable.

### Events

Events use `#[contractevent]`, so they are part of the contract spec and can be
decoded with generated bindings. The first topic is always the event name; the
remaining topics and the data map follow the field tables below (data is a map
keyed by field name).

| Event | Topics | Data |
| --- | --- | --- |
| `initialized` | `["initialized"]` | `{authority, native_token, fee_bps, fixed_fee_cents, created_at}` |
| `fee_collected` | `["fee_collected", payer]` | `{amount, fee_reference, payment_count}` |
| `withdrawn` | `["withdrawn", to]` | `{amount, total_withdrawn}` |
| `pricing_updated` | `["pricing_updated"]` | `{fee_bps, fixed_fee_cents}` |

`fee_collected.payment_count` is the running `payment_count` after the payment.
`withdrawn.total_withdrawn` is the running `total_withdrawn` after the transfer.

## Invariants

- A `withdraw` can never exceed `total_collected - total_withdrawn`.
- `collect_fee` increases `total_collected` and `payment_count`; `withdraw`
  increases `total_withdrawn`; neither ever decreases.
- All arithmetic on totals uses `checked_add`/`checked_sub` and maps `None` to
  `MathOverflow`.
- The token used is fixed at `initialize`; no call can change it.
- Accounting is updated before the token transfer, and Soroban prohibits
  re-entering the same contract, so accounting cannot be corrupted by a
  re-entrant call.

## Build

The canonical build goes through Stellar CLI, which removes spec-shaking
markers and runs `wasm-opt`:

```sh
stellar contract build        # from this directory
```

This produces:

```
target/wasm32v1-none/release/stellar_card_fee_vault.wasm
```

`soroban-sdk` 28 refuses direct `cargo build` for wasm targets unless the build
system advertises spec-shaking support, so the equivalent raw command is:

```sh
SOROBAN_SDK_BUILD_SYSTEM_SUPPORTS_SPEC_SHAKING_V2=1 \
  cargo build -p stellar-card-fee-vault --target wasm32v1-none --release
```

Prefer `stellar contract build`: a raw cargo build leaves the contract spec
unshaken (larger wasm, extra spec entries) and skips `wasm-opt`.

## Tests, format, lint

```sh
cargo test -p stellar-card-fee-vault
cargo clippy -p stellar-card-fee-vault --all-targets -- -D warnings
```

Tests use `soroban-sdk` testutils with a test Stellar Asset Contract
(`register_stellar_asset_contract_v2` + `StellarAssetClient::mint`), no
network access. Covered cases:

- `initialize` stores state and captures `created_at`
- double `initialize` fails with `AlreadyInitialized`
- `initialize` rejects `fee_bps > 10_000` and accepts `10_000`
- `get_state` before `initialize` fails with `NotInitialized`
- `collect_fee` moves tokens to the vault and accumulates totals over two calls
- `collect_fee` without payer auth fails and changes nothing
- `collect_fee` before `initialize` fails with `NotInitialized`
- `collect_fee` rejects zero and negative amounts
- `withdraw` moves tokens, updates `total_withdrawn`, and allows the remainder
- `withdraw` with a non-authority authorization fails
- `withdraw` rejects amounts above available balance and zero/negative amounts
- `update_pricing` applies valid pricing and rejects `fee_bps > 10_000`
