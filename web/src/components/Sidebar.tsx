"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Icon } from "./Icon";
import dynamic from "next/dynamic";

const WalletInfo = dynamic(() => import("./ConnectWallet").then(m => m.WalletInfo), { ssr: false });

const navItems = [
  { href: "/dashboard", icon: "dashboard", label: "Overview" },
  { href: "/cards", icon: "credit_card", label: "Cards" },
  { href: "/deposit", icon: "account_balance_wallet", label: "Deposits" },
  { href: "#", icon: "terminal", label: "API Keys" },
  { href: "#", icon: "settings", label: "Settings" },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="hidden md:flex flex-col p-6 gap-2 h-screen w-64 bg-slate-50/50 backdrop-blur-lg fixed left-0 top-0 z-40 font-body leading-relaxed">
      <div className="mb-10 px-2">
        <Link href="/">
          <h1 className="font-headline text-lg font-black tracking-tighter text-slate-900">
            ***REMOVED***
          </h1>
        </Link>
        <p className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">Terminal Edition</p>
      </div>
      <nav className="flex flex-col gap-1 flex-1">
        {navItems.map((item) => {
          const isActive = pathname === item.href;
          return (
            <Link
              key={item.label}
              href={item.href}
              className={`flex items-center gap-3 px-4 py-3 transition-all duration-300 ease-in-out rounded-lg ${
                isActive
                  ? "bg-white text-primary rounded-lg shadow-sm font-semibold"
                  : "text-slate-500 hover:bg-slate-100"
              }`}
            >
              <Icon name={item.icon} className="text-[20px]" filled={isActive} />
              <span className="text-sm">{item.label}</span>
            </Link>
          );
        })}
      </nav>
      <div className="mt-auto pt-6 px-2 space-y-3">
        {/* Network indicator */}
        <div className="flex items-center gap-2 px-1">
          <span className="w-2 h-2 rounded-full bg-tertiary animate-pulse" />
          <span className="text-[10px] uppercase tracking-widest font-bold text-tertiary">Testnet</span>
        </div>
        <WalletInfo />
      </div>
    </aside>
  );
}
