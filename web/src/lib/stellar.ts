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
