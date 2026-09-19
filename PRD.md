# Product Requirements Document — stellar-card

**Agent-first virtual card CLI funded by Stellar testnet, issued through Stripe test mode.**

| Field | Value |
|---|---|
| Product Name | stellar-card |
| Version | 0.1.0 (testnet MVP) |
| Status | Testnet implementation; Soroban fee vault not yet deployed |
| Network | Stellar testnet (default); `mainnet` accepted by config but direct mode is testnet-only |
| Primary Interface | `stellar-card` CLI (Rust) with an npm wrapper |
| License | MIT |

## 1. Executive Summary

`stellar-card` is an agent-first virtual card CLI. Developers and AI agents deposit
XLM or USDC on Stellar testnet, and the CLI issues a virtual card through Stripe
Issuing test mode. The CLI is designed for machine callers: structured JSON output
by default, idempotent creates, deterministic error codes, and no interactive
prompts.

The Rust binary lives in this repository (`stellar-card`, crate `stellar-card`,
library `stellar_card`, version 0.1.0, MIT). An npm wrapper (`index.js`) exposes it
as the `stellar-card` command.

**Current stage:** a working testnet CLI and local state. The optional on-chain fee
collection path targets a Soroban fee-vault contract in `contracts/fee-vault`; the
contract exists and is unit-tested, but it has not been deployed. No deposit, fee,
or card evidence has been published yet — see `docs/uat.md` for the template that
will hold it. Evidence is recorded only after it is produced.

## 2. Problem

- Developers who want a payment instrument for tests, subscriptions, or automation
  must use fintech products built for manual, dashboard-driven use. Adding a card
  to a script or CI job is awkward or impossible.
- AI agents are increasingly expected to complete purchases. Most card products
  assume a human is watching and return prose or HTML instead of parseable data.
- On-chain balances are hard to convert into spendable cards without leaving the
  developer workflow.

**Gap:** there is no small, scriptable path from "fund an address" to "hold a
virtual card" that an agent can drive end to end.

## 3. Target Users

### 3.1 AI-agent operators
Builders who run autonomous purchasing, browsing, or SaaS-management agents. They
need a CLI that returns JSON, signals state through exit codes, supports safe
retries, and never blocks on a prompt.

### 3.2 Developer automation
Backend or platform engineers who create cards from scripts, tests, or internal
tooling. They value a single binary, local state, and clear failure messages.

### 3.3 Stellar testnet users
Developers with testnet XLM or testnet USDC who want to try an end-to-end
deposit-to-card flow without touching real funds.

## 4. Product Scope

### 4.1 In scope

| Area | Description |
|---|---|
| CLI direct mode | Local CLI with no server; config, credentials, and state stored on disk |
| Stellar deposits | Per-deposit Stellar account (XLM native, USDC with trustline); Horizon observation |
| Stripe Issuing test mode | Cardholder and virtual card creation via Stripe test keys |
| Optional Soroban fee vault | On-chain fee collection in XLM through the `contracts/fee-vault` contract, disabled by default |
| Agent ergonomics | JSON envelope, exit codes, idempotency keys, `--quiet`, non-interactive defaults |

### 4.2 Out of scope for this document

The repository also contains a web UI. The web UI is a separate surface; this PRD
covers the CLI and the shared CLI core only.

## 5. Architecture

```
CLI (Rust)                        local disk
  auth / card / balance  ────►  config.toml, credentials.toml, state.json
        │
        ├── Stripe API (Issuing, test mode) ......... card/cardholder operations
        ├── Horizon (testnet) ....................... account state, payments,
        │                                             balances, Friendbot funding,
        │                                             transaction submission
        ├── Coinbase spot API ....................... XLM/USD price for fee quotes
        └── Soroban RPC (testnet) ................... collect_fee invocation
                                                      (only when enabled)
```

- **Horizon** is the source of truth for deposit accounts, balances, and submitted
  classic transactions (ChangeTrust for USDC, funding checks).
- **Soroban RPC** is used only when on-chain fee collection is enabled; the CLI
  simulates, signs, submits, and polls the `collect_fee` invocation.
