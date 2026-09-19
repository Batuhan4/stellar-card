---
name: stellar-card-agent
description: Operate the stellar-card CLI on Stellar testnet. Use when an agent must install the CLI, authenticate with a Stripe test key, create and fund XLM/USDC deposit addresses, track deposits, enable the on-chain fee vault, buy, reveal, or freeze a virtual card, and report Stellar Expert links.
license: MIT
---

# stellar-card-agent

Operating guide for the `stellar-card` CLI: an agent-first virtual card CLI that
funds cards from Stellar testnet deposits and issues them through Stripe Issuing
test mode. Every command returns JSON by default. Follow the steps in order and
stop to report if a step fails.

## Safety rules (read first)

1. **Testnet only.** Direct mode supports Stellar testnet. Do not attempt a real
   card, real funds, or mainnet. If `--network mainnet` is requested, stop and
   report that it is unsupported.
2. **Never echo the Stellar secret key.** The `stellar_private_key` config value
   starts with `S`. Never print it, log it, include it in summaries, or pass it to
   any third party. After setup, refer to it as "configured". Treat command
   transcripts as sensitive too: prefer setting it via the CLI once, not pasting it
   into chat repeatedly.
3. **Stripe API keys are secrets.** Use a test-mode key (`sk_test_...`). Never echo
   it; `auth status` only reports a masked form.
4. **PAN/CVC are sensitive.** Card number and CVC are credential material. Reveal
   only when the user explicitly asks, show once, never cache or transmit them
   elsewhere. If Stripe test mode does not return them, report
   `reveal_unavailable_reason` and fall back to last4/brand/expiry.
5. **Confirm destructive actions.** `card freeze` requires `--confirm`. Do not add
   `--confirm` unless the user asked for the freeze.
6. **No fabricated evidence.** Never invent transaction hashes, contract ids,
   addresses, or card ids. Use `<TBD>` until a real value is observed. Only report
   Stellar Expert links built from hashes actually returned by the CLI.
7. **`debug confirm-deposit` is a local shortcut, not evidence.** Do not use it to
   claim a real deposit was observed.

## Prerequisites

- Rust toolchain (the CLI builds with `cargo`).
- Node.js 18+ only if installing through npm.
- A Stripe test-mode secret key with Issuing access.
- An internet connection for Horizon, Soroban RPC, Coinbase, and Stripe.
- Config/state location: `STELLAR_CARD_HOME` if set, otherwise the platform config
  directory (`~/.config/stellar-card` on Linux). API key override:
  `STELLAR_CARD_API_KEY`.

## Step 1 — Install

From a repository checkout:

```bash
cargo build --release
./target/release/stellar-card --help
```

Or through the npm wrapper, which runs the same build:

```bash
npm install        # runs postinstall: cargo build --release
npm start          # node index.js --help
```

If `stellar-card` is published to npm, `npm install -g stellar-card` installs the
`stellar-card` command. Otherwise use the release binary path directly.

Optional: install this skill for Codex (default `~/.codex/skills`) and optionally
Claude Code:

```bash
./scripts/***REMOVED***
./scripts/***REMOVED*** --claude
```

## Step 2 — Authenticate

```bash
stellar-card auth login --api-key sk_test_...
stellar-card auth status
```

`auth login` validates the key against Stripe and stores it. `auth status` shows
`authenticated`, the network, and a masked key. Confirm `network` is `testnet`.

## Step 3 — Configure the Stellar key

```bash
stellar-card config set stellar_private_key S...
stellar-card config set cardholder_name "Agent Demo User"
stellar-card config set cardholder_email "agent@example.com"
```

The key is validated as an `S...` ed25519 secret at set time. It is used to derive
deposit addresses and to pay on-chain fees when enabled. Never print it back.

Optional defaults (already set in code, change only if needed):

```bash
stellar-card config set network testnet
stellar-card config set xlm_price_usd 0.20
stellar-card config set usdc_issuer GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5
```

## Step 4 — Optional: enable the Soroban fee vault

The fee vault contract (`contracts/fee-vault`) may not be deployed yet. Only enable
it with a contract id that was actually deployed and recorded.

```bash
# Replace with the deployed C... address; do not invent one
stellar-card config set fee_contract_id <TESTNET_FEE_VAULT_CONTRACT_ID>
stellar-card config set onchain_fee_collection_enabled true
stellar-card config set fee_fixed_cents 10
stellar-card config set fee_variable_bps 20
```

If no deployment exists, leave `onchain_fee_collection_enabled false`, continue in
off-chain fee-accounting mode, and report the missing contract as a blocker.

