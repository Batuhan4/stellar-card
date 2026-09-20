# StellarCard UAT Evidence — Stellar Testnet

Date: 2026-09-20 (UTC). All identifiers below are real Stellar testnet artifacts
produced by this repository's CLI, the deployed Soroban contract, Horizon, and
Stripe Issuing test mode. No values are simulated.

## Fee vault deployment (Soroban)

| Item | Value |
|---|---|
| Contract ID | `CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ` |
| Wasm sha256 | `67b47374682d7943bd2ceb544ad2ace549c8a9197dedf9b708bd9a0777023d07` |
| Wasm upload tx | `136ed91b47e1e2e81d753893c3daa2ba3b0b887bbd5af2cf60d951372791388f` |
| Deploy tx | `99933b3bd2dbac5dc3a165bc23401d8dd9a005d66ad384d54094ff1e773de670` |
| Initialize tx | `9f80c7505dbf9c579c63dc74d61f85d3f14923ffff13d6fa09819fc813865137` |
| Authority | `GBG5IH4QGSIV7HTEOVGXOV7DEWZVRNXWA57FI4NT6PEZVNGAKRW23SJN` |
| Native XLM SAC | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` |
| Pricing stored | `fee_bps=20`, `fixed_fee_cents=10` |

Explorer: https://stellar.expert/explorer/testnet/contract/CACWNJ65VRCHKMGZSIGRH775ZU6S3C7MGAFVIKPBMVIJ62NIXVEXGVQQ

`get_state` after the UAT run:

```json
{
  "authority": "GBG5IH4QGSIV7HTEOVGXOV7DEWZVRNXWA57FI4NT6PEZVNGAKRW23SJN",
  "created_at": 1789881282,
  "fee_bps": 20,
  "fixed_fee_cents": 10,
  "native_token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "payment_count": 1,
  "total_collected": "5789474",
  "total_withdrawn": "0"
}
```

## Deposit observation (Horizon + Friendbot)

| Item | Value |
|---|---|
| Deposit id | `dep_338b76d59798` |
| Address | `GAJOYH7BUYIR2VPLGB3IKU3PDNK3Y5JCJOM4HZH7R5LYBN4JFGU7WAVL` |
| Funding tx (Friendbot) | `9a2d8ddcf55cfd2df1cbce98a91c6ba6503552c8ef35c601fb4e47b3863c6421` |
| Observed balance | `10000.0000000` XLM |
| Observed USD | `$1900.00` (Coinbase `XLM-USD/spot`) |
| Status | `confirmed` |

## Card purchase with on-chain fee (Stripe Issuing test mode)

| Item | Value |
|---|---|
| Card request | `$5.00` (fee `$0.11` = fixed `$0.10` + 0.20%) |
| Fee tx (Soroban `collect_fee`) | `ffedc71a2219c3f79b576fea831982fb7a0feab4dffdcad0fd544c1764bd4a23` |
| Fee tx ledger | `4771548`, `successful: true` |
| Fee tx source | deposit address (payer == transaction source) |
| Fee charged | `5789474` stroops (~0.579 XLM at the live spot price) |
| Stripe account | `acct_1TIQRMEAzMrENaFX` (test mode, key masked) |
| Stripe card id | `ic_1UHd64EAzMrENaFXAUUqGBqo` |
| Card | Visa, `last4=0153`, exp `05/2029`, status `active` |

PAN/CVC are not exposed by Stripe Issuing test mode for virtual cards; the
`card show` command returns the fields Stripe makes available and records the
reason in `reveal_unavailable_reason`.

## Repeatability run (same day, second deposit)

A second full run confirmed the flow is repeatable, not a one-off:

| Item | Value |
|---|---|
| Deposit id | `dep_c198d13dbb13` |
| Address | `GBNFK4GGHIP7W7UXEGGYIXP2LDJSZII324UJ77T3ZRIWS557RZ6GWTNG` |
| Funding tx (Friendbot) | `18691cda791e92bc968e8ac47d2fc8de58a9a282727a2564850d6398c193ed58` |
| Observed | `10000.0000000` XLM = `$1900.00` |
| Card | Visa `last4=0161`, exp `05/2029`, Stripe id `ic_1UHd7REAzMrENaFXiKe51gGs` |
| Fee tx | `8f841537d4215ccb084b8f49a32df31d33e9f55dd34c9df14ae82a5f85bda66c` |
| Fee charged | `5789474` stroops |
| Vault after run | `payment_count=2`, `total_collected=11578948`, `total_withdrawn=0` |

`card show` returned the test-mode PAN/CVC, `card freeze --confirm` set status
`frozen` with a `frozen_at` timestamp, and `card list` reported both cards with
their live Stripe statuses.

## USDC trustline (classic transaction, live)

`deposit address --asset usdc` → `deposit fund` → `deposit trustline` was also
exercised live, proving the classic transaction build/sign/submit path:

| Item | Value |
|---|---|
| Deposit id | `dep_ca3159bf5a0f` |
| Address | `GASCEZZBGNUDP333XUPFGX25XNWHD2HD64CG6FQZO2GMN2BCJUL34EKZ` |
| Funding tx (Friendbot) | `63d36c5082b8fbbfba472ca45bc93a2a45d8080a258fc86f9eb654d2cd24c5f4` |
| Trustline tx (ChangeTrust USDC) | `abf56d29557e486c644a571945f9a8adecaaad01e2a184c3711d2919a882c0d5` (ledger `4771579`, successful) |

## On-ramp (SEP-1/SEP-10/SEP-24) — live reference anchor

Verified live on 2026-09-20 against SDF's testnet reference anchor
`testanchor.stellar.org` (real SEP-1 fetch, real SEP-10 signed challenge, real
SEP-24 interactive deposit):

| Item | Value |
|---|---|
| Anchor | `testanchor.stellar.org` |
| `onramp info` currencies | `SRT`, `USDC`, `native` (all `status=test`) |
| SEP-24 server | `https://testanchor.stellar.org/sep24` |
| Web auth endpoint | `https://testanchor.stellar.org/auth` |
| Deposit id | `dep_462e642f242f` |
| Account | `GC5XXIANNGOZUDRRDHYCPCGA5OLTONAHTYVSDQZB6FFRM2COLNFDCFPK` |
| Anchor transaction id | `3d5171b7-e5b2-4d83-9e77-108c7c5a35d1` |
| Amount / asset | `10` / `USDC` |
| Interactive URL | `https://anchor-ref-ui-testanchor.stellar.org?transaction_id=3d5171b7-...&token=<redacted>` |
| Status at handoff | `incomplete` (KYC/funding is a browser step by design) |

