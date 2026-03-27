import { PageShell } from "@/components/layout/PageShell";
import { PipelineContent } from "./pipeline-content";

export default function PipelinePage() {
  return (
    <PageShell title="pipeline" subtitle="scoring flow">
      <PipelineContent />
    </PageShell>
  );
}
