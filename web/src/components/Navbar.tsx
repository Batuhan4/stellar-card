"use client";

import Link from "next/link";
import dynamic from "next/dynamic";

const ConnectWallet = dynamic(() => import("./ConnectWallet").then(m => m.ConnectWallet), { ssr: false });

export function Navbar() {
  return (
    <nav className="fixed top-0 w-full z-50 glass-panel shadow-soft-diffuse">
      <div className="flex justify-between items-center px-4 sm:px-8 py-4 max-w-7xl mx-auto">
        <div className="flex items-center gap-8">
          <Link
            href="/"
            className="text-xl sm:text-2xl font-bold text-slate-900 font-headline tracking-tight"
          >
            StellarCard
          </Link>
          <div className="hidden md:flex gap-2">
            <a href="#how-it-works" className="text-slate-600 hover:text-primary hover:bg-primary/10 px-4 py-2 rounded-lg transition-all duration-200 font-body text-base font-medium">
              How it Works
            </a>
            <a href="#security" className="text-slate-600 hover:text-primary hover:bg-primary/10 px-4 py-2 rounded-lg transition-all duration-200 font-body text-base font-medium">
              Security
            </a>
            <a href="#pricing" className="text-slate-600 hover:text-primary hover:bg-primary/10 px-4 py-2 rounded-lg transition-all duration-200 font-body text-base font-medium">
              Pricing
            </a>
          </div>
        </div>
        <ConnectWallet />
      </div>
    </nav>
  );
}
