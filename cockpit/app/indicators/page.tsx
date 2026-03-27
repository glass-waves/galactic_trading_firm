import { PageShell } from "@/components/layout/PageShell";
import { IndicatorsContent } from "./indicators-content";

export default function IndicatorsPage() {
  return (
    <PageShell title="indicators" subtitle="tool belt">
      <IndicatorsContent />
    </PageShell>
  );
}
