"use client";

import { useEffect, useState } from "react";
import { LazyMotion, domAnimation, m } from "framer-motion";
import { Icon } from "@/components/Icon";
import { VirtualCard } from "@/components/VirtualCard";
import { CrabMascot } from "@/components/CrabMascot";
import Link from "next/link";
import { useLocalStorage, useBalance, type DepositRecord } from "@/hooks/useStellar";
import { explorerTxUrl, isValidEmail, listCards, type CardSummary } from "@/lib/stellar";

export default function DashboardPage() {
  const { balance } = useBalance();
  const [cardholderName] = useLocalStorage(
    "stellar-card-holder-name",
    "StellarCard Demo User"
  );
  const [cardholderEmail] = useLocalStorage(
    "stellar-card-holder-email",
    "demo@stellar-card.dev"
  );
  const [deposits] = useLocalStorage<DepositRecord[]>("stellar-card-deposits", []);
  const [cards, setCards] = useState<CardSummary[]>([]);

  useEffect(() => {
    const email = cardholderEmail.trim().toLowerCase();
    if (!isValidEmail(email)) return;
    let cancelled = false;
    listCards(email)
      .then((list) => {
        if (!cancelled) setCards(list);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [cardholderEmail]);

  const activeCards = cards.filter((c) => c.status === "active");
  const confirmedDeposits = deposits.filter(d => d.status === "confirmed");

  return (
    <LazyMotion features={domAnimation} strict>
      {/* Header */}
      <header className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between mb-12 animate-fade-in-up">
        <div>
          <h2 className="text-3xl font-headline font-bold tracking-tight">System Overview</h2>
          <p className="text-outline text-sm">Real-time financial telemetry & card management</p>
        </div>
        <div className="flex gap-4">
          <button className="bg-surface-container-highest px-5 py-2 rounded-lg font-medium text-sm flex items-center gap-2 transition-all hover:bg-surface-bright">
            <Icon name="download" className="text-lg" /> Report
          </button>
          <Link href="/deposit" className="bg-primary text-on-primary px-5 py-2 rounded-lg font-medium text-sm flex items-center gap-2 transition-all shadow-soft-diffuse hover:opacity-90">
            <Icon name="add" className="text-lg" /> New Deposit
          </Link>
        </div>
      </header>

      {/* KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-10">
        {[
          { label: "Available Balance", icon: "account_balance", color: "primary" },
          { label: "Active Cards", icon: "credit_card", color: "secondary" },
          { label: "Deposits", icon: "account_balance_wallet", color: "tertiary" },
        ].map((kpi, i) => (
          <m.div
            key={kpi.label}
            className="bg-surface-container-lowest rounded-2xl p-6 shadow-soft-diffuse shadow-soft-diffuse"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.1, duration: 0.4 }}
          >
            <div className="flex justify-between items-start mb-4">
              <div className={`w-10 h-10 rounded-xl bg-${kpi.color}/10 flex items-center justify-center`}>
                <Icon name={kpi.icon} className={`text-${kpi.color}`} />
              </div>
              <span className="text-[10px] font-mono text-outline uppercase tracking-widest">{kpi.label}</span>
            </div>
            {kpi.label === "Available Balance" ? (
              <>
                <p className="text-3xl font-headline font-bold tracking-tight text-on-surface">
                  {balance !== null ? `${balance.toFixed(4)} XLM` : "— XLM"}
                </p>
                <p className="text-xs text-on-surface-variant mt-1">
                  {balance !== null ? "Connected" : "Connect wallet to see balance"}
                </p>
              </>
            ) : kpi.label === "Active Cards" ? (
              <>
                <p className="text-3xl font-headline font-bold tracking-tight">{activeCards.length}</p>
                <p className="text-xs text-on-surface-variant mt-1">{cards.length} total issued</p>
              </>
            ) : (
              <>
                <p className="text-3xl font-headline font-bold tracking-tight">{confirmedDeposits.length}</p>
                <p className="text-xs text-on-surface-variant mt-1">{deposits.length} total</p>
              </>
            )}
          </m.div>
        ))}
      </div>

      <div className="grid lg:grid-cols-12 gap-8">
        {/* Active Cards */}
        <section className="lg:col-span-7">
          <div className="flex justify-between items-center mb-6">
            <h3 className="font-headline font-bold text-lg">Card Inventory</h3>
            <Link href="/cards" className="text-primary text-sm font-medium hover:underline flex items-center gap-1">
              View all <Icon name="arrow_forward" className="text-sm" />
            </Link>
          </div>

          {cards.length === 0 ? (
            <div className="bg-surface-container-lowest rounded-2xl p-8 shadow-soft-diffuse shadow-soft-diffuse text-center">
              <Icon name="credit_card" className="text-4xl text-outline mb-4" />
              <p className="text-on-surface-variant text-sm">No cards yet. Deposit XLM and buy your first card.</p>
              <Link href="/deposit" className="inline-flex mt-4 px-6 py-3 bg-primary text-on-primary rounded-lg font-medium text-sm">
                Make a Deposit
              </Link>
            </div>
          ) : (
            <div className="space-y-4">
              {cards.map((card, i) => (
                <m.div
                  key={card.id}
                  className="bg-surface-container-lowest rounded-2xl p-5 shadow-soft-diffuse shadow-soft-diffuse flex items-center justify-between hover:bg-surface-container transition-all group"
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: 0.3 + i * 0.08 }}
                >
                  <div className="flex items-center gap-4 flex-1 min-w-0">
                    <div className="w-12 h-8 shrink-0 rounded-lg bg-primary text-on-primary flex items-center justify-center">
                      <span className="text-white font-mono text-[10px]">{card.last4}</span>
                    </div>
                    <div className="min-w-0">
                      <p className="font-mono font-medium text-sm truncate">{cardholderName}</p>
                      <p className="text-[11px] text-outline">**** {card.last4} &middot; Exp {card.exp}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-3 sm:gap-6 shrink-0">
                    <span className="font-mono font-bold text-sm">
                      {card.amountUsd != null ? `$${card.amountUsd.toFixed(2)}` : "—"}
                    </span>
                    <span className={`text-[10px] px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ${card.status === "active" ? "bg-tertiary/10 text-tertiary" : "bg-error/10 text-error"}`}>
                      {card.status}
                    </span>
                  </div>
                </m.div>
              ))}
            </div>
          )}

          {cards.length > 0 && (
            <div className="mt-8 flex justify-center">
              <div className="relative">
                <VirtualCard
                  name={cardholderName}
                  last4={cards[0].last4}
                  exp={cards[0].exp}
                  balance={
                    cards[0].amountUsd != null
                      ? `$${cards[0].amountUsd.toFixed(2)}`
                      : "—"
                  }
                  animate3d
                />
                <div className="absolute -bottom-6 -right-6">
                  <CrabMascot size="sm" mood="idle" />
                </div>
              </div>
            </div>
          )}
        </section>

        {/* Recent Deposits */}
        <section className="lg:col-span-5">
          <div className="flex justify-between items-center mb-6">
            <h3 className="font-headline font-bold text-lg">Recent Deposits</h3>
            <Link href="/deposit" className="text-primary text-sm font-medium hover:underline flex items-center gap-1">
              New <Icon name="add" className="text-sm" />
            </Link>
          </div>

          {deposits.length === 0 ? (
            <div className="bg-surface-container-lowest rounded-2xl p-8 shadow-soft-diffuse shadow-soft-diffuse text-center">
              <Icon name="account_balance_wallet" className="text-4xl text-outline mb-4" />
              <p className="text-on-surface-variant text-sm">No deposits yet.</p>
            </div>
          ) : (
            <div className="space-y-4">
              {deposits.slice(0, 5).map((dep, i) => (
                <m.div
                  key={dep.id}
                  className="bg-surface-container-lowest rounded-2xl p-5 shadow-soft-diffuse shadow-soft-diffuse hover:bg-surface-container transition-all"
                  initial={{ opacity: 0, x: 20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: 0.4 + i * 0.08 }}
                >
                  <div className="flex justify-between items-start mb-3">
                    <div className="flex items-center gap-3">
                      <div className={`w-8 h-8 rounded-lg flex items-center justify-center text-xs font-mono font-bold ${dep.asset === "XLM" ? "bg-secondary/10 text-secondary" : "bg-tertiary/10 text-tertiary"}`}>
                        {dep.asset}
                      </div>
                      <div>
                        <p className="font-mono text-sm font-medium">{dep.amount}</p>
                        <p className="text-[10px] text-outline">{new Date(dep.timestamp).toLocaleString()}</p>
                      </div>
                    </div>
                    <span className={`text-[10px] px-2 py-0.5 rounded-full font-bold uppercase tracking-wider ${dep.status === "confirmed" ? "bg-tertiary/10 text-tertiary" : "bg-amber-900/20 text-amber-400"}`}>
                      {dep.status}
                    </span>
                  </div>
                  {dep.txHash && (
                    <div className="flex justify-between text-xs text-outline">
                      <a
                        href={explorerTxUrl(dep.txHash)}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="font-mono text-primary hover:underline"
                      >
                        {dep.txHash.slice(0, 8)}...{dep.txHash.slice(-4)}
                      </a>
                      <span className="font-mono font-bold text-on-surface">{dep.usd}</span>
                    </div>
                  )}
                </m.div>
              ))}
            </div>
          )}

          {/* CLI Quick Access */}
          <div className="mt-8 bg-inverse-surface text-inverse-on-surface rounded-xl p-5 font-mono text-xs animate-fade-in-up delay-500">
            <div className="flex items-center gap-2 mb-3 text-outline">
              <Icon name="terminal" className="text-sm" />
              <span className="text-[10px] uppercase tracking-widest">Quick Terminal</span>
            </div>
            <p className="text-green-400">$ stellar-card balance</p>
            <p className="text-primary mt-1">{`{"ok":true,"data":{"balance":"...","currency":"xlm"}}`}</p>
          </div>
        </section>
      </div>
    </LazyMotion>
  );
}
