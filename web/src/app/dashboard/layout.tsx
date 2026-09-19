import { Sidebar } from "@/components/Sidebar";
import { MobileBottomNav, MobileTopBar } from "@/components/MobileNav";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen">
      <MobileTopBar />
      <div className="flex">
        <Sidebar />
        <main className="flex-1 md:ml-64 p-4 sm:p-8 pb-24 md:pb-8 min-h-screen">
          {children}
        </main>
      </div>
      <MobileBottomNav />
    </div>
  );
}