- **Local state** stores deposits, cards, fee payments, an audit log, and
  idempotency records. Files are written with `0600` permissions on Unix.
- **Stripe** is the payment rail for card issuance. The CLI never stores raw PAN or
  CVC in its output stores beyond what Stripe returns; see §10.

Direct mode currently returns an error for `--network mainnet`; testnet is the only
supported execution network.

## 6. Command Surface

### 6.1 Commands

| Command | Behavior |
|---|---|
| `auth login [--api-key sk_test_...]` | Validate and persist a Stripe API key |
| `auth status` | Show auth state, network, endpoints, and fee configuration |
| `deposit address [--asset xlm\|usdc]` | Create a deposit account; prints address and deposit id |
| `deposit status <id> [--wait] [--timeout N]` | Observe the address; exit code 10 while pending |
| `deposit fund <id>` | Fund a testnet deposit account through Friendbot |
| `deposit trustline <id>` | Submit a USDC ChangeTrust for a funded USDC deposit account |
| `deposit list` | List locally tracked deposits |
| `card buy --amount <USD>` | Debit local balance, collect the fee if enabled, create a Stripe virtual card ($5.00–$500.00) |
| `card show <id>` | Retrieve card details from Stripe; reveal if available |
| `card list` | List locally tracked cards |
| `card freeze <id> --confirm [--dry-run]` | Freeze a card; requires explicit confirmation |
| `balance` | Show available USD balance, deposit/card counts, fees paid |
| `config set <key> <value>` | Persist a configuration value |
| `debug confirm-deposit <id> --amount-usd <n>` | Hidden local debugging aid |

### 6.2 Global flags

`--format json|table|plain` (default `json`), `--network testnet|mainnet`,
`--api-key <key>`, `--quiet`, `--idempotency-key <key>`.

### 6.3 Agent contract

- Success: one JSON object on stdout shaped
  `{"ok": true, "data": {...}, "meta": {"request_id": "req_...", "timestamp": "..."}}`.
- Failure: JSON on stderr shaped
  `{"ok": false, "error": {"code", "message", "details", "suggestion"}, "meta": {...}}`.
- Exit codes: 1 general, 2 usage, 3 authentication, 4 insufficient balance,
  5 not found, 6 rate limited, 7 external service, 8 network, 10 deposit pending.
- Retries of `deposit address` and `card buy` with the same `--idempotency-key`
  return the previously created resource instead of creating a duplicate.

## 7. Data Model

All state is local; see `src/models.rs` and `src/store.rs`.

- **Config** (`config.toml`): `network`, `format`, `quiet`, `horizon_url`,
  `rpc_url`, `network_passphrase`, `stripe_base_url`, `coinbase_base_url`,
  `stellar_private_key`, `fee_contract_id`, `onchain_fee_collection_enabled`,
  `fee_fixed_cents`, `fee_variable_bps`, `xlm_price_usd`, `usdc_issuer`,
  `cardholder_name`, `cardholder_email`.
- **Credentials** (`credentials.toml`): Stripe API key plus creation timestamp.
- **State** (`state.json`):
  - `stripe_cardholder_id`
  - `deposits[]`: id, asset, network, address, token issuer, secret key,
    timestamps, status (`pending`/`confirmed`/`expired`), native and USD amounts,
    fee paid in stroops, last transaction hash
  - `cards[]`: id, Stripe card/holder ids, amount, currency, status, last4, brand,
    expiry, fee breakdown in USD, fee stroops and transaction hash, optional
    PAN/CVC, timestamps
  - `fee_payments[]`: id, source deposit, contract id, transaction hash, fee USD,
    fee stroops, card amount, XLM price used, timestamp
  - `audit_log[]` and `idempotency_keys[]`

Computed balances are local: confirmed deposit credits minus card debits (card
amount plus fee). Balances are only as accurate as local state and Horizon
observation; there is no ledger reconciliation service.

## 8. Fee Model

- Quote: **fixed $0.10 + 0.20% (20 bps)** of the card amount, rounded up at the
  cent level.
- Settlement asset: **XLM (stroops)**.
- Conversion: live Coinbase XLM/USD spot price when reachable, falling back to the
  configured `xlm_price_usd`. Conversion rounds up and never produces a zero fee.
