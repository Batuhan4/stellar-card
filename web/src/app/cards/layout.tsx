import { Sidebar } from "@/components/Sidebar";
import { MobileBottomNav, MobileTopBar } from "@/components/MobileNav";

export default function CardsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen">
      <MobileTopBar />
      <div className="flex">
        <Sidebar />
        <main className="flex-1 md:ml-64 pb-24 md:pb-0 min-h-screen">{children}</main>
      </div>
      <MobileBottomNav />
    </div>
  );
}
