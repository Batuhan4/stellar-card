export interface KV {
  get(key: string): Promise<string | null>;
  put(
    key: string,
    value: string,
    options?: { expirationTtl?: number }
  ): Promise<void>;
  delete(key: string): Promise<void>;
}

export interface Env {
  STRIPE_TEST_KEY?: string;
  FEE_VAULT_CONTRACT_ID?: string;
  HORIZON_URL?: string;
  COINBASE_URL?: string;
  STELLAR_CARD_KV?: KV;
}

export interface StripeCardholder {
  id: string;
  email?: string;
  name?: string;
}

export interface StripeCard {
  id: string;
  last4: string;
  exp_month: number;
  exp_year: number;
  status: string;
  brand?: string;
  number?: string;
  cvc?: string;
  livemode?: boolean;
  created?: number;
}

interface HorizonBalanceChange {
  asset_type?: string;
  type?: string;
  from?: string;
  to?: string;
  amount?: string;
}

interface HorizonOperation {
  type?: string;
  asset_balance_changes?: HorizonBalanceChange[];
}

export interface CardMapping {
  email: string;
  payer: string;
  txHash: string;
  amountUsd: number;
  name: string;
  demo?: boolean;
}

export const DEFAULT_HORIZON_URL = "https://horizon-testnet.stellar.org";
export const DEFAULT_COINBASE_URL = "https://api.coinbase.com";

const STROOPS_PER_XLM = 10_000_000;
const FEE_FRESHNESS_MS = 20 * 60 * 1000;

export class HttpError extends Error {
  constructor(
    public status: number,
    message: string
  ) {
    super(message);
  }
}

export function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    },
  });
}

export function fail(status: number, message: string): Response {
  return json({ ok: false, error: message }, status);
}

export function errorResponse(error: unknown): Response {
  if (error instanceof HttpError) return fail(error.status, error.message);
  const message = error instanceof Error ? error.message : "Unexpected error";
  return fail(500, message);
}

export function isGAddress(value: string): boolean {
  return /^G[A-Z2-7]{55}$/.test(value);
}

export function isTxHash(value: string): boolean {
  return /^[0-9a-f]{64}$/i.test(value);
}