- On-chain collection is opt-in (`onchain_fee_collection_enabled`, default
  `false`). When enabled, the CLI requires `fee_contract_id` and invokes
  `collect_fee(payer, amount, fee_reference)` on the fee vault through Soroban RPC,
  paying from the configured wallet or a confirmed XLM deposit.
- Fee vault contract: `contracts/fee-vault`, functions `initialize`,
  `collect_fee`, `update_pricing`, `withdraw`, `get_state`. It stores pricing
  parameters and accounting but does not compute fees.
- **Not yet deployed.** Use `<TESTNET_FEE_VAULT_CONTRACT_ID>` as the placeholder
  until a testnet deployment is recorded in `docs/uat.md`.

## 9. Testnet and Verification Expectations

- Default test suite runs offline: no Friendbot, no public Horizon/Soroban RPC, no
  Stripe network calls. Provider calls are covered with HTTP mocks.
- Live testnet checks are explicit, owner-authorized runs: Friendbot funding,
  deposit observation, USDC trustline, fee-vault deployment and `collect_fee`,
  Stripe Issuing test-mode card creation.
- Every live run is recorded in `docs/uat.md` with transaction hashes, contract id,
  and Stellar Expert links. No hash, id, or contract address may be written before
  it is observed.
- Verification commands: `npm test` (workspace tests), `cargo test --workspace`,
  `npm run typecheck`, `npm run check` where applicable.

## 10. Security and Compliance Boundaries

- **Testnet only.** Direct mode refuses `mainnet`. There are no real funds in the
  deposit, fee, or card flows. `mainnet` config exists for future work.
- **Not a regulated business.** stellar-card is not a bank, custodian, exchange,
  fiat on/off-ramp, or KYC provider, and this project does not claim otherwise.
  Stripe test mode performs no real issuance.
- **No secrets in output.** The Stellar secret key and Stripe API key are never
  echoed in normal output; `auth status` masks the key. Secret state files are
  written `0600`.
- **PAN/CVC are sensitive.** Card reveal output must be treated as credential
  material: do not log it, cache it, or forward it to third parties. Stripe
  Issuing test mode may not return PAN/CVC at all; the CLI then returns summary
  fields with a `reveal_unavailable_reason`.
- **Authorization before spend.** `card freeze` requires `--confirm`; fee
  collection requires an explicit opt-in and a configured payer; the fee vault
  checks authorization on every mutating call.
- **No yield or investment claims.** Fees are product pricing, not interest, and
  balances carry no yield.

## 11. Non-Goals

- Mainnet deployment or real-money card issuance.
- Fiat on/off ramps, custody, exchange, or KYC services.
- Multi-chain support.
- A hosted API, multi-tenant accounts, teams, or webhooks.
- Programmable spend policies, budgets, or card-level limits beyond Stripe defaults.
- On-chain balances for card spend: card spend is a Stripe-side concept; local
  balance only gates CLI purchases.

## 12. Open Questions and TBD

| # | Item | Status |
|---|---|---|
| 1 | Fee-vault testnet deployment (wasm, deploy tx, contract id) | TBD — pending owner-authorized publish |
| 2 | Whether Stripe Issuing test mode returns PAN/CVC for virtual cards | TBD — observe and record in UAT |
| 3 | Mainnet path (Horizon/Soroban endpoints, key handling, compliance) | TBD — not scheduled |
| 4 | USDC issuer and trustline behavior to confirm live | TBD — defaults in code, not yet verified live |
| 5 | npm publication of the `stellar-card` wrapper | TBD — package metadata prepared |

## 13. Wallet/dApp Integration Attribution

Wallet/dApp integration references the official stellar/stellar-dev-skill
(Apache-2.0) dapp module; distributed via the stellar-build skills installer
(kaankacar/stellar-build).

## 14. Related Documents

- `README.md` — install, quick start, command reference
- `docs/uat.md` — testnet UAT evidence template and results
- `.agents/skills/stellar-card-agent/SKILL.md` — agent operating guide
- `contracts/fee-vault/README.md` — fee vault interface and invariants
