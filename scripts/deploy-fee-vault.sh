#!/usr/bin/env bash
set -euo pipefail

# Build, deploy, and initialize the stellar-card Soroban fee vault on Stellar testnet.
#
# Usage:
#   STELLAR_ACCOUNT=<stellar-cli-identity> ./scripts/deploy-fee-vault.sh
#
# Environment:
#   STELLAR_NETWORK    network name for the stellar CLI (default: testnet)
#   STELLAR_ACCOUNT    stellar CLI identity that pays for and administers the vault
#                      (default: stellar-card-owner; generated and funded via Friendbot if missing)
#   FEE_BPS            variable fee in basis points (default: 20)
#   FIXED_FEE_CENTS    fixed fee in USD cents (default: 10)
#   FEE_VAULT_ALIAS    contract alias stored by the stellar CLI (default: stellar-card-fee-vault)
#
# Testnet only. Never use a mainnet secret key with this script.

NETWORK="${STELLAR_NETWORK:-testnet}"
SOURCE="${STELLAR_ACCOUNT:-stellar-card-owner}"
FEE_BPS="${FEE_BPS:-20}"
FIXED_FEE_CENTS="${FIXED_FEE_CENTS:-10}"
ALIAS="${FEE_VAULT_ALIAS:-stellar-card-fee-vault}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v stellar >/dev/null 2>&1; then
  echo "stellar CLI not found. Install it first: https://developers.stellar.org/docs/tools/developer-tools" >&2
  exit 1
fi

if [[ "$NETWORK" != "testnet" && "$NETWORK" != "futurenet" ]]; then
  echo "Refusing to deploy the demo vault to '$NETWORK' (testnet/futurenet only)." >&2
  exit 1
fi

if ! stellar keys ls | grep -qx "$SOURCE"; then
  echo "No stellar CLI identity '$SOURCE' found; generating and funding it via Friendbot..."
  stellar keys generate "$SOURCE" --network "$NETWORK" --fund
fi

AUTHORITY="$(stellar keys address "$SOURCE")"
echo "authority=$AUTHORITY"

echo "Building the fee vault wasm..."
stellar contract build --package stellar-card-fee-vault

WASM="target/wasm32v1-none/release/stellar_card_fee_vault.wasm"
if [[ ! -f "$WASM" ]]; then
  echo "Expected wasm not found at $WASM" >&2
  exit 1
fi

NATIVE_TOKEN="$(stellar contract id asset --asset native --network "$NETWORK")"
echo "native_token=$NATIVE_TOKEN"

echo "Deploying..."
DEPLOY_OUTPUT="$(stellar contract deploy \
  --wasm "$WASM" \
  --network "$NETWORK" \
  --source-account "$SOURCE" \
  --alias "$ALIAS")"

CONTRACT_ID="$(printf '%s\n' "$DEPLOY_OUTPUT" | grep -oE 'C[A-Z2-7]{55}' | tail -n1)"
if [[ -z "$CONTRACT_ID" ]]; then
  echo "Failed to parse a contract id from the deploy output:" >&2
  printf '%s\n' "$DEPLOY_OUTPUT" >&2
  exit 1
fi
echo "contract_id=$CONTRACT_ID"

echo "Initializing the vault..."
stellar contract invoke \
  --id "$CONTRACT_ID" \
  --network "$NETWORK" \
  --source-account "$SOURCE" \
  -- initialize \
  --authority "$AUTHORITY" \
  --native_token "$NATIVE_TOKEN" \
  --fee_bps "$FEE_BPS" \
  --fixed_fee_cents "$FIXED_FEE_CENTS"

cat <<EOF

Fee vault deployed and initialized on $NETWORK.

CLI configuration:
  stellar-card config set network testnet
  stellar-card config set fee_contract_id $CONTRACT_ID
  stellar-card config set onchain_fee_collection_enabled true
  stellar-card config set fee_fixed_cents $FIXED_FEE_CENTS
  stellar-card config set fee_variable_bps $FEE_BPS

Web demo (web/.env.local):
  NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID=$CONTRACT_ID

Explorer:
  https://stellar.expert/explorer/testnet/contract/$CONTRACT_ID
EOF
