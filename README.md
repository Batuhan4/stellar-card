# 💳 StellarCard

**Agent-first virtual card CLI funded by Stellar testnet, issued through Stripe Issuing test mode.**

Deposit XLM or USDC on Stellar and get a virtual Visa from a single command.
Structured JSON output, idempotent commands, machine-readable errors, and a
Soroban fee vault that collects the service fee on-chain — built so an autonomous
agent or a developer script can run the whole flow without touching a UI.

> ⚠️ Testnet only. Stripe test mode only. No real funds, no mainnet issuance.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## ⭐ Why StellarCard wins

- ✅ **It already works, end to end, on testnet.** Not a mock, not a roadmap:
  a real Friendbot-funded deposit was observed on Horizon, the fee was collected
  by the deployed Soroban contract, and Stripe Issuing issued a test-mode card.
  Every step is verifiable — see the transaction hashes below.
- 🤖 **Agent-first from the first commit.** JSON in, JSON out, idempotency keys,
  stable exit codes, actionable error suggestions, zero interactive prompts, and
  a shipped agent skill (`.agents/skills/stellar-card-agent`). An agent can go
  from `deposit address` to a card number without a human in the loop.
- ⛓️ **A real Stellar integration, not a bridge glue.** Horizon for accounts,
  balances, and funding transactions; classic transactions for USDC trustlines;
  a Soroban fee vault with `require_auth`, checked arithmetic, events, and an
  authority-only withdrawal path. The chain layer is the product.
- 🔍 **Verifiable in two minutes by anyone.** Open the UAT file, click the
  Stellar Expert links, and check the vault's `get_state`: one payment, exact
  stroop total. The repo also ships `scripts/demo-testnet.sh` so judges can
  reproduce the whole run themselves.
- 🧮 **Transparent economics.** Fixed $0.10 + 0.20% per card, quoted in XLM at
  the live Coinbase price, paid on-chain. The fee is visible in the JSON, the
  transaction, and the vault state — no hidden spread.
- 🧩 **Composable by design.** CLI, Soroban contract, and web dashboard are
  independent: point the CLI at any deployed vault, invoke the contract from any
  client, or drive the dashboard with Freighter.
- 🛡️ **Honest boundaries.** Not a bank, custodian, exchange, anchor, or KYC
  provider. Testnet only, no yield claims, secrets never leave the local state
  directory with `0600` permissions.

## 🌐 Live on Stellar testnet

