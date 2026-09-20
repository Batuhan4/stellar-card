"use client";

import { useWallet } from "@/contexts/WalletContext";
import { useBalance } from "@/hooks/useStellar";
import { truncateAddress } from "@/lib/stellar";
import { Icon } from "./Icon";

export function ConnectWallet() {
  const { address, connected, installed, connecting, error, connect, disconnect } =
    useWallet();

  if (installed === false) {
    return (
      <>
        <a
          href="https://www.freighter.app/"
          target="_blank"
          rel="noopener noreferrer"
          className="hidden sm:inline-flex items-center gap-2 px-4 py-2 sm:px-7 sm:py-3 rounded-xl font-semibold text-xs sm:text-sm text-white shadow-lg transition-all hover:opacity-90"
          style={{ background: "linear-gradient(135deg, #0043eb, #3962ff)" }}
        >
          <Icon name="account_balance_wallet" className="text-[18px]" filled />
          Install Freighter
        </a>
        <a
          href="https://www.freighter.app/"
          target="_blank"
          rel="noopener noreferrer"
          className="sm:hidden inline-flex items-center gap-1.5 px-3 py-2 rounded-xl font-semibold text-[11px] bg-surface-container text-on-surface-variant"
        >
          <Icon name="desktop_windows" className="text-[16px]" />
          Desktop wallet
        </a>
      </>
    );
  }

  if (connected && address) {
    return (
      <div className="flex items-center gap-2 bg-surface-container-low rounded-xl px-3 py-2">
        <span className="w-2 h-2 rounded-full bg-tertiary animate-pulse" />
        <span className="font-mono text-xs font-medium text-slate-900">
          {truncateAddress(address, 6)}
        </span>
        <button
          onClick={disconnect}
          title="Disconnect Freighter"
          className="w-7 h-7 rounded-lg flex items-center justify-center text-outline hover:bg-surface-container-high hover:text-error transition-colors"
        >
          <Icon name="logout" className="text-[16px]" />
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-end gap-1">
      <button
        onClick={connect}
        disabled={connecting}
        className="inline-flex items-center gap-2 px-4 py-2 sm:px-7 sm:py-3 rounded-xl font-semibold text-xs sm:text-sm text-white shadow-lg transition-all hover:opacity-90 disabled:opacity-60"
        style={{ background: "linear-gradient(135deg, #0043eb, #3962ff)" }}
      >
        <Icon name="account_balance_wallet" className="text-[18px]" filled />
        {connecting ? "Connecting..." : "Connect Freighter"}
      </button>
      {error && <span className="text-[10px] text-error">{error}</span>}
    </div>
  );
}

export function WalletInfo() {
  const { address, connected, installed, connecting, error, connect } =
    useWallet();
  const { balance, unfunded } = useBalance();

  if (!connected || !address) {
    return (
      <div className="bg-surface-container-low rounded-xl p-3 space-y-2">
        <p className="text-xs text-slate-400 text-center">
          {installed === false
            ? "Freighter is not installed"
            : "Connect Freighter to begin"}
        </p>
        <button
          onClick={connect}
          disabled={connecting || installed === false}
          className="w-full py-2 rounded-lg bg-primary text-on-primary text-xs font-semibold disabled:opacity-60"
        >
          {connecting ? "Connecting..." : "Connect Freighter"}
        </button>
        {error && (
          <p className="text-[10px] text-error text-center">{error}</p>
        )}
      </div>
    );
  }

  return (
    <div className="bg-surface-container-low rounded-xl p-3 space-y-2">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-full bg-primary flex items-center justify-center text-white text-xs font-bold">
          {address.slice(0, 2)}
        </div>
        <div className="overflow-hidden">
          <p className="text-xs font-mono font-medium text-slate-900 truncate">
            {truncateAddress(address, 6)}
          </p>
          <p className="text-[10px] text-slate-400">
            {unfunded
              ? "Unfunded account"
              : balance !== null
                ? `${balance.toFixed(4)} XLM`
                : "Loading..."}
          </p>
        </div>
      </div>
    </div>
  );
}