Notes:

- The anchor caps each asset at `max_amount: 10` and reports
  `account_creation: false`, so the deposit account must be funded and (for USDC)
  hold a trustline before completion.
- The `completed` → credited-deposit branch is covered by the offline integration
  tests (`tests/onramp_flow.rs`); completing KYC live requires a human browser
  session.
- **TRY**: no publicly documented TRY-capable Stellar anchor exists — on mainnet
  or testnet — as of the 2026-09-19 research (SDF directory has no TRY SEP asset;
  KB Trading TRYB is test-status and not in SEP asset lists; BiLira publishes no
  Stellar TOML). The client is anchor-agnostic: `anchor_home_domain`,
  `anchor_sep24_url`, and `anchor_web_auth_endpoint` can point at any SEP-24
  anchor without code changes.

## Web demo edge issuance (Cloudflare Pages, live)

The hosted web demo runs the same real flow from the browser: Freighter signs a
real `collect_fee` transaction, and a Cloudflare Pages Function verifies it on
Horizon before issuing a real Stripe test-mode card. Verified live on 2026-09-20:

| Item | Value |
|---|---|
| Production URL | `https://card.batuhan4.com` (Cloudflare Pages custom domain, SSL valid) |
| Deployments | `f530b807` (first), `8616a559` (after `STRIPE_TEST_KEY` secret) |
| KV namespace | `STELLAR_CARD_KV` = `55926b9b6752448ea08f05d30bf44253` |
| Local run | fee tx `fe9a19fe51a91338425dfe97b93e1242915083eb90f665b0ca313b37a265e163` → Stripe card `ic_1UHdX0EAzMrENaFXlepCYbLj`, last4 `0187` |
| `pages.dev` run | fee tx `8b822a2172a2b6004cd64274a0e8bb7d81f89b0ea4ffc429367395d507190238` → Stripe card `ic_1UHdbSEAzMrENaFXBEQU1yq4`, last4 `0203` |
| Custom-domain run | fee tx `7916e861609a6d6e29eae5d625b720553aa89336ca0b20aec07b75b98aac622f` → Stripe card `ic_1UHdlVEAzMrENaFXLdoGZURr`, last4 `0229` |
| PAN/CVC expand | returned (`4000…` test PAN, `cvc` present, `livemode: false`) |
| Replay guard | same fee tx again → HTTP `409` |
| Wrong-email read | `GET /api/card/:id` with another email → HTTP `403` |
| Freeze | `POST /api/card/:id/freeze` → `status: inactive` |
| Fee verification | Horizon success + freshness < 20 min + native transfer to the vault from the payer + amount ≥ quoted fee (5% price-drift tolerance) |