| Artifact | Value |
|---|---|
| ⛓️ Fee vault contract | [`CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ`](https://stellar.expert/explorer/testnet/contract/CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ) |
| 🚀 Deploy tx | [`99933b3b…`](https://stellar.expert/explorer/testnet/tx/99933b3bd2dbac5dc3a165bc23401d8dd9a005d66ad384d54094ff1e773de670) |
| 🔧 Initialize tx | [`9f80c750…`](https://stellar.expert/explorer/testnet/tx/9f80c7505dbf9c579c63dc74d61f85d3f14923ffff13d6fa09819fc813865137) |
| 💸 Live fee collection #1 | [`ffedc71a…`](https://stellar.expert/explorer/testnet/tx/ffedc71a2219c3f79b576fea831982fb7a0feab4dffdcad0fd544c1764bd4a23) |
| 💸 Live fee collection #2 | [`8f841537…`](https://stellar.expert/explorer/testnet/tx/8f841537d4215ccb084b8f49a32df31d33e9f55dd34c9df14ae82a5f85bda66c) |

The second live run issued a Visa card (`last4 0161`), revealed its test-mode
PAN/CVC, then froze it — all through the CLI. Full evidence, hashes, and the
vault state snapshot live in [`docs/uat.md`](docs/uat.md).

## 🏗️ How it works

```
deposit address (Stellar keypair)
        │  XLM / USDC on testnet
        ▼
Horizon observation ──► local USD balance (live Coinbase XLM-USD price)
        │
        ▼
card buy ──► Soroban fee vault `collect_fee` (fee in XLM, payer = tx source)
        └──► Stripe Issuing test mode virtual card
```

- **Deposits** — per-deposit Stellar accounts; balances and funding transactions
  are read from Horizon. USDC uses the Circle testnet issuer and requires a
  trustline (`deposit trustline`).
- **Fees** — fixed $0.10 + 0.20% per card, converted to XLM at the live spot
  price and paid on-chain through the Soroban fee vault.
- **Cards** — Stripe Issuing test mode (cardholder + virtual card). Stripe test
  mode does expose PAN/CVC for virtual cards through the reveal API; the CLI
  returns them once and stores nothing beyond the summary.
- **State** — local JSON/TOML under `~/.config/stellar-card`, `0600` permissions,
  no database, no server.

## 🚀 Quickstart

Requirements: Rust toolchain (pinned by `rust-toolchain.toml`), a Stripe
**test** secret key with Issuing access, and optionally the `stellar` CLI for
deploying your own fee vault.

```bash
# build the CLI
cargo build --release

# 1. authenticate Stripe (test mode)
./target/release/stellar-card auth login --api-key sk_test_...

# 2. use the deployed vault (or deploy your own with scripts/deploy-fee-vault.sh)
./target/release/stellar-card config set fee_contract_id CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ
./target/release/stellar-card config set onchain_fee_collection_enabled true

# 3. create a deposit address and fund it with testnet XLM
./target/release/stellar-card deposit address --asset xlm
./target/release/stellar-card deposit fund <deposit-id>
./target/release/stellar-card deposit status <deposit-id> --wait --timeout 180

# 4. buy a virtual card
./target/release/stellar-card card buy --amount 5.00
```

Or run the whole flow in one shot:

```bash
./scripts/demo-testnet.sh
```

## 🧭 Commands

| Command | Description |
|---|---|
| `auth login --api-key <sk_test_...>` | Store and validate a Stripe test key |
| `auth status` | Show auth, network, wallet, and vault configuration |
| `deposit address --asset xlm\|usdc` | Create a Stellar deposit address |
| `deposit fund <id>` | Fund the deposit address from Friendbot (testnet) |
| `deposit trustline <id>` | Create the USDC trustline for a deposit account |
| `deposit status <id> [--wait --timeout N]` | Observe the deposit on Horizon |
| `deposit list` | List deposits |
| `card buy --amount <USD>` | Collect the fee on-chain and create a Stripe virtual card |
| `card show <id>` | Reveal available card details |
| `card list` / `card freeze <id> --confirm` | List or freeze cards |
| `balance` | Available balance in USD |
| `config set <key> <value>` | Update configuration |

Global flags: `--format json|table|plain`, `--network testnet|mainnet`,
`--api-key`, `--quiet`, `--idempotency-key`.

Config keys: `network`, `format`, `horizon_url`, `rpc_url`, `network_passphrase`,
`stripe_base_url`, `coinbase_base_url`, `stellar_private_key`, `fee_contract_id`,
`onchain_fee_collection_enabled`, `fee_fixed_cents`, `fee_variable_bps`,
`xlm_price_usd`, `usdc_issuer`, `cardholder_name`, `cardholder_email`.

## 📜 Soroban fee vault

`contracts/fee-vault` implements:

- `initialize(authority, native_token, fee_bps, fixed_fee_cents)`
- `collect_fee(payer, amount, fee_reference)` — transfers XLM into the vault,
  bumps totals, emits `fee_collected`
- `update_pricing(fee_bps, fixed_fee_cents)` / `withdraw(to, amount)` — authority-only
- `get_state()` — authority, pricing, totals, payment count

All arithmetic is checked; authorization is enforced with `require_auth`; the
native XLM SAC address is stored at initialization and is immutable.

```bash
cargo test -p stellar-card-fee-vault     # 13 contract tests
stellar contract build --package stellar-card-fee-vault
STELLAR_ACCOUNT=<identity> ./scripts/deploy-fee-vault.sh
```

## 🖥️ Web demo

A Next.js dashboard with Freighter wallet connection lives in [`web/`](web/).
It connects to Stellar testnet, shows the connected account balance, offers a
Friendbot link for unfunded accounts, and invokes `collect_fee` through the
Soroban contract client.

```bash
cd web
npm install
echo "NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID=CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ" > .env.local
npm run dev
```

## 🧪 Testing

```bash
cargo test --workspace   # 54 offline tests: unit, CLI integration, contract
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
npm --prefix web run build && npm --prefix web run lint
```

- Default tests never call Friendbot, Horizon, Soroban RPC, Coinbase, or Stripe —
  every external endpoint is pointed at a local wiremock server.
- The live path is exercised by `scripts/deploy-fee-vault.sh` and
  `scripts/demo-testnet.sh`, whose real outputs are recorded in
  [`docs/uat.md`](docs/uat.md).

## 🔐 Security and compliance

- 🔒 Testnet only; mainnet direct mode is blocked by the CLI (`ensure_testnet`).
- 🔑 Stripe test keys and Stellar secret keys are never committed; local config
  files are written with `0600` permissions, and the CLI masks keys in output.
- 🏦 StellarCard is not a bank, custodian, exchange, anchor, or KYC provider.
  It makes no yield claims and handles no real funds.
- 🧮 Fees, balances, and pricing use integer arithmetic where it matters
  (stroops, cents); overflow checks are enabled in release builds.

## 🏆 Hackathon disclosure

Built for the Stellar Pro Hackathon 2026, Genesis Track.

- The Stellar integration is real and live on testnet: Horizon for deposits and
  balances, Friendbot for funding, classic transactions for trustlines, and a
  deployed Soroban contract for fee collection (links above). Nothing in the
  testnet flows is simulated.
- AI skills used during development, disclosed per event expectations:
  - [`stellar/stellar-dev-skill`](https://github.com/stellar/stellar-dev-skill)
    — dapp module (Apache-2.0), reference for wallet connection,
    `@stellar/stellar-sdk` usage, and transaction submission in `web/`.
  - [`kaankacar/stellar-build`](https://github.com/kaankacar/stellar-build) —
    the community skills installer that distributes the module above; no code
    was vendored from it.
  - The repo's own `.agents/skills/stellar-card-agent` skill is original (MIT).
***REMOVED***
***REMOVED***
***REMOVED***
***REMOVED***

## 📁 Project layout

```
stellar-card/
├── src/                    # Rust CLI
│   ├── app.rs              # command handlers
│   ├── horizon.rs          # Horizon client and deposit observation
│   ├── soroban.rs          # Soroban simulate/assemble/sign/send
│   ├── tx.rs               # Stellar keypairs, classic transactions, signing
│   ├── fee_contract.rs     # fee math and contract argument encoding
│   ├── providers.rs        # Stripe client
│   └── models.rs, store.rs, output.rs, error.rs, cli.rs
├── contracts/fee-vault/    # Soroban fee vault (soroban-sdk 28)
├── tests/                  # offline CLI integration tests
├── scripts/                # deploy and demo scripts
├── web/                    # Next.js + Freighter dashboard
├── docs/                   # UAT evidence, PRD
└── PRD.md                  # product requirements
```

## 📏 Limits

- 💵 Card value: **$5.00 – $500.00**.
- 🧪 Direct mode is testnet-only; mainnet requires validation and a live
  Stripe key.
- 🎭 Stripe Issuing test mode cards are not real cards and cannot be spent.
- ♻️ Stellar testnet resets periodically; redeploy the vault if the network
  resets.

## 📄 License

[MIT](LICENSE) — Batuhan Bayazit
