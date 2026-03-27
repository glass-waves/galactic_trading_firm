import { Header } from "./Header";

interface PageShellProps {
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}

export function PageShell({ title, subtitle, children }: PageShellProps) {
  return (
    <div className="flex flex-col min-h-screen w-full">
      <Header title={title} subtitle={subtitle} />
      <main className="flex-1 p-4">{children}</main>
    </div>
  );
}
