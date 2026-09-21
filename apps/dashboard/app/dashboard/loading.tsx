import { Phase6LoadingState } from "../../components/phase6-loading-state";
import { LoadingState } from "../../components/states/loading-state";

export default function DashboardLoading() {
  return (
    <main className="dashboard-shell">
      <header className="dashboard-header">
        <p className="eyebrow">Web Analytics</p>
        <h1>Dashboard</h1>
      </header>
      <section className="card">
        <LoadingState context={{ siteId: "pending", dateRange: { from: "", to: "" } }} />
      </section>
      <Phase6LoadingState heading="Visitors and Sessions" />
      <Phase6LoadingState heading="Dimension Report" />
    </main>
  );
}
