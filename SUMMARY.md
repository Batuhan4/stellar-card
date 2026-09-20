# StellarCard — Project Summary

**Buy a virtual Visa card in three taps. No wallet, no crypto knowledge, no seed
phrase. Stellar settles underneath; the user never has to know.**

Live demo: <https://card.batuhan4.com> · Code: <https://github.com/Batuhan4/stellar-card>

---

## The problem we solve

For most people, spending money online still means a bank card. Meanwhile a huge
amount of value now lives on public blockchains, and almost none of it can be
spent without learning wallets, networks, gas, and bridges. The people who lose
out are not developers — they are freelancers, remote workers, and anyone paid
in digital dollars who just wants a card that works on a checkout page.

Crypto products have spent a decade asking users to learn crypto first. We think
that is backwards. The user should learn nothing.

## What StellarCard does

1. Open the site, tap **Quick Buy with TRY**.
2. Choose an amount (for example ₺500) and get an **IBAN with a reference code**.
3. Send the money from any Turkish bank app, tap once, and the **virtual card
   appears** with number, expiry, and CVC — ready to use at any online checkout.

No extension, no wallet, no network selection, no gas token. It works on a phone.
Under the hood, the money is settled through Stellar rails (USDC and the
Stellar-native token flow), the service fee is collected by a Soroban contract
on-chain, and the card itself is issued by Stripe Issuing. The user experience
is a bank transfer and a card. That is the whole product.

## Who it is for

- **Humans first.** Anyone with a bank account — starting with Türkiye — who
  wants a card without opening a crypto wallet or understanding Stellar.
- **AI agents are a bonus.** The same core is a JSON-in/JSON-out CLI: agents can
  fund a card, pay the fee on-chain, and issue it without a human in the loop.
  Those users get a clean, programmable surface; they are not the pitch.

## How we make money

**$0.10 + 0.20% per card.** No subscription, no monthly fee, no hidden FX spread
invented by us. A $100 card costs $0.30 in service fee — ten cents fixed plus
0.20% — and the fee is settled on-chain through the Soroban fee vault, so it is
visible and verifiable instead of buried in a bank statement.

Every card sold is also a new person using Stellar: the first Stellar
transaction most of our users will ever touch happens because they wanted a
card, not because they wanted a blockchain.

## What already works (verified on Stellar testnet)

- A deployed Soroban **fee vault** contract collects the service fee on-chain;
  every payment is a real transaction on Stellar Expert.
- The **Quick Buy** flow: IBAN + reference → transfer check → card, no wallet.
- The **no-BS developer path**: Rust CLI (`stellar-card`) with deposits, on-chain
  fees, card buy/reveal/freeze, structured JSON, and a SEP-1/10/24 anchor client
  for fiat on-ramps.
- The hosted web demo issues **real Stripe test-mode cards** from an edge
  function after verifying the payment, with rate limits and no secrets in the
  browser.

## What is honest about today

This is a testnet build. There is no licensed TRY on-ramp partner yet, so in the
public demo the TRY leg is simulated and the cards are Stripe test-mode cards —
clearly labeled everywhere. The Stellar settlement, the fee vault, and the card
issuance are real code paths, not mockups.

## What comes next

1. Partner with a licensed payment provider for the TRY→USDC leg.
2. Flip Stripe and the vault to live mode with the same transparent fee
   ($0.10 + 0.20%, visible on-chain).
3. Open a second corridor (remote-worker payouts in USD/EUR) using the same SEP
   on-ramp client and fee vault. The rail that matters is Stellar; the corridor
   is a config line.