The static UI no longer fabricates card data: empty and error states are honest,
and every card shown comes from Stripe through the edge API. `npm run build`
(static export) and `npm run lint` pass; `STRIPE_TEST_KEY` lives only as a
Pages secret.

## Full wallet E2E (real Freighter extension, live)

The complete demo path was driven with the **real Freighter extension**
(v5.48.0, unpacked, headless Chromium) and a funded testnet key
(`GB3JDW…BDQBYX`), against `https://card.batuhan4.com` on 2026-09-20:

| Step | Result |
|---|---|
| Wallet unlock + network | unlocked with password, switched to **Test Net** in the extension |
| Connect | Freighter connection-request popup → `Connect anyway` → site shows `GB3JDW…BDQBYX` |
| Home hero CTA | after connect it reads **“Go to Dashboard”** (was a static “Connect Wallet” link) |
| Pay | Freighter confirm-transaction popup (`fee 0.0055858 XLM`) → real Soroban `collect_fee` |
| Fee transactions | `f94de21e89f01196a8b573cb6c7ac198954f7bf28da1a65209d72c1adb214993`, `22fbfc4a8f853bf0e9619a129a6578a0a8497fa57ee0a058a4febfae577c5441` |
| Card issuance | real Stripe test cards via the edge function, e.g. `ic_1UHeJzEAzMrENaFXHHP57gdm` (last4 `0237`) |
| Reveal | real PAN `4000 0099 9000 0237`, expiry `11/29`, CVC `123` |
| Wrong-network path | wallet on Mainnet → site keeps the session, shows a banner and disables payment with “Switch Freighter to Testnet” |

Automation notes: the wallet harness lives outside the repo (extension build,
persistent Chrome profile, mnemonic) because it is environment-specific; the
repo’s `npm run test:e2e` covers page/browser health and the reveal API flow.

## Browser E2E (Playwright, live demo)

`npm --prefix web run test:e2e` drives the deployed demo in headless Chromium:

| Item | Value |
|---|---|
| Result (2026-09-20) | **17 checks, 0 failures** across 4 pages × 4 viewports + reveal flow |
| Viewports | 360×800, 390×844, 768×1024 (touch/mobile), 1440×900 (desktop) |
| Checks | HTTP 200, no horizontal overflow, no console/page errors, no failed requests |
| Interaction | `/cards` "Reveal Card Details" → real edge API → real PAN/CVC rendered |
| Fixes it verified | mobile bottom navigation, stacked page headers, responsive flip card, no desktop hero overflow, mascot rendering |
| Artifacts | screenshots under `web/e2e/artifacts/` (gitignored) |

## Repository test coverage (offline)
`cargo test --workspace` runs 66 tests with no network access:

- 42 library unit tests (`src/`): fee math, classic transaction build/sign/verify,
  Horizon parsing and deposit observation, Soroban simulate/assemble/sign/send
  assembly, SEP-1/10/24 anchor flows, wiremock provider errors.
- 11 integration tests (`tests/`): end-to-end CLI flows against mocked Stripe,
  Horizon, Coinbase, Soroban RPC, and anchor endpoints, including the on-chain
  fee path and the on-ramp flow.
- 13 Soroban contract tests (`contracts/fee-vault`): initialization, auth,
  collection, withdrawal, pricing validation, and error paths.

Live testnet checks are intentionally separate: `scripts/deploy-fee-vault.sh`
and `scripts/demo-testnet.sh`.
