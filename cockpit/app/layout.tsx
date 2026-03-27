import type { Metadata } from "next";
import { Sidebar } from "@/components/layout/Sidebar";
import "./globals.css";

export const metadata: Metadata = {
  title: "cockpit",
  description: "trading engine visualization",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body className="flex bg-bg text-text antialiased">
        <Sidebar />
        <div className="flex-1 min-w-0">{children}</div>
      </body>
    </html>
  );
}
