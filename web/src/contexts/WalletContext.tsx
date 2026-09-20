"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  getAddress,
  getNetwork,
  isConnected as freighterIsConnected,
  requestAccess,
  signTransaction as freighterSignTransaction,
} from "@stellar/freighter-api";
import { NETWORK_PASSPHRASE } from "@/lib/stellar";

export interface WalletSignature {
  signedTxXdr: string;
  signerAddress?: string;
}

export interface SignOptions {
  networkPassphrase?: string;
  address?: string;
}

interface WalletContextValue {
  address: string | null;
  connected: boolean;
  installed: boolean | null;
  connecting: boolean;
  error: string | null;
  networkOk: boolean | null;
  connect: () => Promise<void>;
  disconnect: () => void;
  signTransaction: (
    xdr: string,
    opts?: SignOptions
  ) => Promise<WalletSignature>;
}

const WalletContext = createContext<WalletContextValue | null>(null);

const WRONG_NETWORK_MESSAGE =
  "Freighter is on a different network. Open Freighter → Settings → Network → Testnet, then reconnect.";

export function WalletProvider({ children }: { children: ReactNode }) {
  const [address, setAddress] = useState<string | null>(null);
  const [installed, setInstalled] = useState<boolean | null>(null);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [networkOk, setNetworkOk] = useState<boolean | null>(null);

  useEffect(() => {
    let cancelled = false;

    const detect = async () => {
      try {
        const status = await freighterIsConnected();
        if (cancelled) return;
        if (status.error) {
          setInstalled(false);
          return;
        }
        setInstalled(status.isConnected);
        if (!status.isConnected) return;

        const current = await getAddress();
        if (cancelled) return;
        if (!current.error && current.address) {
          const network = await getNetwork();
          if (cancelled) return;
          const correct =
            !network.error &&
            network.networkPassphrase === NETWORK_PASSPHRASE;
          setNetworkOk(correct);
          setAddress(current.address);
          if (!correct) setError(WRONG_NETWORK_MESSAGE);
        }
      } catch {
        if (!cancelled) setInstalled(false);
      }
    };

    detect();
    return () => {
      cancelled = true;
    };
  }, []);

  const connect = useCallback(async () => {
    setConnecting(true);
    setError(null);
    try {
      const network = await getNetwork();
      const correct =
        !network.error && network.networkPassphrase === NETWORK_PASSPHRASE;
      setNetworkOk(correct);
      if (!correct) setError(WRONG_NETWORK_MESSAGE);

      const access = await requestAccess();
      if (access.error) {
        setError(access.error.message || "Freighter denied access");
        return;
      }
      if (!access.address) {
        setError("Freighter returned no address");
        return;
      }
      setAddress(access.address);
      setInstalled(true);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to connect to Freighter"
      );
    } finally {
      setConnecting(false);
    }
  }, []);

  const disconnect = useCallback(() => {
    setAddress(null);
    setError(null);
    setNetworkOk(null);
  }, []);

  const signTransaction = useCallback(
    async (xdr: string, opts?: SignOptions) => {
      if (networkOk === false) {
        throw new Error(WRONG_NETWORK_MESSAGE);
      }
      const result = await freighterSignTransaction(xdr, {
        networkPassphrase: opts?.networkPassphrase ?? NETWORK_PASSPHRASE,
        address: opts?.address ?? address ?? undefined,
      });
      if (result.error) {
        throw new Error(
          result.error.message || "Freighter rejected the transaction"
        );
      }
      return {
        signedTxXdr: result.signedTxXdr,
        signerAddress: result.signerAddress,
      };
    },
    [address, networkOk]
  );

  const value = useMemo<WalletContextValue>(
    () => ({
      address,
      connected: Boolean(address),
      installed,
      connecting,
      error,
      networkOk,
      connect,
      disconnect,
      signTransaction,
    }),
    [
      address,
      installed,
      connecting,
      error,
      networkOk,
      connect,
      disconnect,
      signTransaction,
    ]
  );

  return (
    <WalletContext.Provider value={value}>{children}</WalletContext.Provider>
  );
}

export function useWallet(): WalletContextValue {
  const context = useContext(WalletContext);
  if (!context) {
    throw new Error("useWallet must be used within WalletProvider");
  }
  return context;
}
