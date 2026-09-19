"use client";

import { useCallback, useEffect, useState } from "react";
import { LazyMotion, domAnimation, m, AnimatePresence } from "framer-motion";
import { contract } from "@stellar/stellar-sdk";
import { Icon } from "@/components/Icon";
import { CrabMascot } from "@/components/CrabMascot";
import { useWallet } from "@/contexts/WalletContext";
import { useLocalStorage } from "@/hooks/useStellar";
import {
  DEFAULT_FIXED_FEE_CENTS,
  FEE_VAULT_CONTRACT_ID,
  FEE_VAULT_CONTRACT_ID_PLACEHOLDER,
  NETWORK_PASSPHRASE,
  SOROBAN_TESTNET_RPC_URL,
  STROOPS_PER_XLM,
  cardFeeCents,
  explorerContractUrl,
  explorerTxUrl,
  freezeCardViaApi,
  generateFeeReference,
  getCard,
  groupCardNumber,
  isFeeVaultConfigured,
  isValidEmail,
  issueCardViaApi,
  listCards,
  usdCentsToStroops,
  type CardSummary,
  type FeeVaultContract,
  type IssuedCard,
} from "@/lib/stellar";

const XLM_PRICE_URL = "https://api.coinbase.com/v2/prices/XLM-USD/spot";