export function isEmail(value: string): boolean {
  return value.length <= 254 && /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

export async function stripeRequest<T>(
  env: Env,
  path: string,
  init?: { method?: string; body?: URLSearchParams }
): Promise<T> {
  if (!env.STRIPE_TEST_KEY) {
    throw new HttpError(500, "STRIPE_TEST_KEY secret is not configured");
  }
  const response = await fetch(`https://api.stripe.com${path}`, {
    method: init?.method ?? "GET",
    headers: {
      Authorization: `Bearer ${env.STRIPE_TEST_KEY}`,
      "Content-Type": "application/x-www-form-urlencoded",
    },
    body: init?.body,
  });
  const body = (await response.json()) as {
    error?: { message?: string };
  };
  if (!response.ok) {
    const message =
      body?.error?.message ?? `Stripe request failed (${response.status})`;
    throw new HttpError(response.status === 401 ? 500 : 402, message);
  }
  return body as T;
}

async function horizonJson<T>(env: Env, path: string): Promise<T> {
  const base = env.HORIZON_URL || DEFAULT_HORIZON_URL;
  const response = await fetch(`${base}${path}`, {
    headers: { Accept: "application/json" },
  });
  if (!response.ok) {
    throw new HttpError(502, `Horizon request failed (${response.status})`);
  }
  return (await response.json()) as T;
}

async function xlmSpotPrice(env: Env): Promise<number> {
  const base = env.COINBASE_URL || DEFAULT_COINBASE_URL;
  const response = await fetch(`${base}/v2/prices/XLM-USD/spot`);
  if (!response.ok) {
    throw new HttpError(502, "Could not read the XLM-USD spot price");
  }
  const body = (await response.json()) as { data?: { amount?: string } };
  const price = Number(body?.data?.amount);
  if (!Number.isFinite(price) || price <= 0) {
    throw new HttpError(502, "XLM-USD spot price is invalid");
  }
  return price;
}

export function feeCentsFor(amountCents: number): number {
  return 10 + Math.ceil((amountCents * 20) / 10_000);
}

export interface VerifiedFee {
  feeXlm: string;
  stroops: string;
  requiredStroops: string;
}

export async function verifyFeePayment(
  env: Env,
  params: { txHash: string; payer: string; amountCents: number }
): Promise<VerifiedFee> {
  if (!env.FEE_VAULT_CONTRACT_ID) {
    throw new HttpError(500, "FEE_VAULT_CONTRACT_ID is not configured");
  }

  const tx = await horizonJson<{ successful?: boolean; created_at?: string }>(
    env,
    `/transactions/${params.txHash}`
  );
  if (!tx.successful) {
    throw new HttpError(402, "The fee transaction did not succeed on-chain");
  }
  const createdAt = Date.parse(tx.created_at ?? "");
  if (!Number.isFinite(createdAt) || Date.now() - createdAt > FEE_FRESHNESS_MS) {
    throw new HttpError(
      402,
      "The fee transaction is older than 20 minutes; pay the fee again"
    );
  }

  const operations = await horizonJson<{
    _embedded?: { records?: HorizonOperation[] };
  }>(env, `/transactions/${params.txHash}/operations`);
  const records = operations._embedded?.records ?? [];
  const transfer = records
    .flatMap((operation) => operation.asset_balance_changes ?? [])
    .find(
      (change) =>
        change.asset_type === "native" &&
        change.type === "transfer" &&
        change.to === env.FEE_VAULT_CONTRACT_ID &&
        change.from === params.payer
    );
  if (!transfer?.amount) {
    throw new HttpError(
      402,
      "No fee payment from this account to the fee vault was found in that transaction"
    );
  }

  const paidStroops = BigInt(Math.round(Number(transfer.amount) * STROOPS_PER_XLM));
  const price = await xlmSpotPrice(env);
  const priceCents = Math.round(price * 100);
  const requiredCents = feeCentsFor(params.amountCents);
  const requiredStroops = BigInt(
    Math.max(Math.ceil((requiredCents * STROOPS_PER_XLM) / priceCents), 1)
  );
  const tolerance = requiredStroops - requiredStroops / BigInt(20);
  if (paidStroops < tolerance) {
    throw new HttpError(
      402,
      `On-chain fee of ${transfer.amount} XLM is below the required fee`
    );
  }
  return {
    feeXlm: transfer.amount,
    stroops: paidStroops.toString(),
    requiredStroops: requiredStroops.toString(),
  };
}

export async function findOrCreateCardholder(
  env: Env,
  params: { name: string; email: string; ip: string }
): Promise<StripeCardholder> {
  const listed = await stripeRequest<{ data?: StripeCardholder[] }>(
    env,
    "/v1/issuing/cardholders?limit=100"
  );
  const existing = (listed.data ?? []).find(
    (cardholder) => (cardholder.email ?? "").toLowerCase() === params.email
  );
  if (existing) return existing;

  const parts = params.name.split(/\s+/).filter(Boolean);
  const firstName = parts[0] ?? "Demo";
  const lastName = parts.slice(1).join(" ") || "User";
  const body = new URLSearchParams({
    type: "individual",
    name: params.name,
    email: params.email,
    "individual[first_name]": firstName,
    "individual[last_name]": lastName,
    "individual[dob][day]": "1",
    "individual[dob][month]": "1",
    "individual[dob][year]": "1990",
    "billing[address][line1]": "510 Townsend Street",
    "billing[address][city]": "San Francisco",
    "billing[address][state]": "CA",
    "billing[address][postal_code]": "94103",
    "billing[address][country]": "US",
    "individual[card_issuing][user_terms_acceptance][date]": String(
      Math.floor(Date.now() / 1000)
    ),
    "individual[card_issuing][user_terms_acceptance][ip]": params.ip,
  });
  return stripeRequest<StripeCardholder>(env, "/v1/issuing/cardholders", {
    method: "POST",
    body,
  });
}

export interface SerializedCard {
  id: string;
  last4: string;
  exp: string;
  status: string;
  brand: string;
  number: string | null;
  cvc: string | null;
  livemode: boolean;
  amountUsd: number;
  holderName: string;
  feeTxHash: string;
  demo: boolean;
}

export function serializeCard(
  card: StripeCard,
  extra: {
    amountUsd: number;
    holderName: string;
    feeTxHash: string;
    demo?: boolean;
  }
): SerializedCard {
  const brand = card.brand ?? "visa";
  return {
    id: card.id,
    last4: card.last4,
    exp: `${String(card.exp_month).padStart(2, "0")}/${String(
      card.exp_year
    ).slice(-2)}`,
    status: card.status,
    brand: brand.charAt(0).toUpperCase() + brand.slice(1),
    number: card.number ?? null,
    cvc: card.cvc ?? null,
    livemode: card.livemode ?? false,
    amountUsd: extra.amountUsd,
    holderName: extra.holderName,
    feeTxHash: extra.feeTxHash,
    demo: extra.demo ?? false,
  };
}

export function groupCardNumber(number: string): string {
  return number.replace(/\s+/g, "").replace(/(.{4})/g, "$1 ").trim();
}
