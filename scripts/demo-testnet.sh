#!/usr/bin/env bash
set -euo pipefail

# End-to-end testnet demo for the stellar-card CLI.
#
# Requires:
#   - `cargo build --release` (or STELLAR_CARD_BIN pointing at the binary)
#   - Stripe test mode authenticated: `stellar-card auth login --api-key sk_test_...`
#   - The fee vault configured:
#       stellar-card config set fee_contract_id <C...>
#       stellar-card config set onchain_fee_collection_enabled true
#
# Usage: ./scripts/demo-testnet.sh [idempotency-prefix]
#
# Testnet only. Burns only Friendbot-funded test XLM and Stripe test-mode cards.

BIN="${STELLAR_CARD_BIN:-./target/release/stellar-card}"
PREFIX="${1:-demo-$(date +%s)}"

if [[ ! -x "$BIN" ]]; then
  echo "stellar-card binary not found at $BIN. Run: cargo build --release" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required for this demo script." >&2
  exit 1
fi

step() {
  echo
  echo "=== $* ==="
}

step "Create an XLM deposit address"
DEPOSIT_JSON="$("$BIN" deposit address --asset xlm --idempotency-key "$PREFIX-deposit")"
echo "$DEPOSIT_JSON"
DEPOSIT_ID="$(echo "$DEPOSIT_JSON" | jq -r '.data.id')"
ADDRESS="$(echo "$DEPOSIT_JSON" | jq -r '.data.address')"
echo "deposit_id=$DEPOSIT_ID address=$ADDRESS"

step "Fund the deposit address from Friendbot"
"$BIN" deposit fund "$DEPOSIT_ID"

step "Wait for the deposit to be observed on Horizon"
"$BIN" deposit status "$DEPOSIT_ID" --wait --timeout 180

step "Balance"
"$BIN" balance

step "Buy a \$5.00 virtual card (Stripe test mode + on-chain fee)"
"$BIN" card buy --amount 5.00 --idempotency-key "$PREFIX-buy"
