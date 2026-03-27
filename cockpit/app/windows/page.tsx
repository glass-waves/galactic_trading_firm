import { PageShell } from "@/components/layout/PageShell";
import { WindowsContent } from "./windows-content";

export default function WindowsPage() {
  return (
    <PageShell title="windows" subtitle="entry windows">
      <WindowsContent />
    </PageShell>
  );
}
