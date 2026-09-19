"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useWallet } from "@/contexts/WalletContext";
import { horizon } from "@/lib/stellar";

export type { CardRecord, DepositRecord } from "@/lib/stellar";

export interface DepositTransaction {
  hash: string;
  successful: boolean;
  createdAt: string;
  ledger: number;
}

interface BalanceState {
  address: string | null;
  balance: number | null;
  unfunded: boolean;
}

interface DepositState {
  address: string | null;
  transactions: DepositTransaction[];
  confirmed: boolean;
}

export function useBalance() {
  const { address } = useWallet();
  const [state, setState] = useState<BalanceState>({
    address: null,
    balance: null,
    unfunded: false,
  });
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!address) return;
    setLoading(true);
    try {
      const account = await horizon.loadAccount(address);
      const native = account.balances.find(
        (entry) => entry.asset_type === "native"
      );
      setState({
        address,
        balance: native ? Number(native.balance) : 0,
        unfunded: false,
      });
    } catch (error) {
      const status = (error as { response?: { status?: number } })?.response
        ?.status;
      if (status === 404) {
        setState({ address, balance: null, unfunded: true });
      }
    } finally {
      setLoading(false);
    }
  }, [address]);

  useEffect(() => {
    if (!address) return;
    const timer = setTimeout(refresh, 0);
    const interval = setInterval(refresh, 15_000);
    return () => {
      clearTimeout(timer);
      clearInterval(interval);
    };
  }, [address, refresh]);

  const current: BalanceState =
    state.address === address
      ? state
      : { address, balance: null, unfunded: false };

  return {
    balance: current.balance,
    unfunded: current.unfunded,
    loading,
    refresh,
  };
}

export function useDeposit(
  address: string | null,
  onConfirmed?: (transaction: DepositTransaction) => void
) {
  const [state, setState] = useState<DepositState>({
    address: null,
    transactions: [],
    confirmed: false,
  });
  const [loading, setLoading] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval>>(undefined);
  const onConfirmedRef = useRef(onConfirmed);

  useEffect(() => {
    onConfirmedRef.current = onConfirmed;
  }, [onConfirmed]);

  useEffect(() => {
    if (!address) return;

    let cancelled = false;

    const poll = async () => {
      setLoading(true);
      try {
        const page = await horizon
          .transactions()
          .forAccount(address)
          .order("desc")
          .limit(5)
          .call();
        if (cancelled) return;
        const latest = page.records.map((record) => ({
          hash: record.hash,
          successful: record.successful,
          createdAt: record.created_at,
          ledger: record.ledger_attr,
        }));
        const confirmed = latest.length > 0 && latest[0].successful;
        setState({ address, transactions: latest, confirmed });
        if (confirmed) {
          if (intervalRef.current) clearInterval(intervalRef.current);
          onConfirmedRef.current?.(latest[0]);
        }
      } catch {
        // Horizon error - keep polling
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    const timer = setTimeout(poll, 0);
    intervalRef.current = setInterval(poll, 5_000);
    return () => {
      cancelled = true;
      clearTimeout(timer);
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [address]);

  const current: DepositState =
    state.address === address
      ? state
      : { address, transactions: [], confirmed: false };

  return {
    transactions: current.transactions,
    confirmed: current.confirmed,
    loading,
  };
}

export function useLocalStorage<T>(key: string, initialValue: T) {
  const [storedValue, setStoredValue] = useState<T>(() => {
    if (typeof window === "undefined") return initialValue;
    try {
      const item = window.localStorage.getItem(key);
      return item ? (JSON.parse(item) as T) : initialValue;
    } catch {
      return initialValue;
    }
  });

  const setValue = useCallback(
    (value: T | ((val: T) => T)) => {
      const valueToStore = value instanceof Function ? value(storedValue) : value;
      setStoredValue(valueToStore);
      if (typeof window !== "undefined") {
        window.localStorage.setItem(key, JSON.stringify(valueToStore));
      }
    },
    [key, storedValue]
  );

  return [storedValue, setValue] as const;
}
