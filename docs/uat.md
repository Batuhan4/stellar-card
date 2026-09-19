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

## Repository test coverage (offline)

`cargo test --workspace` runs 54 tests with no network access:

- 33 library unit tests (`src/`): fee math, classic transaction build/sign/verify,
  Horizon parsing and deposit observation, Soroban simulate/assemble/sign/send
  assembly, wiremock provider errors.
- 8 integration tests (`tests/`): end-to-end CLI flows against mocked Stripe,
  Horizon, Coinbase, and Soroban RPC, including the on-chain fee path.
- 13 Soroban contract tests (`contracts/fee-vault`): initialization, auth,
  collection, withdrawal, pricing validation, and error paths.

Live testnet checks are intentionally separate: `scripts/deploy-fee-vault.sh`
and `scripts/demo-testnet.sh`.
