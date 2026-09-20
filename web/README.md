# StellarCard Web

Next.js web surface for **StellarCard**, an agent-first virtual-card CLI funded on Stellar testnet. The app connects to Freighter, reads live testnet balances and deposits from Horizon, and collects card fees on-chain through the Soroban fee-vault contract.

## Requirements

- Node.js 22 or newer
- npm
- [Freighter](https://www.freighter.app/) browser extension switched to **Testnet**

## Setup

```bash
npm install
```

Create `.env.local` in this directory and set the deployed fee-vault contract ID:

```bash
NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID=C...
```

The value must be the contract ID (`C...`) of the deployed `fee-vault` Soroban contract. Until it is set, the Cards page shows a configuration error and fee payment is disabled. No contract ID is shipped with the repository because nothing is deployed by default.

## Run

```bash
npm run dev    # development server on http://localhost:3000
npm run build  # production build
npm run lint   # eslint
```

## Pages

- `/` — landing page with pricing and product overview
- `/dashboard` — XLM balance, card inventory, recent deposits
- `/deposit` — deposit address and QR, Friendbot funding, Horizon transaction monitoring
- `/cards` — demo card reveal plus the on-chain fee payment flow

## Wallet flow

`WalletProvider` (`src/contexts/WalletContext.tsx`) wraps `@stellar/freighter-api`:

1. On load it calls `isConnected()` to detect the extension and `getAddress()` to restore an already-allowed session.
2. `connect()` calls `requestAccess()`; the returned address is stored in context.
3. `signTransaction()` wraps Freighter's signer with the testnet passphrase and throws on wallet errors.
4. `disconnect()` clears the local session. Freighter has no revoke call, so site permission remains until removed inside the extension.

The wallet must be on Testnet. Freighter signs the `collect_fee` invocation with the connected account as payer and transaction source.

## Network and assets

- Horizon: `https://horizon-testnet.stellar.org`
- Soroban RPC: `https://soroban-testnet.stellar.org`
- Network passphrase: `Test SDF Network ; September 2015`
- Friendbot: `https://friendbot.stellar.org?addr={G...}`
- USDC testnet issuer: `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`

USDC recipients need a trustline to that issuer before the account can receive USDC. XLM is the native asset and needs no trustline; unfunded accounts can be created with Friendbot.

## Fee model

Card fees are `$0.10` fixed plus `0.20%` (20 bps) of the card amount. The USD fee is converted to stroops using the live Coinbase `XLM-USD/spot` price and passed to the contract as an `i128`. The contract is called as:

```ts
collect_fee(payer: Address, amount: i128, fee_reference: BytesN<16>)
```

## Secrets

Only public configuration belongs here. `NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID` is a public contract address; never commit secret keys or API tokens.
