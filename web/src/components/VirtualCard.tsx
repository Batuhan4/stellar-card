"use client";

interface VirtualCardProps {
  name?: string;
  last4?: string;
  exp?: string;
  balance?: string;
  type?: "visa" | "mastercard";
  variant?: "dark" | "gradient";
  className?: string;
  animate3d?: boolean;
}

export function VirtualCard({
  name = "***REMOVED*** Agent",
  last4 = "4242",
  exp = "04/29",
  balance = "$250.00",
  type = "visa",
  variant = "gradient",
  className = "",
  animate3d = false,
}: VirtualCardProps) {
  const bgClass =
    variant === "gradient"
      ? "bg-gradient-to-br from-primary via-primary-container to-secondary"
      : "bg-gradient-to-br from-slate-800 to-slate-950";

  return (
    <div
      className={`relative w-full max-w-[380px] aspect-[1.586/1] rounded-2xl p-6 text-white overflow-hidden select-none ${bgClass} ${animate3d ? "card-rotate-3d" : ""} ${className}`}
    >
      <div className="absolute inset-0 bg-gradient-to-tr from-white/0 via-white/10 to-white/0 pointer-events-none" />
      <div className="w-10 h-7 rounded-md bg-amber-300/80 mb-6 flex items-center justify-center">
        <div className="w-6 h-4 rounded-sm border border-amber-500/40" />
      </div>
      <div className="font-mono text-lg tracking-[0.15em] mb-4 opacity-90">
        **** **** **** {last4}
      </div>
      <div className="flex justify-between items-end">
        <div>
          <p className="text-[10px] uppercase tracking-widest opacity-60 mb-1">Card Holder</p>
          <p className="font-headline text-sm font-semibold">{name}</p>
        </div>
        <div className="text-right">
          <p className="text-[10px] uppercase tracking-widest opacity-60 mb-1">Expires</p>
          <p className="font-mono text-sm">{exp}</p>
        </div>
      </div>
      <div className="absolute top-6 right-6 opacity-80">
        {type === "visa" ? (
          <span className="font-headline text-xl font-bold italic">VISA</span>
        ) : (
          <div className="flex -space-x-2">
            <div className="w-6 h-6 rounded-full bg-red-500 opacity-80" />
            <div className="w-6 h-6 rounded-full bg-amber-400 opacity-80" />
          </div>
        )}
      </div>
      <div className="absolute bottom-6 right-6 bg-white/15 backdrop-blur-sm px-3 py-1 rounded-full">
        <span className="font-mono text-xs font-medium">{balance}</span>
      </div>
    </div>
  );
}
