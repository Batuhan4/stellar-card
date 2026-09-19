"use client";

import { LazyMotion, domAnimation } from "framer-motion";
import { Navbar } from "@/components/Navbar";
import { CrabMascot } from "@/components/CrabMascot";
import { Icon } from "@/components/Icon";
import Link from "next/link";

export default function LandingPage() {
  return (
    <LazyMotion features={domAnimation} strict>
      <Navbar />
      <main className="pt-24">
        {/* Hero */}
        <section className="relative min-h-[900px] flex items-center px-8 max-w-7xl mx-auto">
          <div className="absolute -top-24 -left-24 w-[600px] h-[600px] bg-primary/5 rounded-full blur-[120px]" />
          <div className="absolute top-1/2 -right-24 w-[400px] h-[400px] bg-secondary/5 rounded-full blur-[100px]" />

          <div className="grid lg:grid-cols-2 gap-16 items-center w-full relative z-10">
            <div className="space-y-8 animate-fade-in-up">
              <div className="inline-flex items-center gap-2 bg-surface-container-low px-4 py-1.5 rounded-full">
                <span className="w-2 h-2 rounded-full bg-tertiary-container animate-pulse" />
                <span className="text-[0.65rem] font-headline font-bold uppercase tracking-[0.2em] text-on-surface-variant">
                  Stellar Testnet Live
                </span>
              </div>

              <h1 className="text-6xl md:text-7xl font-headline font-bold leading-[0.9] tracking-tight text-on-surface">
                Virtual cards for{" "}
                <span className="text-primary">agents</span> and developers
              </h1>

              <p className="text-xl text-on-surface-variant max-w-md leading-relaxed">
                Fund with Stellar. Spend anywhere. The first CLI-native
                infrastructure for instant global liquidity.
              </p>

              <div className="flex flex-wrap gap-4 pt-4">
                <Link
                  href="/dashboard"
                  className="px-8 py-4 bg-primary text-on-primary rounded-xl font-bold flex items-center gap-3 shadow-lg shadow-primary/20 hover:shadow-primary/40 transition-all"
                >
                  Connect Wallet
                  <Icon name="arrow_forward" className="text-[18px]" />
                </Link>
                <a href="#how-it-works" className="px-8 py-4 bg-surface-container-highest text-on-surface rounded-xl font-bold hover:bg-surface-variant transition-all">
                  View CLI Docs
                </a>
              </div>
            </div>

            {/* Right: Visual — Stitch 3D hero with crab */}
            <div className="relative h-[600px] flex items-center justify-center scene-3d">
              <div className="animate-float-3d relative">
                <div className="absolute -top-12 left-0 animate-crab-reveal z-0">
                  <CrabMascot size="lg" mood="idle" />
                </div>

                <div className="animate-card-rotation relative z-20">
                  <div className="w-[440px] h-[280px] bg-gradient-to-br from-slate-900 to-slate-800 rounded-[2rem] p-8 shadow-[0_50px_100px_-20px_rgba(0,0,0,0.3)] relative overflow-hidden border border-white/10">
                    <div className="absolute -top-24 -right-24 w-64 h-64 bg-primary/20 rounded-full blur-3xl" />
                    <div className="h-full flex flex-col justify-between relative z-10">
                      <div className="flex justify-between items-start">
                        <div className="w-12 h-10 bg-gradient-to-r from-amber-400 to-amber-200/50 rounded-lg opacity-80" />
                        <div className="text-white/40 font-mono tracking-widest text-xs uppercase">***REMOVED*** Global</div>
                      </div>
                      <div className="space-y-4">
                        <div className="text-2xl text-white font-mono tracking-[0.3em]">4242 &bull;&bull;&bull;&bull; &bull;&bull;&bull;&bull; 1085</div>
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

              <div className="absolute -top-12 -left-20 glass-panel p-5 rounded-2xl shadow-soft-diffuse border border-white/50 w-64 z-30 animate-fade-in-up delay-500">
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

              <div className="absolute -bottom-8 -right-16 glass-panel p-5 rounded-2xl shadow-soft-diffuse border border-white/50 w-60 z-30 animate-fade-in-up delay-700">
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
        <section id="how-it-works" className="py-20 bg-surface-container-low relative overflow-hidden">
          <div className="max-w-7xl mx-auto px-8 relative z-10">
            <div className="flex flex-col items-center text-center mb-16 animate-fade-in-up">
              <h2 className="text-4xl md:text-5xl font-headline font-bold text-on-surface mb-6 tracking-tight">
                Streamlined for Speed
              </h2>
              <p className="text-on-surface-variant max-w-xl text-lg">
                A straight line from crypto to consumer spending. No KYC hurdles for testnets, instant issuance on mainnet.
              </p>
            </div>

            <div className="hidden md:block absolute top-1/2 left-0 w-full h-[2px] bg-gradient-to-r from-transparent via-primary-fixed-dim to-transparent -translate-y-1/2 z-0" />

            <div className="relative grid md:grid-cols-3 gap-12">
              {[
                { icon: "terminal", title: "1. Generate address", desc: "Request a unique deposit address via CLI or wallet. Instant generation on the Stellar network.", code: "***REMOVED*** deposit address --asset xlm" },
                { icon: "currency_bitcoin", title: "2. Send XLM / USDC", desc: "Transfer assets to your assigned address. We detect the transaction within seconds via Horizon.", code: "***REMOVED*** card buy --amount 50" },
                { icon: "credit_card", title: "3. Receive card", desc: "A virtual Visa/Mastercard is issued instantly. Full card details returned in your terminal.", code: "***REMOVED*** card show crd_abc" },
              ].map((item, i) => (
                <div
                  key={item.title}
                  className={`relative z-10 group animate-fade-in-up delay-${(i + 1) * 200}`}
                >
                  <div className="bg-surface-container-lowest p-10 rounded-[2.5rem] shadow-soft-diffuse hover:shadow-lg transition-all border border-transparent hover:border-primary-fixed-dim">
                    <div className="w-16 h-16 bg-surface-container-low rounded-2xl flex items-center justify-center mb-8 text-primary group-hover:scale-110 transition-transform">
                      <Icon name={item.icon} className="text-3xl" filled />
                    </div>
                    <h3 className="text-2xl font-headline font-bold mb-4">{item.title}</h3>
                    <p className="text-on-surface-variant leading-relaxed mb-6">{item.desc}</p>
                    <div className="bg-inverse-surface text-inverse-on-surface font-mono text-xs rounded-lg p-3">
                      <span className="text-green-400">$ </span><span className="text-primary-fixed-dim">{item.code}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Agent Pitch */}
        <section id="security" className="py-20 px-8 max-w-7xl mx-auto">
          <div className="grid lg:grid-cols-2 gap-20 items-center">
            <div className="animate-fade-in-up">
              <h2 className="text-4xl md:text-5xl font-headline font-bold text-on-surface mb-8 tracking-tight">
                Built for the <br /><span className="text-secondary">Agentic Future</span>
              </h2>
              <div className="space-y-6">
                {["Headless Operations — Perfect for AI agents that need to pay autonomously", "Structured JSON Output — Every response machine-parseable", "Idempotent Operations — Safe to retry on network failures", "Zero Interactive Prompts — No human required in the loop", "Deterministic Exit Codes — Scriptable in any CI/CD pipeline"].map((feat) => (
                  <div key={feat} className="flex items-start gap-4">
                    <div className="w-6 h-6 rounded-full bg-secondary-fixed flex items-center justify-center mt-1 shrink-0">
                      <Icon name="check" className="text-secondary text-[14px]" filled />
                    </div>
                    <p className="text-on-surface-variant text-sm">{feat}</p>
                  </div>
                ))}
              </div>
            </div>

            <div className="bg-inverse-surface rounded-2xl p-6 font-mono text-sm shadow-soft-diffuse animate-fade-in-up delay-200">
              <div className="flex items-center gap-2 mb-4">
                <span className="w-3 h-3 rounded-full bg-error/60" />
                <span className="w-3 h-3 rounded-full bg-amber-400/60" />
                <span className="w-3 h-3 rounded-full bg-green-400/60" />
                <span className="text-slate-500 text-xs ml-2">agent_workflow.py</span>
              </div>
              <pre className="text-primary-fixed-dim overflow-x-auto leading-relaxed"><code>{`# AI Agent: autonomous card purchase
result = subprocess.run(
  ["***REMOVED***", "card", "buy",
   "--amount", "50",
   "--format", "json"],
  capture_output=True
)

data = json.loads(result.stdout)
if data["ok"]:
    card = data["data"]
    print(f"Card ready: {card['last4']}")
else:
    error = data["error"]
    print(f"Fix: {error['suggestion']}")`}</code></pre>
            </div>
          </div>
        </section>

        {/* Features Grid */}
        <section className="py-20 px-8 bg-surface-container-low">
          <div className="max-w-7xl mx-auto">
            <h2 className="text-5xl font-headline font-bold tracking-tight text-center mb-16 animate-fade-in-up">
              Streamlined for speed
            </h2>
            <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
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
                  className={`bg-surface-container-lowest rounded-2xl p-6 shadow-soft-diffuse hover:shadow-lg transition-all group animate-fade-in-up delay-${(i % 3 + 1) * 100}`}
                >
                  <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center mb-4 group-hover:bg-primary/20 transition-colors">
                    <Icon name={item.icon} className="text-primary" />
                  </div>
                  <h3 className="font-headline font-bold mb-2">{item.title}</h3>
                  <p className="text-on-surface-variant text-sm leading-relaxed">{item.desc}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Pricing — REAL fees from CLI: $0.10 fixed + 0.20% variable */}
        <section id="pricing" className="py-20 px-8">
          <div className="max-w-4xl mx-auto text-center">
            <h2 className="text-5xl font-headline font-bold tracking-tight mb-4 animate-fade-in-up">
              Simple, transparent pricing
            </h2>
            <p className="text-on-surface-variant text-lg mb-16">No monthly fees. Pay only for what you use.</p>
            <div className="bg-surface-container-lowest rounded-2xl p-10 shadow-soft-diffuse max-w-lg mx-auto animate-fade-in-up delay-200">
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
        <footer className="py-16 px-8 bg-inverse-surface text-inverse-on-surface">
          <div className="max-w-7xl mx-auto grid md:grid-cols-4 gap-12">
            <div>
              <h3 className="font-headline font-bold text-lg mb-4">***REMOVED***</h3>
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
                { label: "GitHub", href: "https://github.com" },
                { label: "Stellar Expert", href: "https://stellar.expert/explorer/testnet" },
                { label: "Security", href: "#security" },
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
          <div className="max-w-7xl mx-auto mt-12 pt-8 border-t border-white/10 text-center text-sm text-slate-500">
            &copy; 2026 ***REMOVED***. All rights reserved.
          </div>
        </footer>
      </main>
    </LazyMotion>
  );
}
