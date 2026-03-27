import { PageShell } from "@/components/layout/PageShell";
import { DashboardContent } from "./dashboard-content";

export default function DashboardPage() {
  return (
    <PageShell title="dashboard">
      <DashboardContent />
    </PageShell>
  );
}
