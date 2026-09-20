"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import dynamic from "next/dynamic";
import { Icon } from "./Icon";

const ConnectWallet = dynamic(
  () => import("./ConnectWallet").then((m) => m.ConnectWallet),
  { ssr: false }
);

const items = [
  { href: "/", icon: "home", label: "Home" },
  { href: "/quick", icon: "bolt", label: "Quick" },
  { href: "/dashboard", icon: "dashboard", label: "Overview" },
  { href: "/deposit", icon: "account_balance_wallet", label: "Deposit" },
  { href: "/cards", icon: "credit_card", label: "Cards" },
];

export function MobileTopBar() {
  return (
    <header className="md:hidden sticky top-0 z-40 glass-panel border-b border-outline-variant/20">
      <div className="flex items-center justify-between gap-3 px-4 py-3 min-w-0">
        <Link
          href="/"
          className="font-headline text-lg font-black tracking-tighter text-slate-900 truncate"
        >
          StellarCard
        </Link>
        <ConnectWallet />
      </div>
    </header>
  );
}

export function MobileBottomNav() {
  const pathname = usePathname();

  return (
    <nav className="md:hidden fixed bottom-0 inset-x-0 z-40 glass-panel border-t border-outline-variant/20 pb-[env(safe-area-inset-bottom)]">
      <div className="grid grid-cols-5">
        {items.map((item) => {
          const active =
            item.href === "/" ? pathname === "/" : pathname.startsWith(item.href);
          return (
            <Link
              key={item.href}
              href={item.href}
              className={`flex flex-col items-center gap-1 py-3 text-[9px] font-medium transition-colors ${
                active ? "text-primary" : "text-slate-500"
              }`}
            >
              <Icon name={item.icon} className="text-[20px]" filled={active} />
              <span>{item.label}</span>
            </Link>
          );
        })}
      </div>
    </nav>
  );
}
