"use client";

import { LazyMotion, domAnimation } from "framer-motion";
import { Navbar } from "@/components/Navbar";
import { MobileBottomNav } from "@/components/MobileNav";
import { CrabMascot } from "@/components/CrabMascot";
import { Icon } from "@/components/Icon";
import { useWallet } from "@/contexts/WalletContext";
import Link from "next/link";

export default function LandingPage() {
  const { connected, connecting, installed, error, networkOk, connect } =
    useWallet();

  return (
    <LazyMotion features={domAnimation} strict>
      <Navbar />
      <main className="pt-24">
        {/* Hero */}
        <section className="relative min-h-[800px] xl:min-h-[900px] flex items-center px-6 sm:px-8 xl:px-16 overflow-hidden">
          <div className="absolute inset-0 pointer-events-none">
            <div className="absolute -top-16 -left-16 w-[600px] h-[600px] bg-primary/10 rounded-full blur-[120px]" />
            <div className="absolute top-1/3 -right-24 w-[400px] h-[400px] bg-secondary/8 rounded-full blur-[100px]" />
            <div className="absolute bottom-0 left-1/3 w-[300px] h-[300px] bg-tertiary/8 rounded-full blur-[80px]" />
          </div>

          <div className="grid min-w-0 lg:grid-cols-2 gap-12 xl:gap-20 items-center w-full max-w-[1600px] mx-auto relative z-10">
            <div className="space-y-6 sm:space-y-8 animate-fade-in-up min-w-0">
              <div className="inline-flex items-center gap-2 bg-surface-container-low px-4 py-1.5 rounded-full transition-all duration-300">
                <span className="w-2 h-2 rounded-full bg-tertiary-container animate-pulse" />
                <span className="text-[0.65rem] font-headline font-bold uppercase tracking-[0.2em] text-on-surface-variant">
                  Stellar Testnet Live
                </span>
              </div>

              <h1 className="text-[2.5rem] sm:text-6xl xl:text-7xl 2xl:text-[6rem] font-headline font-bold leading-[0.9] tracking-tight text-on-surface">
                Virtual cards for{" "}
                <span className="text-primary">humans</span> and developers
              </h1>

              <p className="text-base sm:text-xl xl:text-2xl text-on-surface-variant max-w-lg leading-relaxed">
                Buy a card in three taps. No crypto wallet needed — Stellar settles
                the payment under the hood.
              </p>

              <div className="flex flex-wrap gap-3 pt-4">
                <Link
                  href="/quick"
                  className="px-6 sm:px-8 py-4 bg-primary text-on-primary rounded-xl font-bold flex items-center gap-3 shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all"
                >
                  <Icon name="bolt" className="text-[18px]" filled />
                  Quick Buy with TRY
                  <span className="text-[10px] font-bold uppercase tracking-widest bg-amber-400/90 text-amber-900 px-2 py-0.5 rounded-full">
                    demo
                  </span>
                </Link>
                {connected ? (
                  <Link
                    href="/dashboard"
                    className="px-6 sm:px-8 py-4 bg-surface-container-highest text-on-surface rounded-xl font-bold flex items-center gap-3 hover:bg-surface-variant transition-all"
                  >
                    Go to Dashboard
                    <Icon name="arrow_forward" className="text-[18px]" />
                  </Link>
                ) : installed === false ? (
                  <a
                    href="https://www.freighter.app/"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="px-6 sm:px-8 py-4 bg-surface-container-highest text-on-surface rounded-xl font-bold flex items-center gap-3 hover:bg-surface-variant transition-all"
                  >
                    <Icon name="desktop_windows" className="text-[18px]" />
                    <span className="sm:hidden">Desktop wallet only</span>
                    <span className="hidden sm:inline">Install Freighter</span>
                  </a>
                ) : (
                  <button
                    onClick={connect}
                    disabled={connecting}
                    className="px-6 sm:px-8 py-4 bg-surface-container-highest text-on-surface rounded-xl font-bold flex items-center gap-3 hover:bg-surface-variant transition-all disabled:opacity-60"
                  >
                    <Icon name="account_balance_wallet" className="text-[18px]" />
                    {connecting ? "Connecting..." : "Connect Freighter"}
                  </button>
                )}
              </div>
              {connected && networkOk === false && (
                <p className="text-xs text-error max-w-md">
                  Freighter is on a different network. Open Freighter → Settings →
                  Network → Testnet, then reconnect.
                </p>
              )}
              {error && !connected && (
                <p className="text-xs text-error max-w-md">{error}</p>
              )}
            </div>

            {/* Right: Visual — Stitch 3D hero with crab */}
            <div className="relative h-[400px] sm:h-[600px] xl:h-[700px] 2xl:h-[800px] w-full flex items-center justify-center scene-3d">
              <div className="animate-float-3d relative w-[320px] h-[200px] sm:w-[440px] sm:h-[280px] xl:w-[520px] xl:h-[340px]">
                <div className="absolute -top-12 -left-4 sm:-top-16 sm:-left-6 z-30 origin-top-left scale-75 sm:scale-90 xl:scale-100">
                  <CrabMascot size="lg" mood="idle" />
                </div>

                <div className="animate-card-rotation absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-20 origin-center scale-[0.6] sm:scale-[0.8] xl:scale-100">
                  <div className="w-[440px] h-[280px] xl:w-[520px] xl:h-[340px] bg-gradient-to-br from-slate-900 to-slate-800 rounded-[2rem] xl:rounded-[2.5rem] p-8 xl:p-10 shadow-[0_50px_100px_-20px_rgba(0,0,0,0.3)] relative overflow-hidden border border-white/10">
                    <div className="absolute -top-24 -right-24 w-64 h-64 bg-primary/20 rounded-full blur-3xl" />
                    <div className="h-full flex flex-col justify-between relative z-10">
                      <div className="flex justify-between items-start">
                        <div className="w-12 h-10 bg-gradient-to-r from-amber-400 to-amber-200/50 rounded-lg opacity-80" />
                        <div className="text-white/40 font-mono tracking-widest text-xs uppercase">StellarCard Global</div>
                      </div>
                      <div className="space-y-4">
                        <div className="text-2xl xl:text-3xl text-white font-mono tracking-[0.3em]">4242 &bull;&bull;&bull;&bull; &bull;&bull;&bull;&bull; 1085</div>
                        <div className="flex justify-between items-end">
                          <div className="space-y-1">
                            <div className="text-[10px] text-white/40 uppercase tracking-widest">Card Holder</div>
                            <div className="text-sm text-white font-mono uppercase">AI AGENT #0042</div>
                          </div>
                          <div className="text-right">
                            <div className="text-[10px] text-white/40 uppercase tracking-widest">Valid Thru</div>
                            <div className="text-sm text-white font-mono">12/28</div>
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="absolute top-1/2 left-1/2 w-[30px] h-[20px] bg-gradient-to-br from-primary to-secondary rounded-sm pointer-events-none animate-boomerang z-50" />
              </div>

              <div className="hidden sm:block absolute -top-12 -left-16 xl:-left-24 glass-panel p-5 rounded-2xl shadow-soft-diffuse border border-white/50 w-64 xl:w-72 z-30 animate-fade-in-up delay-500">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-8 h-8 rounded-full bg-tertiary-fixed flex items-center justify-center">
                    <Icon name="account_balance_wallet" className="text-on-tertiary-fixed text-[18px]" filled />
                  </div>
                  <div>
                    <div className="text-[10px] font-headline font-bold text-tertiary uppercase tracking-tighter">Status</div>
                    <div className="text-sm font-bold text-on-surface">Deposit detected</div>
                  </div>
                </div>
                <div className="text-[11px] text-on-surface-variant font-mono">+ 42.50 XLM confirmed</div>
              </div>

              <div className="hidden sm:block absolute -bottom-8 -right-16 xl:-right-24 glass-panel p-5 rounded-2xl shadow-soft-diffuse border border-white/50 w-60 xl:w-68 z-30 animate-fade-in-up delay-700">
                <div className="flex items-center gap-3 mb-2">
                  <div className="w-8 h-8 rounded-full bg-primary-fixed flex items-center justify-center">
                    <Icon name="verified" className="text-primary text-[18px]" />
                  </div>
                  <div>
                    <div className="text-[10px] font-headline font-bold text-primary uppercase tracking-tighter">Action</div>
                    <div className="text-sm font-bold text-on-surface">Card issued</div>
                  </div>
                </div>
                <div className="text-[11px] text-on-surface-variant">Ready for instant terminal use</div>
              </div>
            </div>
          </div>
        </section>

        {/* How it Works */}
        <section id="how-it-works" className="py-20 xl:py-28 bg-surface-container-low relative overflow-hidden">
          <div className="max-w-7xl mx-auto px-8 xl:px-16 relative z-10">
            <div className="flex flex-col items-center text-center mb-16 animate-fade-in-up">
              <h2 className="text-4xl md:text-5xl xl:text-6xl font-headline font-bold text-on-surface mb-6 tracking-tight">
                How it works
              </h2>
              <p className="text-on-surface-variant max-w-xl xl:text-xl text-lg">
                Three steps between you and a virtual card. No crypto knowledge needed — Stellar handles the complex part.
              </p>
            </div>

            <div className="hidden md:block absolute top-1/2 left-0 w-full h-[2px] bg-gradient-to-r from-transparent via-primary-fixed-dim to-transparent -translate-y-1/2 z-0" />

            <div className="relative grid md:grid-cols-3 gap-8 xl:gap-12">
              {[
                { icon: "send", title: "1. Send money", desc: "Transfer TRY to the IBAN we give you. Reference code ensures it's tracked automatically." },
                { icon: "autorenew", title: "2. We convert & fund", desc: "Your TRY is converted to USDC on Stellar and deposited into your card account on-chain." },
                { icon: "credit_card", title: "3. Card is issued", desc: "A virtual Visa is created instantly. Use the card number anywhere that accepts Visa — it's a real test-mode card." },
              ].map((item, i) => (
                <div
                  key={item.title}
                  className={`relative z-10 group animate-fade-in-up delay-${(i + 1) * 200}`}
                >
                  <div className="bg-surface-container-lowest p-8 xl:p-10 rounded-[2rem] shadow-soft-diffuse hover:shadow-xl transition-all duration-300 border border-transparent hover:border-primary-fixed-dim">
                    <div className="w-14 h-14 bg-primary/10 rounded-2xl flex items-center justify-center mb-6 text-primary group-hover:scale-110 transition-transform duration-300">
                      <Icon name={item.icon} className="text-2xl" filled />
                    </div>
                    <h3 className="text-xl xl:text-2xl font-headline font-bold mb-3">{item.title}</h3>
                    <p className="text-on-surface-variant leading-relaxed text-sm xl:text-base">{item.desc}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Agent Pitch */}
        <section id="security" className="py-20 xl:py-28 px-8 xl:px-16 max-w-[1600px] mx-auto">
          <div className="grid min-w-0 lg:grid-cols-2 gap-10 xl:gap-20 items-center">
            <div className="animate-fade-in-up">
              <h2 className="text-4xl md:text-5xl xl:text-6xl font-headline font-bold text-on-surface mb-8 tracking-tight">
                Built for the <br /><span className="text-secondary">Real World</span>
              </h2>
              <div className="space-y-5">
                {["Works without a crypto wallet — just an IBAN transfer", "Real virtual Visa card, usable at any merchant", "Instant issuance — card ready in seconds", "Transparent $0.10 + 0.20% fee, visible on-chain", "Freeze or unfreeze the card anytime, instantly"].map((feat) => (
                  <div key={feat} className="flex items-start gap-4">
                    <div className="w-6 h-6 rounded-full bg-secondary-fixed flex items-center justify-center mt-0.5 shrink-0">
                      <Icon name="check" className="text-secondary text-[14px]" filled />
                    </div>
                    <p className="text-on-surface-variant text-sm xl:text-base leading-relaxed">{feat}</p>
                  </div>
                ))}
              </div>
            </div>

            <div className="min-w-0 overflow-hidden bg-white rounded-2xl p-6 xl:p-8 shadow-soft-diffuse border border-outline-variant animate-fade-in-up delay-200">
              <div className="flex items-center gap-2 mb-5">
                <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center">
                  <Icon name="credit_card" className="text-primary text-xl" filled />
                </div>
                <div>
                  <div className="font-headline font-bold text-sm text-on-surface">StellarCard</div>
                  <div className="text-[10px] text-outline">Virtual Card Issuance</div>
                </div>
              </div>
              <div className="space-y-2 font-mono text-xs text-on-surface-variant mb-5">
                <div className="flex justify-between"><span>Card amount</span><span className="font-bold">$50.00 USD</span></div>
                <div className="flex justify-between"><span>Fixed fee</span><span>$0.10</span></div>
                <div className="flex justify-between"><span>Variable fee (20 bps)</span><span>$0.10</span></div>
                <div className="border-t border-outline-variant/30 pt-2 mt-2 flex justify-between font-bold text-sm">
                  <span>Total</span><span className="text-primary">$0.20</span>
                </div>
              </div>
              <div className="rounded-lg bg-surface-container-low p-3 space-y-1">
                <div className="flex items-center gap-2 text-[10px] text-on-surface-variant">
                  <Icon name="check_circle" className="text-tertiary text-sm" filled />
                  <span>Card ending <span className="font-mono font-bold">4242</span> issued</span>
                </div>
                <div className="flex items-center gap-2 text-[10px] text-on-surface-variant">
                  <Icon name="check_circle" className="text-tertiary text-sm" filled />
                  <span>Fee collected on-chain (Soroban)</span>
                </div>
                <div className="flex items-center gap-2 text-[10px] text-on-surface-variant">
                  <Icon name="check_circle" className="text-tertiary text-sm" filled />
                  <span>Available for immediate use</span>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Features Grid */}
        <section className="py-20 xl:py-28 px-8 xl:px-16 bg-surface-container-low">
          <div className="max-w-7xl mx-auto">
            <h2 className="text-5xl xl:text-6xl font-headline font-bold tracking-tight text-center mb-16 animate-fade-in-up">
              Everything you need
            </h2>
            <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6 xl:gap-8">
              {[
                { icon: "bolt", title: "Instant Issuance", desc: "Cards are active immediately upon creation. No waiting period." },
                { icon: "lock", title: "PCI Compliant", desc: "Card data never touches our servers. Stripe handles all sensitive data." },
                { icon: "code", title: "Developer Native", desc: "Rust CLI, JSON output, pipe-friendly. Integrates into any workflow." },
                { icon: "currency_bitcoin", title: "Crypto to Fiat", desc: "Bridge on-chain assets to real-world spending power in seconds." },
                { icon: "shield", title: "Freeze Anytime", desc: "Instant card freeze/unfreeze via CLI or dashboard. Full control." },
                { icon: "analytics", title: "Full Audit Trail", desc: "Every action logged. Every transaction traceable. Compliance ready." },
              ].map((item, i) => (
                <div
                  key={item.title}
                  className={`bg-surface-container-lowest rounded-2xl p-6 xl:p-8 shadow-soft-diffuse hover:shadow-xl transition-all duration-300 group animate-fade-in-up delay-${(i % 3 + 1) * 100}`}
                >
                  <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center mb-4 group-hover:bg-primary/20 transition-colors duration-300">
                    <Icon name={item.icon} className="text-primary" />
                  </div>
                  <h3 className="font-headline font-bold mb-2 xl:text-lg">{item.title}</h3>
                  <p className="text-on-surface-variant text-sm xl:text-base leading-relaxed">{item.desc}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Pricing — REAL fees from CLI: $0.10 fixed + 0.20% variable */}
        <section id="pricing" className="py-20 xl:py-28 px-8 xl:px-16">
          <div className="max-w-5xl mx-auto text-center">
            <h2 className="text-5xl xl:text-6xl font-headline font-bold tracking-tight mb-4 animate-fade-in-up">
              Simple, transparent pricing
            </h2>
            <p className="text-on-surface-variant text-lg xl:text-xl mb-16">No monthly fees. Pay only for what you use.</p>
            <div className="bg-surface-container-lowest rounded-2xl p-10 xl:p-14 shadow-soft-diffuse max-w-2xl mx-auto animate-fade-in-up delay-200">
              <div className="flex items-baseline justify-center gap-3 mb-2">
                <span className="text-5xl font-headline font-bold text-primary">$0.10</span>
                <span className="text-2xl font-headline font-bold text-on-surface-variant">+ 0.20%</span>
              </div>
              <p className="text-on-surface-variant mb-8">fixed fee + variable per card issuance</p>
              <div className="bg-surface-container-low rounded-xl p-4 mb-8 text-left">
                <p className="text-xs text-on-surface-variant mb-2 font-headline font-bold uppercase tracking-wider">Example: $50.00 card</p>
                <div className="space-y-1 font-mono text-sm">
                  <div className="flex justify-between"><span className="text-on-surface-variant">Fixed fee</span><span>$0.10</span></div>
                  <div className="flex justify-between"><span className="text-on-surface-variant">Variable (20 bps)</span><span>$0.10</span></div>
                  <div className="flex justify-between border-t border-outline-variant/30 pt-1 mt-1 font-bold"><span>Total fee</span><span className="text-primary">$0.20</span></div>
                </div>
              </div>
              <div className="space-y-3 text-left max-w-xs mx-auto">
                {["No monthly subscription", "No minimum deposit", "Cards from $5 to $500", "Testnet testing is free", "XLM & USDC accepted", "On-chain fee collection via Soroban"].map((line) => (
                  <div key={line} className="flex items-center gap-3 text-sm">
                    <Icon name="check_circle" className="text-tertiary text-lg" filled />
                    <span>{line}</span>
                  </div>
                ))}
              </div>
              <Link href="/dashboard" className="mt-8 inline-flex px-8 py-4 bg-primary text-on-primary rounded-xl font-bold shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all">
                Start Building
              </Link>
            </div>
          </div>
        </section>

        {/* Footer */}
        <footer className="py-16 px-8 xl:px-16 bg-inverse-surface text-inverse-on-surface">
          <div className="max-w-7xl mx-auto grid md:grid-cols-4 gap-12 xl:gap-16">
            <div>
              <h3 className="font-headline font-bold text-lg mb-4">StellarCard</h3>
              <p className="text-sm text-slate-400 leading-relaxed">Agent-first virtual card infrastructure funded by Stellar.</p>
            </div>
            {[
              { title: "Product", links: [
                { label: "How it Works", href: "#how-it-works" },
                { label: "Pricing", href: "#pricing" },
                { label: "Dashboard", href: "/dashboard" },
                { label: "Deposit", href: "/deposit" },
              ]},
              { title: "Resources", links: [
                { label: "Live Demo", href: "https://card.batuhan4.com" },
                { label: "GitHub", href: "https://github.com/Batuhan4/stellar-card" },
                { label: "Stellar Expert", href: "https://stellar.expert/explorer/testnet" },
                { label: "Cards", href: "/cards" },
              ]},
              { title: "Legal", links: [
                { label: "Privacy", href: "#" },
                { label: "Terms", href: "#" },
                { label: "Compliance", href: "#" },
              ]},
            ].map((col) => (
              <div key={col.title}>
                <h4 className="font-headline font-semibold text-sm uppercase tracking-widest mb-4 text-slate-400">{col.title}</h4>
                <div className="space-y-2 text-sm">
                  {col.links.map((l) => (
                    <a key={l.label} href={l.href} className="block hover:text-white transition-colors cursor-pointer">{l.label}</a>
                  ))}
                </div>
              </div>
            ))}
          </div>
          <div className="max-w-7xl mx-auto mt-12 pt-8 border-t border-white/10 text-center text-sm text-slate-400">
            &copy; 2026 StellarCard. All rights reserved.
          </div>
        </footer>
        <MobileBottomNav />
      </main>
    </LazyMotion>
  );
}
