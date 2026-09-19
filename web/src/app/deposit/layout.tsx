import { Sidebar } from "@/components/Sidebar";

export default function DepositLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex min-h-screen">
      <Sidebar />
      <main className="flex-1 md:ml-64 min-h-screen">{children}</main>
    </div>
  );
}