export default function CardRevealPage() {
  const { address, connected, connecting, connect, signTransaction } = useWallet();
  const [revealed, setRevealed] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const [cardholderName, setCardholderName] = useLocalStorage(
    "stellar-card-holder-name",
    "StellarCard Demo User"
  );
  const [cardholderEmail, setCardholderEmail] = useLocalStorage(
    "stellar-card-holder-email",
    "demo@stellar-card.dev"
  );
  const [cards, setCards] = useState<CardSummary[]>([]);
  const [activeCard, setActiveCard] = useState<IssuedCard | null>(null);
  const [loadingCards, setLoadingCards] = useState(false);
  const [freezing, setFreezing] = useState(false);
  const [amountUsd, setAmountUsd] = useState(50);
  const [xlmPrice, setXlmPrice] = useState<number | null>(null);
  const [buying, setBuying] = useState(false);
  const [purchaseError, setPurchaseError] = useState<string | null>(null);
  const [purchaseHash, setPurchaseHash] = useState<string | null>(null);

  const fetchPrice = useCallback(async () => {
    try {
      const response = await fetch(XLM_PRICE_URL);
      const payload = await response.json();
      const price = Number(payload?.data?.amount);
      if (Number.isFinite(price) && price > 0) setXlmPrice(price);
    } catch {
      // keep last known price
    }
  }, []);

  useEffect(() => {
    const timer = setTimeout(fetchPrice, 0);
    const interval = setInterval(fetchPrice, 60_000);
    return () => {
      clearTimeout(timer);
      clearInterval(interval);
    };
  }, [fetchPrice]);

  const loadCards = useCallback(async (email: string) => {
    if (!isValidEmail(email)) return;
    setLoadingCards(true);
    try {
      setCards(await listCards(email));
    } catch {
      // An empty list is the honest state; errors surface when issuing.
    } finally {
      setLoadingCards(false);
    }
  }, []);

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

  const currentSummary =
    cards.find((item) => item.id === activeCard?.id) ?? cards[0] ?? null;
  const hasCard = activeCard !== null || currentSummary !== null;
  const last4 = activeCard?.last4 ?? currentSummary?.last4 ?? "••••";
  const cardStatus = activeCard?.status ?? currentSummary?.status ?? "none";
  const cardExp = activeCard?.exp ?? currentSummary?.exp ?? "••/••";
  const cardBalance =
    activeCard !== null
      ? `$${activeCard.amountUsd.toFixed(2)}`
      : currentSummary?.amountUsd != null
        ? `$${currentSummary.amountUsd.toFixed(2)}`
        : "—";
  const feeTxHash = activeCard?.feeTxHash ?? currentSummary?.feeTxHash ?? null;

  const card = { last4 };

  const cardDetails = {
    number: activeCard?.number
      ? groupCardNumber(activeCard.number)
      : "•••• •••• •••• ••••",
    exp: cardExp,
    cvc: activeCard?.cvc ?? "•••",
    name: activeCard?.holderName ?? cardholderName,
    brand: activeCard?.brand ?? currentSummary?.brand ?? "—",
    status: cardStatus,
    balance: cardBalance,
    id: activeCard?.id ?? currentSummary?.id ?? "—",
  };

  const amountCents = Math.round(amountUsd * 100);
  const feeCents = amountCents > 0 ? cardFeeCents(amountCents) : 0;
  const fixedFeeCents = Math.min(feeCents, DEFAULT_FIXED_FEE_CENTS);
  const variableFeeCents = Math.max(feeCents - fixedFeeCents, 0);
  const feeStroops =
    xlmPrice !== null && feeCents > 0 ? usdCentsToStroops(feeCents, xlmPrice) : null;
  const feeXlm = feeStroops !== null ? Number(feeStroops) / STROOPS_PER_XLM : null;
  const amountValid = amountCents >= 500 && amountCents <= 50_000;
  const contractConfigured = isFeeVaultConfigured();

  const copyToClipboard = (text: string, field: string) => {
    navigator.clipboard.writeText(text.replace(/\s/g, ""));
    setCopied(field);
    setTimeout(() => setCopied(null), 2000);
  };

  const issueCard = async (hash: string) => {
    const card = await issueCardViaApi({
      payer: address ?? "",
      txHash: hash,
      amountUsd,
      name: cardholderName.trim() || "StellarCard Demo User",
      email: cardholderEmail.trim().toLowerCase(),
    });
    setActiveCard(card);
    setRevealed(false);
    await loadCards(cardholderEmail.trim().toLowerCase());
  };

  const buyCard = async () => {
    if (!address) {
      setPurchaseError("Connect Freighter before paying the card fee.");
      return;
    }
    if (!contractConfigured) {
      setPurchaseError(
        "Fee vault contract is not configured. Set NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID."
      );
      return;
    }
    if (!amountValid) {
      setPurchaseError("Card amount must be between $5.00 and $500.00.");
      return;
    }
    if (!isValidEmail(cardholderEmail.trim())) {
      setPurchaseError("Enter a valid cardholder email before buying a card.");
      return;
    }

    setBuying(true);
    setPurchaseError(null);
    setPurchaseHash(null);

    try {
      let price = xlmPrice;
      if (price === null) {
        const response = await fetch(XLM_PRICE_URL);
        const payload = await response.json();
        price = Number(payload?.data?.amount);
        if (!Number.isFinite(price) || price <= 0) {
          throw new Error("Could not fetch the XLM price from Coinbase");
        }
        setXlmPrice(price);
      }

      const stroops = usdCentsToStroops(cardFeeCents(amountCents), price);
      const client = await contract.Client.from<FeeVaultContract>({
        contractId: FEE_VAULT_CONTRACT_ID,
        rpcUrl: SOROBAN_TESTNET_RPC_URL,
        networkPassphrase: NETWORK_PASSPHRASE,
        publicKey: address,
        signTransaction,
      });

      const assembled = await client.collect_fee({
        payer: address,
        amount: stroops,
        fee_reference: generateFeeReference(),
      });
      const sent = await assembled.signAndSend();
      const hash = sent.sendTransactionResponse?.hash;
      if (!hash) {
        throw new Error("Transaction was sent but no hash was returned");
      }

      setPurchaseHash(hash);
      await issueCard(hash);
    } catch (error) {
      setPurchaseError(
        error instanceof Error ? error.message : "Fee payment failed"
      );
    } finally {
      setBuying(false);
    }
  };

  const retryIssuance = async () => {
    if (!purchaseHash) return;
    setBuying(true);
    setPurchaseError(null);
    try {
      await issueCard(purchaseHash);
    } catch (error) {
      setPurchaseError(
        error instanceof Error ? error.message : "Card issuance failed"
      );
    } finally {
      setBuying(false);
    }
  };

  const revealCard = async () => {
    if (activeCard?.number) {
      setRevealed(true);
      return;
    }
    const cardId = activeCard?.id ?? currentSummary?.id;
    if (!cardId) {
      setPurchaseError("Buy a card first — there is no issued card to reveal.");
      return;
    }
    try {
      const detail = await getCard(cardId, cardholderEmail.trim().toLowerCase());
      setActiveCard(detail);
      setRevealed(true);
    } catch (error) {
      setPurchaseError(
        error instanceof Error ? error.message : "Could not load card details"
      );
    }
  };

  const freezeActiveCard = async () => {
    const cardId = activeCard?.id ?? currentSummary?.id;
    if (!cardId) {
      setPurchaseError("No card to freeze.");
      return;
    }
    setFreezing(true);
    setPurchaseError(null);
    try {
      const frozen = await freezeCardViaApi(
        cardId,
        cardholderEmail.trim().toLowerCase()
      );
      setActiveCard(frozen);
      await loadCards(cardholderEmail.trim().toLowerCase());
    } catch (error) {
      setPurchaseError(
        error instanceof Error ? error.message : "Could not freeze the card"
      );
    } finally {
      setFreezing(false);
    }
  };

  return (
    <LazyMotion features={domAnimation} strict>
      <div className="pt-8 px-4 sm:px-8 pb-12 max-w-6xl mx-auto bg-radial-glow min-h-screen">
        <header className="mb-8 animate-fade-in-up">
          <h1 className="text-4xl font-headline font-bold tracking-tight text-on-surface">Card Issuance</h1>
          <p className="text-on-surface-variant mt-2 max-w-2xl">Deploy high-performance virtual cards instantly. Securely reveal credentials with end-to-end encryption.</p>
        </header>

        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
          {/* Card Reveal Stage */}
          <section className="lg:col-span-7 space-y-6">
            <div className="relative w-full max-w-md mx-auto h-[230px] sm:h-[280px] perspective-1000">
              <m.div
                className="relative w-full h-full preserve-3d cursor-pointer"
                animate={{ rotateY: revealed ? 180 : 0 }}
                transition={{ duration: 0.8, ease: [0.22, 1, 0.36, 1] as const }}
                onClick={() => {
                  if (!revealed) void revealCard();
                }}
              >
                {/* Front */}
                <div className="absolute inset-0 backface-hidden">
                  <div className="w-full h-full rounded-xl bg-primary text-on-primary p-4 sm:p-6 text-white overflow-hidden shadow-xl">
                    <div className="absolute inset-0 bg-gradient-to-tr from-white/0 via-white/10 to-white/0 pointer-events-none" />
                    <div className="flex justify-between items-start mb-4 sm:mb-8">
                      <span className="font-headline text-xl font-bold italic opacity-80">VISA</span>
                      <div className="flex items-center gap-1.5">
                        <span
                          className={`w-2 h-2 rounded-full ${
                            cardStatus === "active"
                              ? "bg-green-400 animate-pulse"
                              : cardStatus === "inactive"
                                ? "bg-amber-400"
                                : "bg-slate-400"
                          }`}
                        />
                        <span className="text-[10px] uppercase tracking-widest opacity-70">
                          {cardStatus === "active"
                            ? "Active"
                            : cardStatus === "inactive"
                              ? "Frozen"
                              : "No card"}
                        </span>
                      </div>
                    </div>
                    <div className="w-10 h-7 rounded-md bg-amber-300/80 mb-4 sm:mb-8 flex items-center justify-center">
                      <div className="w-6 h-4 rounded-sm border border-amber-500/40" />
                    </div>
                    <div className="font-mono text-base sm:text-lg tracking-[0.15em] mb-4 opacity-90">**** **** **** {card.last4}</div>
                    <div className="flex justify-between items-end">
                      <div><p className="text-[10px] uppercase tracking-widest opacity-60">Card Holder</p><p className="font-headline text-sm font-semibold">{cardDetails.name}</p></div>
                      <div className="text-right"><p className="text-[10px] uppercase tracking-widest opacity-60">Expires</p><p className="font-mono text-sm">{cardDetails.exp}</p></div>
                    </div>
                    {!revealed && (
                      <div className="absolute inset-0 bg-black/30 backdrop-blur-[2px] flex items-center justify-center rounded-xl">
                        <div className="text-center"><Icon name="visibility" className="text-4xl mb-2 opacity-80" /><p className="text-sm font-semibold opacity-90">Click to Reveal</p></div>
                      </div>
                    )}
                  </div>
                </div>
                {/* Back */}
                <div className="absolute inset-0 backface-hidden" style={{ transform: "rotateY(180deg)" }}>
                  <div className="w-full h-full rounded-xl bg-gradient-to-br from-slate-800 to-slate-950 p-4 sm:p-6 text-white shadow-xl">
                    <div className="w-full h-10 bg-slate-700 -mx-4 -mt-4 mb-4 px-4 sm:-mx-6 sm:-mt-6 sm:mb-6 sm:px-6" />
                    <div className="bg-white/10 rounded-lg p-4 mb-4">
                      <p className="text-[10px] uppercase tracking-widest text-slate-400 mb-1">Card Number</p>
                      <p className="font-mono text-base tracking-wider">{cardDetails.number}</p>
                    </div>
                    <div className="flex gap-4">
                      <div className="flex-1 bg-white/10 rounded-lg p-3"><p className="text-[10px] uppercase tracking-widest text-slate-400 mb-1">Expiry</p><p className="font-mono text-sm">{cardDetails.exp}</p></div>
                      <div className="flex-1 bg-white/10 rounded-lg p-3"><p className="text-[10px] uppercase tracking-widest text-slate-400 mb-1">CVC</p><p className="font-mono text-sm">{cardDetails.cvc}</p></div>
                    </div>
                    <div className="mt-4 hidden sm:flex items-center gap-2 text-[10px] text-slate-500">
                      <Icon name="lock" className="text-xs" /><span>256-bit AES encrypted &middot; PCI-DSS Level 1</span>
                    </div>
                  </div>
                </div>
              </m.div>

              <AnimatePresence>
                {revealed && (
                  <m.div className="absolute -right-12 -top-12 z-10" initial={{ y: 80, opacity: 0, scale: 0.5 }} animate={{ y: 0, opacity: 1, scale: 1 }} transition={{ delay: 0.4, duration: 0.5, type: "spring", stiffness: 200 }}>
                    <CrabMascot size="md" mood="peek" />
                    <div className="absolute -left-4 top-2 w-6 h-4 bg-primary text-on-primary rounded-[2px] shadow-md animate-card-float" />
                  </m.div>
                )}
              </AnimatePresence>
            </div>

            {!revealed && (
              <button
                onClick={revealCard}
                disabled={!hasCard}
                className="w-full max-w-md mx-auto block py-4 bg-primary text-on-primary rounded-xl font-bold text-sm shadow-lg shadow-primary-container/20 hover:shadow-primary-container/40 transition-all active:scale-[0.98] disabled:opacity-60"
              >
                <Icon name="visibility" className="mr-2 align-middle" />
                {hasCard ? "Reveal Card Details" : "Buy a Card to Reveal Real Details"}
              </button>
            )}

            {/* Buy a Card */}
            <div className="bg-surface-container-lowest rounded-xl p-6 shadow-soft-diffuse">
              <div className="flex items-center justify-between mb-2">
                <h3 className="font-headline font-bold text-lg">Buy a Card</h3>
                <span className="text-[10px] font-mono text-outline uppercase tracking-widest">Stellar Testnet</span>
              </div>
              <p className="text-xs text-on-surface-variant mb-5">
                The $0.10 + 0.20% fee is collected on-chain by the Soroban fee vault.
                A Cloudflare edge function then verifies that transaction on Horizon and
                issues a real Stripe test-mode virtual card. Stripe secrets stay in the
                edge function; the browser only receives the card it paid for.
              </p>

              <label className="block text-[10px] uppercase tracking-widest text-outline font-bold mb-2">
                Cardholder Name
              </label>
              <input
                type="text"
                value={cardholderName}
                onChange={(event) => setCardholderName(event.target.value)}
                className="w-full bg-surface-container rounded-xl px-4 py-3 mb-5 text-sm outline-none"
              />

              <label className="block text-[10px] uppercase tracking-widest text-outline font-bold mb-2">
                Cardholder Email
              </label>
              <input
                type="email"
                value={cardholderEmail}
                onChange={(event) => setCardholderEmail(event.target.value)}
                className="w-full bg-surface-container rounded-xl px-4 py-3 mb-5 text-sm outline-none"
              />

              {!contractConfigured && (
                <div className="mb-5 flex items-start gap-2 rounded-xl bg-error-container/40 p-4 text-xs text-error">
                  <Icon name="error" className="text-sm mt-0.5" />
                  <div>
                    <p className="font-bold">Fee vault contract is not configured.</p>
                    <p className="mt-1 font-mono break-all">
                      Set NEXT_PUBLIC_FEE_VAULT_CONTRACT_ID (currently {FEE_VAULT_CONTRACT_ID || FEE_VAULT_CONTRACT_ID_PLACEHOLDER}).
                    </p>
                  </div>
                </div>
              )}

              <label className="block text-[10px] uppercase tracking-widest text-outline font-bold mb-2">
                Card Amount (USD)
              </label>
              <div className="flex items-center gap-2 bg-surface-container rounded-xl px-4 py-3 mb-5">
                <span className="text-on-surface-variant font-mono">$</span>
                <input
                  type="number"
                  min={5}
                  max={500}
                  step={1}
                  value={amountUsd}
                  onChange={(event) => setAmountUsd(Number(event.target.value))}
                  className="flex-1 bg-transparent outline-none font-mono text-sm"
                />
                {!amountValid && amountCents > 0 && (
                  <span className="text-[10px] text-error">$5 – $500</span>
                )}
              </div>

              <div className="bg-surface-container-low rounded-xl p-4 mb-5 text-left">
                <div className="space-y-1 font-mono text-sm">
                  <div className="flex justify-between"><span className="text-on-surface-variant">Fixed fee</span><span>${(fixedFeeCents / 100).toFixed(2)}</span></div>
                  <div className="flex justify-between"><span className="text-on-surface-variant">Variable (20 bps)</span><span>${(variableFeeCents / 100).toFixed(2)}</span></div>
                  <div className="flex justify-between border-t border-outline-variant/30 pt-1 mt-1 font-bold"><span>Total fee</span><span className="text-primary">${(feeCents / 100).toFixed(2)}</span></div>
                  <div className="flex justify-between text-xs text-on-surface-variant pt-1">
                    <span>Pay in XLM</span>
                    <span>{feeXlm !== null ? `${feeXlm.toFixed(7)} XLM` : "Fetching price..."}</span>
                  </div>
                </div>
              </div>

              {purchaseHash && (
                <div className="mb-5 flex items-start gap-2 rounded-xl bg-tertiary/10 p-4 text-xs">
                  <Icon name="check_circle" className="text-sm text-tertiary mt-0.5" filled />
                  <div>
                    <p className="font-bold text-tertiary">Fee collected on Stellar testnet</p>
                    <a
                      href={explorerTxUrl(purchaseHash)}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="font-mono text-primary hover:underline break-all"
                    >
                      {purchaseHash}
                    </a>
                  </div>
                </div>
              )}

              {purchaseHash && !activeCard && (
                <button
                  onClick={retryIssuance}
                  disabled={buying}
                  className="mb-5 w-full py-3 bg-tertiary/10 text-tertiary rounded-xl font-bold text-xs disabled:opacity-60"
                >
                  Fee is on-chain — retry card issuance
                </button>
              )}

              {purchaseError && (
                <div className="mb-5 flex items-start gap-2 rounded-xl bg-error-container/40 p-4 text-xs text-error">
                  <Icon name="error" className="text-sm mt-0.5" />
                  <span className="break-all">{purchaseError}</span>
                </div>
              )}

              <button
                onClick={connected ? buyCard : connect}
                disabled={buying || connecting || (connected && (!contractConfigured || !amountValid))}
                className="w-full py-4 bg-primary text-on-primary rounded-xl font-bold text-sm shadow-lg shadow-primary-container/20 hover:shadow-primary-container/40 transition-all active:scale-[0.98] disabled:opacity-60"
              >
                {buying
                  ? "Collecting fee..."
                  : connecting
                    ? "Connecting..."
                    : connected
                      ? `Pay $${(feeCents / 100).toFixed(2)} Fee with Freighter`
                      : "Connect Freighter to Buy"}
              </button>
              {contractConfigured && (
                <a
                  href={explorerContractUrl(FEE_VAULT_CONTRACT_ID)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-3 block text-center text-[10px] font-mono text-outline hover:text-primary truncate"
                >
                  Fee vault: {FEE_VAULT_CONTRACT_ID}
                </a>
              )}
            </div>
          </section>

          {/* Card Info Panel */}
          <section className="lg:col-span-5 space-y-6">
            <div className="bg-surface-container-lowest rounded-xl p-6 shadow-soft-diffuse shadow-soft-diffuse animate-slide-in-right delay-200">
              <div className="flex items-center justify-between mb-6">
                <h3 className="font-headline font-bold text-lg">Card Details</h3>
                <span className="bg-tertiary/10 text-tertiary text-[10px] px-3 py-1 rounded-full font-bold uppercase tracking-wider">{cardDetails.status}</span>
              </div>
              <div className="space-y-4">
                {[{ label: "Card ID", value: cardDetails.id, mono: true }, { label: "Brand", value: cardDetails.brand }, { label: "Card Amount", value: cardDetails.balance, mono: true }, { label: "Network", value: "Stellar Testnet" }].map((row) => (
                  <div key={row.label} className="flex justify-between items-center">
                    <span className="text-xs text-outline uppercase tracking-widest">{row.label}</span>
                    <span className={`text-sm font-medium ${row.mono ? "font-mono" : ""}`}>{row.value}</span>
                  </div>
                ))}
              </div>
              {feeTxHash && (
                <a
                  href={explorerTxUrl(feeTxHash)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="mt-4 block text-[10px] font-mono text-outline hover:text-primary truncate"
                >
                  Fee tx: {feeTxHash}
                </a>
              )}
              {loadingCards && (
                <p className="mt-4 text-[10px] text-outline">Loading issued cards…</p>
              )}
              {cards.length > 0 && (
                <div className="mt-6 border-t border-outline-variant/30 pt-4">
                  <p className="text-[10px] uppercase tracking-widest text-outline font-bold mb-3">
                    Cards issued to {cardholderEmail} ({cards.length})
                  </p>
                  <div className="space-y-2">
                    {cards.map((item) => {
                      const selected =
                        activeCard?.id === item.id ||
                        (activeCard === null && currentSummary?.id === item.id);
                      return (
                        <button
                          key={item.id}
                          onClick={() => {
                            setActiveCard(null);
                            setRevealed(false);
                            void getCard(item.id, cardholderEmail.trim().toLowerCase())
                              .then(setActiveCard)
                              .catch(() => undefined);
                          }}
                          className={`w-full flex items-center justify-between p-3 rounded-xl text-left text-xs transition-colors ${
                            selected
                              ? "bg-primary/10"
                              : "bg-surface-container hover:bg-surface-container-high"
                          }`}
                        >
                          <span className="font-mono">•••• {item.last4}</span>
                          <span className="text-outline">
                            {item.status} · {item.exp}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>

            <AnimatePresence>
              {revealed && (
                <m.div className="bg-surface-container-lowest rounded-xl p-6 shadow-soft-diffuse shadow-soft-diffuse space-y-3" initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.6 }}>
                  <h3 className="font-headline font-bold text-lg mb-4">Quick Copy</h3>
                  {[{ label: "Card Number", value: cardDetails.number, field: "number" }, { label: "Expiry", value: cardDetails.exp, field: "exp" }, { label: "CVC", value: cardDetails.cvc, field: "cvc" }].map((item) => (
                    <button key={item.field} onClick={() => copyToClipboard(item.value, item.field)} className="w-full flex items-center justify-between p-3 bg-surface-container rounded-xl hover:bg-surface-container-high transition-all group">
                      <div className="text-left">
                        <p className="text-[10px] text-outline uppercase tracking-widest">{item.label}</p>
                        <p className="font-mono text-sm font-medium">{item.value}</p>
                      </div>
                      <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center group-hover:bg-primary/20 transition-colors">
                        <Icon name={copied === item.field ? "check" : "content_copy"} className={`text-sm ${copied === item.field ? "text-tertiary" : "text-primary"}`} />
                      </div>
                    </button>
                  ))}
                </m.div>
              )}
            </AnimatePresence>

            <div className="bg-surface-container-lowest rounded-xl p-6 shadow-soft-diffuse shadow-soft-diffuse animate-slide-in-right delay-300">
              <h3 className="font-headline font-bold text-lg mb-4">Actions</h3>
              <div className="space-y-3">
                <button
                  onClick={freezeActiveCard}
                  disabled={freezing || !hasCard || cardStatus === "inactive"}
                  className="w-full flex items-center gap-3 p-3 bg-surface-container rounded-xl hover:bg-surface-container-high transition-all text-sm disabled:opacity-60"
                >
                  <Icon name="ac_unit" className="text-primary" />
                  <span className="font-medium">
                    {freezing
                      ? "Freezing..."
                      : cardStatus === "inactive"
                        ? "Card Frozen"
                        : "Freeze Card"}
                  </span>
                </button>
                {feeTxHash ? (
                  <a
                    href={explorerTxUrl(feeTxHash)}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="w-full flex items-center gap-3 p-3 bg-surface-container rounded-xl hover:bg-surface-container-high transition-all text-sm"
                  >
                    <Icon name="history" className="text-secondary" />
                    <span className="font-medium">On-chain Fee Transaction</span>
                  </a>
                ) : (
                  <div className="w-full flex items-center gap-3 p-3 bg-surface-container rounded-xl text-sm opacity-60">
                    <Icon name="history" className="text-secondary" />
                    <span className="font-medium">No fee transaction yet</span>
                  </div>
                )}
              </div>
            </div>

            <div className="bg-inverse-surface text-inverse-on-surface rounded-xl p-5 font-mono text-xs animate-fade-in-up delay-500">
              <div className="flex items-center gap-2 mb-3 text-outline">
                <Icon name="terminal" className="text-sm" />
                <span className="text-[10px] uppercase tracking-widest">CLI Equivalent</span>
              </div>
              <p className="text-green-400">$ stellar-card card show {cardDetails.id}</p>
              <p className="text-primary mt-1">{`{"ok":true,"data":{"last4":"${card.last4}","exp":"${cardDetails.exp}","status":"${cardDetails.status}"}}`}</p>
            </div>
          </section>
        </div>
      </div>
    </LazyMotion>
  );
}