When enabled, `card buy` quotes `fixed $0.10 + 0.20%` in XLM using the live
Coinbase spot price with `xlm_price_usd` as fallback, then invokes `collect_fee` on
the vault. It needs either a funded configured wallet or a confirmed XLM deposit
with enough balance for the fee plus the base reserve.

## Step 5 — Create a deposit address

```bash
stellar-card deposit address --asset xlm
stellar-card deposit address --asset usdc
```

Use `--idempotency-key <key>` to make retries safe; the same key returns the same
deposit. Capture `data.id` (for example `dep_...`) and `data.address` (`G...`).

## Step 6 — Fund the deposit on testnet

```bash
stellar-card deposit fund dep_...
```

This calls Friendbot and returns a `transaction_hash`. Record it. For USDC, the
deposit account must be funded first, then a trustline is required:

```bash
stellar-card deposit trustline dep_...
```

Send testnet XLM or USDC to the deposit address from any testnet wallet to create
a real deposit observation.

## Step 7 — Track the deposit

```bash
stellar-card deposit status dep_... --wait --timeout 300
```

Exit code `10` (`DEPOSIT_PENDING`) means not yet observed; JSON is still returned.
`status: "confirmed"` with a positive `amount_usd` means the deposit is credited.
`last_transaction_hash` is the Stellar transaction hash to report.

## Step 8 — Buy a card

```bash
stellar-card balance
stellar-card card buy --amount 5.00 --idempotency-key demo-buy-1
```

Allowed range is $5.00–$500.00. The command requires sufficient confirmed balance
for the amount plus the fee. It creates a Stripe cardholder if needed and returns
a card summary including `data.id` (`crd_...`), `stripe_card_id`, `last4`, `brand`,
expiry, status, and the fee breakdown. When on-chain fee collection ran, the
response also includes `fee_payment.transaction_hash` and `fee_stroops`.

## Step 9 — Reveal the card

```bash
stellar-card card show crd_...
```

The response nests the summary and adds `number`, `cvc`, and
`reveal_unavailable_reason`. If `number`/`cvc` are null, Stripe test mode did not
return PAN/CVC; report the reason and give last4, brand, and expiry instead. Do not
print sensitive fields into logs or store them.

## Step 10 — Freeze the card (only when asked)

```bash
stellar-card card freeze crd_... --confirm
```

Without `--confirm` the command refuses. Use `--dry-run` to preview without
calling Stripe (combine with `--confirm`). Freezing returns the updated card with
status `frozen` and `frozen_at`.

## Step 11 — Report results

Summarize: deposit id, card id, status, last4/brand/expiry, fee USD and stroops,
and the transactions below. Build links from real values only:

| Item | URL |
|---|---|
| Transaction | `https://stellar.expert/explorer/testnet/tx/<hash>` |
| Contract | `https://stellar.expert/explorer/testnet/contract/<C...>` |
| Account | `https://stellar.expert/explorer/testnet/account/<G...>` |

## Command reference

| Command | Purpose |
|---|---|
| `auth login [--api-key]` / `auth status` | Stripe test-mode authentication |
| `deposit address [--asset xlm\|usdc]` | Create a deposit account |
| `deposit status <id> [--wait --timeout N]` | Observe and credit a deposit |
| `deposit fund <id>` | Fund a testnet account via Friendbot |
| `deposit trustline <id>` | USDC ChangeTrust for a funded deposit account |
| `deposit list` | List local deposits |
| `card buy --amount <USD>` | Buy a virtual card |
| `card show <id>` | Reveal card details if available |
| `card list` / `card freeze <id> --confirm` | List / freeze cards |
| `balance` | Available USD balance and counters |
| `config set <key> <value>` | Persist configuration |

Global flags: `--format json|table|plain`, `--network testnet`, `--api-key`,
`--quiet`, `--idempotency-key`.

Exit codes: 0 success, 1 general, 2 usage, 3 auth, 4 insufficient balance,
5 not found, 6 rate limited, 7 external service, 8 network, 10 deposit pending.

## Failure handling

- `{}` with `ok: false` on stderr is a structured failure: read `error.code` and
  `error.suggestion`, fix, then retry.
- Retry only idempotent steps (`deposit address`, `card buy`) with the same
  idempotency key after a network error.
- Never "fix" a failed Stripe call by creating another card unless the user agrees.
- If Horizon, Soroban RPC, Coinbase, or Stripe is unreachable, report the failure
  and stop; do not fabricate results.

## Attribution

Wallet/dApp integration references the official stellar/stellar-dev-skill
(Apache-2.0) dapp module; distributed via the stellar-build skills installer
(kaankacar/stellar-build).
