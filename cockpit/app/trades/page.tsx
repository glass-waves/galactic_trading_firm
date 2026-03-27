import { PageShell } from "@/components/layout/PageShell";
import { TradesContent } from "./trades-content";

export default function TradesPage() {
  return (
    <PageShell title="trades" subtitle="trade analysis">
      <TradesContent />
    </PageShell>
  );
}
