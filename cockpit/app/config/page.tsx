import { PageShell } from "@/components/layout/PageShell";
import { ConfigContent } from "./config-content";

export default function ConfigPage() {
  return (
    <PageShell title="config" subtitle="version explorer">
      <ConfigContent />
    </PageShell>
  );
}
