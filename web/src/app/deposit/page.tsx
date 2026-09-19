"use client";

import { useState, useCallback } from "react";
import { LazyMotion, domAnimation, m, AnimatePresence } from "framer-motion";
import { Icon } from "@/components/Icon";
import { CrabMascot } from "@/components/CrabMascot";
import { useWallet } from "@/contexts/WalletContext";
import {
  useBalance,
  useDeposit,
  useLocalStorage,
  type DepositRecord,
  type DepositTransaction,
} from "@/hooks/useStellar";
import {
  USDC_ISSUER_TESTNET,
  explorerTxUrl,
  friendbotUrl,
  truncateAddress,
} from "@/lib/stellar";
import dynamic from "next/dynamic";

const QRCodeSVG = dynamic(() => import("qrcode.react").then(m => m.QRCodeSVG), { ssr: false });

type DepositStep = "select" | "awaiting" | "confirmed";

export default function DepositPage() {
  const { address, connected, installed, connecting, connect } = useWallet();
  const { balance, unfunded, refresh } = useBalance();
  const [step, setStep] = useState<DepositStep>("select");
  const [asset, setAsset] = useState<"XLM" | "USDC">("XLM");
  const [copied, setCopied] = useState(false);
  const [, setDeposits] = useLocalStorage<DepositRecord[]>("stellar-card-deposits", []);

  const depositAddress = connected ? address : null;

  const handleConfirmed = useCallback(
    (latest: DepositTransaction) => {
      setStep("confirmed");
      const newDeposit: DepositRecord = {
        id: `dep_${Date.now().toString(36)}`,
        address: depositAddress || "",
        asset,
        amount: `${asset} deposit`,
        usd: "Pending valuation",
        status: "confirmed",
        txHash: latest.hash,
        timestamp: Date.now(),
      };
      setDeposits(prev => [newDeposit, ...prev]);
    },
    [depositAddress, asset, setDeposits]
  );

  const { transactions, confirmed } = useDeposit(
    step === "awaiting" ? depositAddress : null,
    handleConfirmed
  );

  const startMonitoring = useCallback(() => {
    if (!depositAddress) return;
    setStep("awaiting");
  }, [depositAddress]);

  const copyAddress = () => {
    if (!depositAddress) return;
    navigator.clipboard.writeText(depositAddress);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <LazyMotion features={domAnimation} strict>
      <div className="pt-8 px-4 sm:px-8 pb-12 max-w-5xl mx-auto">
        {/* Header */}
        <div className="flex flex-col md:flex-row justify-between items-baseline mb-12 gap-6 animate-fade-in-up">
          <div>
            <h2 className="text-5xl font-headline font-bold text-on-surface tracking-tight leading-none mb-4">Deposit Flow</h2>
            <p className="text-on-surface-variant max-w-md leading-relaxed">Fund your developer account with Stellar.</p>
          </div>
          <div className="bg-inverse-surface text-inverse-on-surface p-4 rounded-xl font-mono text-[13px] text-on-surface-variant">
            <div className="flex items-center gap-2 mb-2">
              <span className="w-2 h-2 rounded-full bg-tertiary animate-pulse" />
              <span className="text-[10px] uppercase tracking-widest font-bold text-tertiary">Network: Testnet</span>
            </div>
            <p><span className="text-green-400">$</span> <span className="text-primary">stellar-card deposit address --asset {asset.toLowerCase()}</span></p>
          </div>
        </div>

        {/* Progress Steps */}
        <div className="flex items-center gap-2 sm:gap-4 mb-12">
          {["Select Asset", "Send Funds", "Confirmed"].map((label, i) => {
            const stepIndex = ["select", "awaiting", "confirmed"].indexOf(step);
            const isComplete = i < stepIndex;
            const isCurrent = i === stepIndex;
            return (
              <div key={label} className="flex items-center gap-2 sm:gap-3">
                <div className={`w-8 h-8 shrink-0 rounded-full flex items-center justify-center text-sm font-bold transition-all ${isComplete ? "bg-tertiary text-on-tertiary" : isCurrent ? "bg-primary text-on-primary" : "bg-surface-container-highest text-outline"}`}>
                  {isComplete ? <Icon name="check" className="text-sm" /> : i + 1}
                </div>
                <span className={`text-xs sm:text-sm font-medium whitespace-nowrap ${isCurrent ? "text-on-surface" : "text-outline"}`}>{label}</span>
                {i < 2 && <div className={`w-5 sm:w-16 h-0.5 ${isComplete ? "bg-tertiary" : "bg-surface-container-highest"}`} />}
              </div>
            );
          })}
        </div>

        <AnimatePresence mode="wait">
          {step === "select" && (
            <m.div key="select" initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="grid lg:grid-cols-12 gap-8">
              {/* Left: Asset selection + address */}
              <div className="lg:col-span-8 bg-surface-container-lowest rounded-xl p-8 shadow-soft-diffuse shadow-soft-diffuse">
                <div className="flex items-center gap-2 mb-6">
                  <Icon name="qr_code_2" className="text-primary" />
                  <span className="font-headline font-bold uppercase text-xs tracking-widest text-primary">Step 01: Select & Send</span>
                </div>
                <h3 className="text-2xl font-headline font-bold mb-4">Select Deposit Asset</h3>
                <div className="grid grid-cols-2 gap-4 mb-8">
                  {(["XLM", "USDC"] as const).map((a) => (
                    <button key={a} onClick={() => setAsset(a)} className={`p-6 rounded-xl transition-all ${asset === a ? "bg-primary/10 luminous-glow" : "bg-surface-container shadow-soft-diffuse hover:bg-surface-container-high"}`}>
                      <div className={`w-12 h-12 rounded-xl mb-4 flex items-center justify-center font-mono font-bold text-sm ${a === "XLM" ? "bg-secondary/10 text-secondary" : "bg-tertiary/10 text-tertiary"}`}>{a}</div>
                      <p className="font-headline font-bold">{a}</p>
                      <p className="text-xs text-outline mt-1">{a === "XLM" ? "Native Stellar" : "USD Coin (trustline)"}</p>
                    </button>
                  ))}
                </div>

                {!connected ? (
                  <div className="bg-surface-container rounded-xl p-6 shadow-soft-diffuse text-center">
                    <Icon name="account_balance_wallet" className="text-3xl text-outline mb-3" />
                    <p className="text-sm text-on-surface-variant mb-4">
                      {installed === false
                        ? "Install Freighter to generate your deposit address."
                        : "Connect Freighter to see your Stellar deposit address."}
                    </p>
                    <button
                      onClick={connect}
                      disabled={connecting || installed === false}
                      className="px-6 py-3 bg-primary text-on-primary rounded-xl font-bold text-sm disabled:opacity-60"
                    >
                      {connecting ? "Connecting..." : "Connect Freighter"}
                    </button>
                  </div>
                ) : (
                  <>
                    <div className="bg-surface-container rounded-xl p-6 shadow-soft-diffuse">
                      <div className="flex items-center justify-between mb-3">
                        <p className="text-[10px] uppercase tracking-widest text-outline font-bold">Your Deposit Address</p>
                        <span className="text-[10px] font-mono text-on-surface-variant">
                          {unfunded
                            ? "Unfunded"
                            : balance !== null
                              ? `${balance.toFixed(4)} XLM`
                              : "Loading..."}
                        </span>
                      </div>
                      <div className="flex items-center gap-3 bg-surface-container-lowest rounded-xl p-4">
                        <span className="font-mono text-sm flex-1 truncate text-primary">{depositAddress}</span>
                        <button
                          onClick={copyAddress}
                          className="shrink-0 w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center hover:bg-primary/20 transition-colors"
                        >
                          <Icon name={copied ? "check" : "content_copy"} className={`text-sm ${copied ? "text-tertiary" : "text-primary"}`} />
                        </button>
                      </div>
                      <div className="mt-4 flex items-center gap-2 text-xs text-outline">
                        <Icon name="info" className="text-sm" />
                        <span>Only send {asset} on Stellar Testnet to this address</span>
                      </div>
                      {asset === "USDC" && (
                        <div className="mt-3 flex items-start gap-2 text-xs text-amber-600">
                          <Icon name="warning" className="text-sm mt-0.5" />
                          <span>
                            USDC requires a trustline to issuer{" "}
                            <span className="font-mono">{truncateAddress(USDC_ISSUER_TESTNET, 6)}</span>{" "}
                            before this account can receive it.
                          </span>
                        </div>
                      )}
                    </div>

                    {unfunded && (
                      <div className="mt-4 flex flex-wrap items-center gap-3">
                        <a
                          href={friendbotUrl(depositAddress ?? "")}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-2 px-5 py-3 bg-tertiary/10 text-tertiary rounded-xl font-bold text-sm hover:bg-tertiary/20 transition-colors"
                        >
                          <Icon name="water_drop" className="text-lg" /> Fund with Friendbot
                        </a>
                        <button
                          onClick={refresh}
                          className="inline-flex items-center gap-2 px-5 py-3 bg-surface-container text-on-surface rounded-xl font-bold text-sm hover:bg-surface-container-high transition-colors"
                        >
                          <Icon name="refresh" className="text-lg" /> Refresh Balance
                        </button>
                      </div>
                    )}

                    <button onClick={startMonitoring} className="w-full mt-6 py-4 bg-primary text-on-primary rounded-xl font-bold text-sm flex items-center justify-center gap-2 shadow-lg shadow-primary-container/20 hover:shadow-primary-container/40 transition-all active:scale-[0.98]">
                      I&apos;ve Sent the Funds <Icon name="arrow_forward" className="text-lg" />
                    </button>
                  </>
                )}
              </div>

              {/* Right: QR + Crab */}
              <div className="lg:col-span-4 flex flex-col items-center justify-center bg-surface-container rounded-xl p-8 shadow-soft-diffuse">
                <div className="w-48 h-48 bg-white rounded-xl shadow-soft-diffuse flex items-center justify-center mb-6">
                  {QRCodeSVG && depositAddress ? (
                    <QRCodeSVG
                      value={depositAddress}
                      size={160}
                      level="M"
                    />
                  ) : (
                    <Icon name="qr_code_2" className="text-5xl text-outline" />
                  )}
                </div>
                <p className="text-sm text-outline text-center mb-6">
                  {depositAddress ? `Scan to deposit ${asset}` : "Connect wallet to show QR code"}
                </p>
                <div className="relative">
                  <CrabMascot size="lg" mood="idle" />
                  <div className="absolute -top-4 -right-8 w-10 h-6 bg-primary text-on-primary rounded-sm shadow-lg animate-card-float" />
                </div>
              </div>
            </m.div>
          )}

          {step === "awaiting" && (
            <m.div key="awaiting" initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -20 }} className="max-w-2xl mx-auto text-center">
              <div className="relative w-32 h-32 mx-auto mb-8">
                <div className="absolute inset-0 rounded-full border-4 border-surface-container-highest" />
                <div className="absolute inset-0 rounded-full border-4 border-transparent border-t-primary animate-spin-slow" />
                <div className="absolute inset-4 rounded-full bg-surface-container-lowest flex items-center justify-center">
                  <Icon name="hourglass_empty" className="text-primary text-3xl" />
                </div>
                {/* Anxious crab */}
                <div className="absolute -bottom-4 -right-8">
                  <CrabMascot size="sm" mood="anxious" />
                </div>
              </div>
              <h3 className="font-headline font-bold text-2xl mb-2">Monitoring Blockchain</h3>
              <p className="text-on-surface-variant mb-8">Waiting for confirmation on Stellar testnet</p>
              <div className="bg-surface-container-highest rounded-full h-3 mb-4 overflow-hidden">
                <div className="h-full bg-gradient-to-r from-primary to-tertiary rounded-full animate-pulse-ring" style={{ width: transactions.length > 0 ? "80%" : "20%" }} />
              </div>
              <p className="text-sm text-outline font-mono">
                {transactions.length > 0 ? `${transactions.length} transaction(s) detected` : "Watching for transactions..."}
              </p>
              <div className="mt-8 bg-inverse-surface text-inverse-on-surface rounded-xl p-5 font-mono text-xs text-left">
                <p className="text-green-400">$ stellar-card deposit status --wait</p>
                <p className="text-primary mt-1">{`{"ok":true,"data":{"status":"${confirmed ? "confirmed" : "pending"}","transactions":${transactions.length}}}`}</p>
              </div>
            </m.div>
          )}

          {step === "confirmed" && (
            <m.div key="confirmed" initial={{ opacity: 0, scale: 0.95 }} animate={{ opacity: 1, scale: 1 }} className="max-w-2xl mx-auto text-center">
              <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-tertiary/10 flex items-center justify-center animate-scale-in">
                <Icon name="check_circle" className="text-tertiary text-5xl" filled />
              </div>
              <h3 className="font-headline font-bold text-3xl mb-2">Deposit Confirmed</h3>
              <p className="text-on-surface-variant mb-2">Your funds have been credited to your account</p>
              {transactions[0] && (
                <a
                  href={explorerTxUrl(transactions[0].hash)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary font-mono text-sm hover:underline"
                >
                  View on Stellar Expert
                </a>
              )}

              <div className="bg-surface-container-lowest rounded-xl p-6 shadow-soft-diffuse shadow-soft-diffuse text-left mt-8 mb-8">
                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div><p className="text-outline text-xs uppercase tracking-widest mb-1">Asset</p><p className="font-mono font-bold">{asset}</p></div>
                  <div><p className="text-outline text-xs uppercase tracking-widest mb-1">Status</p><p className="font-mono font-bold text-tertiary">Confirmed</p></div>
                  {transactions[0] && (
                    <div className="col-span-2"><p className="text-outline text-xs uppercase tracking-widest mb-1">Transaction</p><p className="font-mono text-primary truncate">{transactions[0].hash}</p></div>
                  )}
                </div>
              </div>
              <div className="flex justify-center mb-8"><CrabMascot size="lg" mood="cheer" /></div>
              <div className="flex gap-4 justify-center">
                <a href="/cards" className="px-8 py-4 bg-primary text-on-primary rounded-xl font-bold flex items-center gap-2 shadow-lg shadow-primary-container/20">
                  <Icon name="credit_card" className="text-lg" /> Buy a Card
                </a>
                <a href="/dashboard" className="px-8 py-4 bg-surface-container-highest text-on-surface rounded-xl font-bold hover:bg-surface-bright transition-all">Back to Dashboard</a>
              </div>
            </m.div>
          )}
        </AnimatePresence>
      </div>
    </LazyMotion>
  );
}
