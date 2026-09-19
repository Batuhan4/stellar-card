#!/usr/bin/env bash
set -euo pipefail

# Deploy the web demo (Next.js static export + Cloudflare Pages Functions).
#
# Requires:
#   - npm install in web/
#   - `wrangler login` against the Cloudflare account that owns the Pages project
#   - STRIPE_TEST_KEY set as a Pages secret:
#       grep -oP '^STRIPE_TEST_KEY=\K.*' ~/.config/stellar-card/.env \
#         | npx wrangler pages secret put STRIPE_TEST_KEY --project-name stellar-card
#
# Test mode only: issues Stripe test cards, never live cards.

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROJECT="${PAGES_PROJECT:-stellar-card}"
BRANCH="${PAGES_BRANCH:-main}"

cd "$ROOT/web"
npm run build
npx wrangler pages deploy --project-name "$PROJECT" --branch "$BRANCH"
