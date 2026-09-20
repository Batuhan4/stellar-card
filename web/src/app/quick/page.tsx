"use client";

import { useState } from "react";
import Link from "next/link";
import { Icon } from "@/components/Icon";
import { CrabMascot } from "@/components/CrabMascot";
import { useLocalStorage } from "@/hooks/useStellar";
import {
  groupCardNumber,
  isValidEmail,
  quickCardViaApi,
  type IssuedCard,
} from "@/lib/stellar";

type Step = "amount" | "iban" | "verifying" | "card";

const TRY_PER_USD = 50;
const PRESETS = [
  { usd: 5, try: 250 },
  { usd: 10, try: 500 },
  { usd: 25, try: 1250 },
  { usd: 50, try: 2500 },
];

const DEMO_IBAN = "TR00 0000 0000 0000 0000 0000 00";
const STAGES = [
  "TRY transfer received (demo)",
  "TRY → USDC conversion (demo)",
  "Issuing Stripe test-mode card",
];

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export default function QuickBuyPage() {
  const [step, setStep] = useState<Step>("amount");
  const [preset, setPreset] = useState(PRESETS[1]);
  const [cardholderName, setCardholderName] = useLocalStorage(
    "stellar-card-holder-name",
    "StellarCard Demo User"
  );
  const [cardholderEmail, setCardholderEmail] = useLocalStorage(
    "stellar-card-holder-email",
    "demo@stellar-card.dev"
  );
  const [reference] = useState(
    () => `SC-${Math.random().toString(36).slice(2, 8).toUpperCase()}`
  );
  const [stage, setStage] = useState(0);
  const [card, setCard] = useState<IssuedCard | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

  const emailOk = isValidEmail(cardholderEmail.trim());

  const copy = (value: string, field: string) => {
    navigator.clipboard.writeText(value.replace(/\s/g, ""));
    setCopied(field);
    setTimeout(() => setCopied(null), 1800);
  };

  const startTransfer = async () => {
    setStep("verifying");
    setStage(0);
    setError(null);
    await sleep(1300);
    setStage(1);
    await sleep(1300);
    setStage(2);
    try {
      const issued = await quickCardViaApi({
        amountUsd: preset.usd,
        name: cardholderName.trim() || "StellarCard Demo User",
        email: cardholderEmail.trim().toLowerCase(),
        reference,
      });
      setCard(issued);
      setStep("card");
    } catch (issueError) {
      setError(
        issueError instanceof Error ? issueError.message : "Card issuance failed"
      );
      setStep("iban");
    }
  };

  return (
    <div className="pt-20 md:pt-8 px-4 sm:px-8 pb-12 max-w-3xl mx-auto">
      <header className="mb-8 animate-fade-in-up">
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-3xl sm:text-4xl font-headline font-bold tracking-tight text-on-surface">
            Quick Buy
          </h1>
          <span className="text-[10px] font-bold uppercase tracking-widest bg-amber-500/15 text-amber-700 px-3 py-1 rounded-full">
            TRY demo
          </span>
        </div>
        <p className="text-on-surface-variant mt-2 max-w-xl">
          Buy a virtual card in three taps: send TRY to the IBAN, and the card is
          issued. No wallet, no Stellar knowledge required.
        </p>
        <div className="mt-4 flex items-start gap-2 rounded-xl bg-amber-50 border border-amber-200 p-4 text-xs text-amber-800">
          <Icon name="info" className="text-sm mt-0.5 shrink-0" />
          <span>
            Demo preview: no live TRY-capable Stellar anchor exists yet, so the
            TRY transfer and conversion steps are simulated. The virtual card
            you receive is a real Stripe <strong>test-mode</strong> card and
            appears in your card list. The standard on-chain flow is unchanged.
          </span>
        </div>
      </header>

      {error && (
        <div className="mb-6 rounded-xl bg-error-container/40 p-4 text-xs text-error">
          {error}
        </div>
      )}

      {step === "amount" && (
        <section className="bg-surface-container-lowest rounded-2xl p-6 sm:p-8 shadow-soft-diffuse animate-fade-in-up">
          <p className="text-[10px] uppercase tracking-widest text-outline font-bold mb-4">
            Step 1 — Choose the card amount
          </p>
          <div className="grid grid-cols-2 gap-3 mb-8">
            {PRESETS.map((option) => (
              <button
                key={option.usd}
                onClick={() => setPreset(option)}
                className={`rounded-xl p-4 text-left transition-all ${
                  preset.usd === option.usd
                    ? "bg-primary/10 ring-2 ring-primary"
                    : "bg-surface-container hover:bg-surface-container-high"
                }`}
              >
                <p className="font-headline font-bold text-lg">
                  ₺{option.try.toLocaleString("tr-TR")}
                </p>
                <p className="text-xs text-outline font-mono">
                  = ${option.usd.toFixed(2)} card
                </p>
              </button>
            ))}
          </div>
          <p className="text-[11px] text-outline mb-5">
            Demo rate: 1 USD = {TRY_PER_USD} TRY.
          </p>

          <label className="block text-[10px] uppercase tracking-widest text-outline font-bold mb-2">
            Cardholder name
          </label>
          <input
            type="text"
            value={cardholderName}
            onChange={(event) => setCardholderName(event.target.value)}
            className="w-full bg-surface-container rounded-xl px-4 py-3 mb-4 text-sm outline-none"
          />
          <label className="block text-[10px] uppercase tracking-widest text-outline font-bold mb-2">
            Cardholder email
          </label>
          <input
            type="email"
            value={cardholderEmail}
            onChange={(event) => setCardholderEmail(event.target.value)}
            className="w-full bg-surface-container rounded-xl px-4 py-3 mb-6 text-sm outline-none"
          />
          <button
            onClick={() => setStep("iban")}
            disabled={!emailOk}
            className="w-full py-4 bg-primary text-on-primary rounded-xl font-bold text-sm disabled:opacity-60"
          >
            Continue — show IBAN
          </button>
        </section>
      )}

      {step === "iban" && (
        <section className="bg-surface-container-lowest rounded-2xl p-6 shadow-soft-diffuse animate-fade-in-up">
          <p className="text-[10px] uppercase tracking-widest text-outline font-bold mb-3">
            Step 2 — Send ₺{preset.try.toLocaleString("tr-TR")} to the IBAN
          </p>
          <div className="rounded-xl bg-surface-container p-4 space-y-4">
            <div>
              <p className="text-[10px] uppercase tracking-widest text-outline font-bold mb-1">
                IBAN (demo)
              </p>
              <div className="flex items-center gap-3">
                <span className="font-mono text-sm flex-1 break-all">
                  {DEMO_IBAN}
                </span>
                <button
                  onClick={() => copy(DEMO_IBAN, "iban")}
                  className="shrink-0 w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center"
                >
                  <Icon
                    name={copied === "iban" ? "check" : "content_copy"}
                    className={`text-sm ${copied === "iban" ? "text-tertiary" : "text-primary"}`}
                  />
                </button>
              </div>
            </div>
            <div className="flex justify-between items-center">
              <div>
                <p className="text-[10px] uppercase tracking-widest text-outline font-bold mb-1">
                  Reference
                </p>
                <span className="font-mono text-sm">{reference}</span>
              </div>
              <button
                onClick={() => copy(reference, "ref")}
                className="w-9 h-9 rounded-lg bg-primary/10 flex items-center justify-center"
              >
                <Icon
                  name={copied === "ref" ? "check" : "content_copy"}
                  className={`text-sm ${copied === "ref" ? "text-tertiary" : "text-primary"}`}
                />
              </button>
            </div>
            <div className="flex justify-between text-xs text-outline">
              <span>Account holder</span>
              <span className="font-medium text-on-surface">
                StellarCard Demo
              </span>
            </div>
          </div>
          <p className="text-[11px] text-amber-700 mt-3">
            This IBAN is a placeholder: no live TRY anchor exists on Stellar yet.
          </p>
          <button
            onClick={startTransfer}
            className="w-full mt-6 py-4 bg-primary text-on-primary rounded-xl font-bold text-sm flex items-center justify-center gap-2"
          >
            <Icon name="send" className="text-[18px]" />
            I&apos;ve sent the TRY
          </button>
          <button
            onClick={() => setStep("amount")}
            className="w-full mt-3 py-3 text-xs text-outline"
          >
            Back
          </button>
        </section>
      )}

      {step === "verifying" && (
        <section className="bg-surface-container-lowest rounded-2xl p-6 shadow-soft-diffuse text-center animate-fade-in-up">
          <div className="relative w-24 h-24 mx-auto mb-6">
            <div className="absolute inset-0 rounded-full border-4 border-surface-container-highest" />
            <div className="absolute inset-0 rounded-full border-4 border-transparent border-t-primary animate-spin-slow" />
            <div className="absolute inset-4 rounded-full bg-surface-container flex items-center justify-center">
              <Icon name="hourglass_empty" className="text-primary text-2xl" />
            </div>
          </div>
          <h2 className="font-headline font-bold text-xl mb-6">
            Processing your payment
          </h2>
          <div className="space-y-3 text-left max-w-sm mx-auto">
            {STAGES.map((label, index) => (
              <div key={label} className="flex items-center gap-3">
                <div
                  className={`w-6 h-6 rounded-full flex items-center justify-center text-xs ${
                    index < stage
                      ? "bg-tertiary text-on-tertiary"
                      : index === stage
                        ? "bg-primary text-on-primary animate-pulse"
                        : "bg-surface-container-highest text-outline"
                  }`}
                >
                  {index < stage ? (
                    <Icon name="check" className="text-xs" />
                  ) : (
                    index + 1
                  )}
                </div>
                <span className="text-sm text-on-surface-variant">{label}</span>
              </div>
            ))}
          </div>
        </section>
      )}

      {step === "card" && card && (
        <section className="animate-fade-in-up">
          <div className="bg-surface-container-lowest rounded-2xl p-6 shadow-soft-diffuse text-center">
            <p className="text-[10px] uppercase tracking-widest text-outline font-bold mb-4">
              Step 3 — Your card is ready
            </p>
            <div className="max-w-sm mx-auto rounded-2xl bg-gradient-to-br from-slate-900 to-slate-800 p-6 text-left text-white relative overflow-hidden">
              <div className="flex justify-between items-start mb-6">
                <span className="font-headline text-lg font-bold italic opacity-80">
                  VISA
                </span>
                <span className="text-[10px] uppercase tracking-widest bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded-full font-bold">
                  demo · test mode
                </span>
              </div>
              <p className="font-mono text-base sm:text-lg tracking-[0.15em] mb-4">
                {card.number
                  ? groupCardNumber(card.number)
                  : `•••• •••• •••• ${card.last4}`}
              </p>
              <div className="flex justify-between items-end">
                <div>
                  <p className="text-[10px] uppercase tracking-widest opacity-60 mb-1">
                    Card holder
                  </p>
                  <p className="font-headline text-sm font-semibold">
                    {card.holderName}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-[10px] uppercase tracking-widest opacity-60 mb-1">
                    Expires
                  </p>
                  <p className="font-mono text-sm">{card.exp}</p>
                </div>
                <div className="text-right">
                  <p className="text-[10px] uppercase tracking-widest opacity-60 mb-1">
                    CVC
                  </p>
                  <p className="font-mono text-sm">{card.cvc ?? "•••"}</p>
                </div>
              </div>
            </div>
            <div className="mt-6 flex flex-col sm:flex-row gap-3 justify-center">
              <Link
                href="/cards"
                className="px-6 py-3 bg-primary text-on-primary rounded-xl font-bold text-sm"
              >
                Manage in Cards
              </Link>
              <button
                onClick={() => {
                  setCard(null);
                  setStep("amount");
                }}
                className="px-6 py-3 bg-surface-container rounded-xl font-bold text-sm"
              >
                Buy another
              </button>
            </div>
          </div>
          <div className="mt-6 flex justify-center">
            <CrabMascot size="md" mood="cheer" />
          </div>
        </section>
      )}
    </div>
  );
}
