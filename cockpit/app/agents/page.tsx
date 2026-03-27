import { PageShell } from "@/components/layout/PageShell";
import { AgentsContent } from "./agents-content";

export default function AgentsPage() {
  return (
    <PageShell title="agents" subtitle="evolution chronicle">
      <AgentsContent />
    </PageShell>
  );
}
