# 💳 StellarCard

🌐 **[Live Demo: card.batuhan4.com](https://card.batuhan4.com)** (fallback: [stellar-card-54s.pages.dev](https://stellar-card-54s.pages.dev))

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
- 🏦 **On-ramp ready (SEP-10/SEP-24).** A real anchor client is implemented and
  was verified live against SDF's testnet reference anchor — SEP-1 discovery,
  SEP-10 signed-challenge auth, and an interactive SEP-24 deposit that lands in
  the same address the card is funded from. TRY is a counterparty gap, not a code
  gap: no publicly documented TRY anchor exists yet, and the moment one does,
  `config set anchor_home_domain` points the CLI at it with no code changes.
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
| 🌐 Live web demo | [`card.batuhan4.com`](https://card.batuhan4.com) (fallback [`stellar-card-54s.pages.dev`](https://stellar-card-54s.pages.dev)) |
| 🪪 Edge-issued test cards | `ic_1UHdbSE…` last4 `0203` via fee [`8b822a21…`](https://stellar.expert/explorer/testnet/tx/8b822a2172a2b6004cd64274a0e8bb7d81f89b0ea4ffc429367395d507190238) · `ic_1UHdlVE…` last4 `0229` via fee [`7916e861…`](https://stellar.expert/explorer/testnet/tx/7916e861609a6d6e29eae5d625b720553aa89336ca0b20aec07b75b98aac622f) |

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
- **Web demo** — the same flow from the browser: Freighter pays the fee, then a
  Cloudflare Pages Function verifies it on Horizon and issues the Stripe
  test-mode card; see [Web demo](#️-web-demo).
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
| `onramp info` | List the anchor's SEP-24 currencies and endpoints |
| `onramp start [--asset usdc\|xlm] [--amount N] [--deposit <id>]` | Start an anchor deposit (SEP-10 + SEP-24) |
| `onramp status <ramp_id>` | Poll the anchor deposit and credit the linked deposit when completed |
| `balance` | Available balance in USD |
| `config set <key> <value>` | Update configuration |

Global flags: `--format json|table|plain`, `--network testnet|mainnet`,
`--api-key`, `--quiet`, `--idempotency-key`.

Config keys: `network`, `format`, `horizon_url`, `rpc_url`, `network_passphrase`,
`stripe_base_url`, `coinbase_base_url`, `stellar_private_key`, `fee_contract_id`,
`onchain_fee_collection_enabled`, `fee_fixed_cents`, `fee_variable_bps`,
`xlm_price_usd`, `usdc_issuer`, `anchor_home_domain`, `anchor_sep24_url`,
`anchor_web_auth_endpoint`, `cardholder_name`, `cardholder_email`.

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

## 🏦 On-ramp (SEP-24)

`stellar-card` ships an anchor-agnostic fiat on-ramp client:

- **SEP-1** — reads `/.well-known/stellar.toml` for endpoints and supported currencies.
- **SEP-10** — signs the anchor's challenge transaction with the deposit key and
  exchanges it for a JWT (`sign_envelope`, real ed25519 signatures).
- **SEP-24** — opens an interactive deposit at the anchor and polls its status.

```bash
stellar-card onramp info                       # anchor currencies + endpoints
stellar-card onramp start --asset usdc --amount 10
# → interactive_url: complete KYC/funding in the browser
stellar-card onramp status ramp_...            # pending → completed
stellar-card deposit status <dep-id>           # credited once funds arrive
stellar-card card buy --amount 5.00            # spend it
```

Live testnet verification (2026-09-20) against SDF's reference anchor
`testanchor.stellar.org`: `onramp info` returned SRT, USDC, and native with the
SEP-24 and auth endpoints; `onramp start --asset usdc --amount 10` produced a
real anchor transaction (`3d5171b7-e5b2-4d83-9e77-108c7c5a35d1`) and an
interactive URL; `onramp status` reported `incomplete` with the linked deposit
pending. The anchor's per-asset max is 10, and completion is a browser KYC step
by design — full evidence in [`docs/uat.md`](docs/uat.md).

### TRY on-ramp readiness

There is **no publicly documented TRY-capable Stellar anchor** — not on mainnet
and not on testnet — as of the verified research date (2026-09-19):

- The SDF anchor directory lists no TRY/TRYB asset with SEP-6/SEP-24 enabled.
- KB Trading's TRYB is status `test` and is not enabled in its SEP asset lists.
- BiLira issues TRYB on EVM/Solana and publishes no Stellar `stellar.toml`.
- SDF's testnet reference anchor supports SRT, USDC, and native only.

So the demo on-ramps USDC on testnet, and **any TRY anchor that appears works
with zero code changes**:

```bash
stellar-card config set anchor_home_domain <try-anchor-domain>
stellar-card config set anchor_sep24_url <https://.../sep24>          # if not in the TOML
stellar-card config set anchor_web_auth_endpoint <https://.../auth>   # if not in the TOML
stellar-card onramp info
```

The missing piece is a licensed counterparty, not the integration. This boundary
is deliberate: StellarCard never pretends to be an anchor or custodian.

## 🖥️ Web demo

Live at [`card.batuhan4.com`](https://card.batuhan4.com) (fallback:
[`stellar-card-54s.pages.dev`](https://stellar-card-54s.pages.dev)) — a Next.js
static export on Cloudflare Pages with edge functions.

**The web demo is not a mock.** Every step touches real infrastructure:

1. Freighter signs a real Soroban `collect_fee` transaction on Stellar testnet.
2. A Cloudflare Pages Function ([`web/functions/`](web/functions/)) verifies
   that transaction on Horizon: it succeeded, it is fresh (< 20 minutes), it
   contains a native transfer **to the fee vault** **from the payer**, and the
   amount covers the quoted $0.10 + 0.20% fee.
3. The same function issues a real Stripe **test-mode** virtual card and stores
   a card↔cardholder mapping in Cloudflare KV. A fee transaction can be used
   once — replays return `409`. PAN/CVC are returned only for the cardholder's
   email; other reads return `403`. `livemode: false` is exposed in every
   response.
4. Freeze calls Stripe directly from the edge function.

Stripe secrets never reach the browser: `STRIPE_TEST_KEY` is a Pages secret, and
the only public configuration is `NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID`.

Freighter must be on **Testnet** (Freighter → Settings → Network → Testnet). If
it is on another network, the site keeps the account connected, shows a banner
with the exact steps, and disables payment until it is switched.

```bash
cd web
npm install
echo "NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID=CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ" > .env.local

# local dev: static export + Pages Functions + local KV
npm run build && npx wrangler pages dev out

# deploy (Cloudflare account + `wrangler login`)
npx wrangler pages deploy --project-name stellar-card --branch main
```

Edge API surface: `POST /api/card` (issue), `GET /api/card/:id`,
`POST /api/card/:id/freeze`, `GET /api/cards?email=`.

### ⚡ Quick Buy (TRY demo preview)

[`/quick`](https://card.batuhan4.com/quick) is a three-tap flow for people with
zero Stellar knowledge: pick a TRY amount, get an IBAN + reference, tap
“I’ve sent the TRY”, and the card appears. It needs no wallet and works on
phones.

Honest boundaries: there is still no live TRY-capable Stellar anchor (see
[TRY on-ramp readiness](#try-on-ramp-readiness)), so the TRY transfer and
conversion stages are simulated in the UI. The issued card is a real Stripe
**test-mode** card: it is created by the edge function, flagged
`metadata[demo]=quick-buy-try` in Stripe and in KV, shown with a
“demo · test mode” badge, and rate-limited (5/day per email, 12/day per
network). It does not touch the fee vault. The standard `/api/card` flow —
on-chain fee verified on Horizon before issuance — is unchanged and remains the
production path.

The UI is responsive (bottom navigation and compact layouts on mobile) and
carries the project mascot — the 3D crab holding a Lumen Card — animated with CSS
for idle, anxious, cheer, and peek states. Mascot artwork supplied by the project
owner.

```bash
# opt-in browser smoke test (Playwright): all pages at 4 viewports,
# overflow/console/network checks, reveal interaction; writes screenshots
npx playwright install chromium
npm run test:e2e
```

## 🧪 Testing

```bash
cargo test --workspace   # 66 offline tests: unit, CLI integration, contract
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
npm --prefix web run build && npm --prefix web run lint
npm --prefix web run test:e2e   # opt-in browser smoke test against the live demo
```

- Default tests never call Friendbot, Horizon, Soroban RPC, Coinbase, or Stripe —
  every external endpoint is pointed at a local wiremock server.
- `web/e2e/smoke.mjs` is opt-in: it drives the deployed demo in a real browser
  (17 checks across 4 pages × 4 viewports plus the card-reveal flow) and is the
  only test that touches the network by default configuration.
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
  balances, Friendbot for funding, classic transactions for trustlines, a
  deployed Soroban contract for fee collection, a SEP-10/SEP-24 on-ramp verified
  against SDF's reference anchor, and a hosted web demo whose edge function
  issues real Stripe test-mode cards after verifying the on-chain fee (links
  above). Nothing in the testnet flows is simulated.
- AI skills used during development, disclosed per event expectations:
  - [`stellar/stellar-dev-skill`](https://github.com/stellar/stellar-dev-skill)
    — dapp module (Apache-2.0), reference for wallet connection,
    `@stellar/stellar-sdk` usage, and transaction submission in `web/`.
  - [`kaankacar/stellar-build`](https://github.com/kaankacar/stellar-build) —
    the community skills installer that distributes the module above; no code
    was vendored from it.
  - The repo's own `.agents/skills/stellar-card-agent` skill is original (MIT).

## 📁 Project layout

```
stellar-card/
├── src/                    # Rust CLI
│   ├── app.rs              # command handlers
│   ├── anchor.rs           # SEP-1/SEP-10/SEP-24 anchor client (on-ramp)
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
│   ├── functions/          # Cloudflare Pages Functions (edge card issuance API)
│   └── wrangler.toml       # Pages project, KV binding, edge vars
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
