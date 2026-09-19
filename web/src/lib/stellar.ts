import { Horizon, Networks, contract } from "@stellar/stellar-sdk";

export const HORIZON_TESTNET_URL = "https://horizon-testnet.stellar.org";
export const SOROBAN_TESTNET_RPC_URL = "https://soroban-testnet.stellar.org";
export const NETWORK_PASSPHRASE = Networks.TESTNET;
export const FRIENDBOT_URL = "https://friendbot.stellar.org";
export const USDC_ISSUER_TESTNET =
  "GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5";
export const USDC_ASSET_CODE = "USDC";

export const STROOPS_PER_XLM = 10_000_000;
export const DEFAULT_FIXED_FEE_CENTS = 10;
export const DEFAULT_VARIABLE_FEE_BPS = 20;

export const FEE_VAULT_CONTRACT_ID =
  process.env.NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID ?? "";
export const FEE_VAULT_CONTRACT_ID_PLACEHOLDER =
  "C...SET_NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID";

export const STELLAR_EXPERT_URL = "https://stellar.expert/explorer/testnet";

export const horizon = new Horizon.Server(HORIZON_TESTNET_URL);

export interface FeeVaultContract {
  collect_fee: (
    args: { payer: string; amount: bigint; fee_reference: Uint8Array },
    opts?: contract.MethodOptions
  ) => Promise<contract.AssembledTransaction<null>>;
}

export interface DepositRecord {
  id: string;
  address: string;
  asset: "XLM" | "USDC";
  amount: string;
  usd: string;
  status: "pending" | "confirmed";
  txHash?: string;
  timestamp: number;
}

export interface CardRecord {
  id: string;
  name: string;
  last4: string;
  exp: string;
  cvc: string;
  balance: string;
  status: "active" | "frozen";
  feeTxHash?: string;
  createdAt: number;
}

export function isFeeVaultConfigured(): boolean {
  return /^C[A-Z2-7]{55}$/.test(FEE_VAULT_CONTRACT_ID);
}

export function cardFeeCents(
  cardAmountCents: number,
  fixedFeeCents: number = DEFAULT_FIXED_FEE_CENTS,
  feeBps: number = DEFAULT_VARIABLE_FEE_BPS
): number {
  const variableFee = Math.ceil((cardAmountCents * feeBps) / 10_000);
  return fixedFeeCents + variableFee;
}

export function usdCentsToStroops(feeCents: number, xlmPriceUsd: number): bigint {
  const priceCents = Math.round(xlmPriceUsd * 100);
  if (priceCents <= 0) throw new Error("XLM price must be > 0");
  const stroops = Math.ceil((feeCents * STROOPS_PER_XLM) / priceCents);
  return BigInt(Math.max(stroops, 1));
}

export function truncateAddress(address: string, chars = 4): string {
  return `${address.slice(0, chars)}...${address.slice(-chars)}`;
}

export function generateFeeReference(): Uint8Array {
  const ref = new Uint8Array(16);
  crypto.getRandomValues(ref);
  return ref;
}

export function explorerTxUrl(hash: string): string {
  return `${STELLAR_EXPERT_URL}/tx/${hash}`;
}

export function explorerContractUrl(contractId: string): string {
  return `${STELLAR_EXPERT_URL}/contract/${contractId}`;
}

export function explorerAccountUrl(address: string): string {
  return `${STELLAR_EXPERT_URL}/account/${address}`;
}

export function friendbotUrl(address: string): string {
  return `${FRIENDBOT_URL}?addr=${encodeURIComponent(address)}`;
}

export interface IssuedCard {
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
}

export interface CardSummary {
  id: string;
  last4: string;
  exp: string;
  status: string;
  brand: string;
  created: number;
  amountUsd: number | null;
  feeTxHash: string | null;
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });
  const body = (await response.json().catch(() => null)) as {
    error?: string;
  } | null;
  if (!response.ok) {
    throw new Error(
      body?.error ?? `Edge API request failed (${response.status})`
    );
  }
  return body as T;
}

export function isValidEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

export async function issueCardViaApi(input: {
  payer: string;
  txHash: string;
  amountUsd: number;
  name: string;
  email: string;
}): Promise<IssuedCard> {
  const body = await api<{ card: IssuedCard }>("/api/card", {
    method: "POST",
    body: JSON.stringify(input),
  });
  return body.card;
}

export async function listCards(email: string): Promise<CardSummary[]> {
  const body = await api<{ cards: CardSummary[] }>(
    `/api/cards?email=${encodeURIComponent(email)}`
  );
  return body.cards;
}

export async function getCard(id: string, email: string): Promise<IssuedCard> {
  const body = await api<{ card: IssuedCard }>(
    `/api/card/${encodeURIComponent(id)}?email=${encodeURIComponent(email)}`
  );
  return body.card;
}

export async function freezeCardViaApi(
  id: string,
  email: string
): Promise<IssuedCard> {
  const body = await api<{ card: IssuedCard }>(
    `/api/card/${encodeURIComponent(id)}/freeze`,
    { method: "POST", body: JSON.stringify({ email }) }
  );
  return body.card;
}

export function groupCardNumber(number: string): string {
  return number
    .replace(/\s+/g, "")
    .replace(/(.{4})/g, "$1 ")
    .trim();
}
