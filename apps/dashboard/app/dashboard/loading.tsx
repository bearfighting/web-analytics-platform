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
    </main>
  );
}
