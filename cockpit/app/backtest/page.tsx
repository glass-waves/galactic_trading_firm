import { PageShell } from "@/components/layout/PageShell";
import { BacktestContent } from "./backtest-content";

export default function BacktestPage() {
  return (
    <PageShell title="backtest" subtitle="equity curves + analysis">
      <BacktestContent />
    </PageShell>
  );
}
