import { createAnalytics } from "@web-analytics/analytics-browser";
import type { Transport } from "@web-analytics/analytics-core";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";
import { useEffect, useMemo, useState } from "react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Link, Outlet, Route, Routes, useNavigate } from "react-router-dom";
import { BrowserRouter } from "react-router-dom";

import { ReactRouterNavigationBridge } from "@web-analytics/observer-react-router";
import type { NavigationEvent } from "@web-analytics/observer-core";

function Shell() {
  const [events, setEvents] = useState<NavigationEvent[]>([]);
  const [sentEvents, setSentEvents] = useState<AnalyticsEvent[]>([]);
  const [analyticsReady, setAnalyticsReady] = useState(false);
  const observer = useMemo(
    () => ({
      listeners: new Set<(event: NavigationEvent) => void>(),
      emit(event: NavigationEvent) {
        this.listeners.forEach((listener) => listener(event));
      },
      subscribe(listener: (event: NavigationEvent) => void) {
        this.listeners.add(listener);
        return () => this.listeners.delete(listener);
      },
    }),
    [],
  );
  const transport = useMemo<Transport>(
    () => ({
      async sendBatch(batch) {
        setSentEvents((current) => [...current, ...batch]);
      },
    }),
    [],
  );
  useEffect(() => {
    const analytics = createAnalytics({
      consent: "granted",
      siteId: "react-router-playground",
      transport,
      bufferSize: 1,
    });
    const unsubscribeAnalytics = analytics.observe(observer);
    const unsubscribeLog = observer.subscribe((event) =>
      setEvents((current) => [...current, event]),
    );
    setAnalyticsReady(true);
    return () => {
      setAnalyticsReady(false);
      unsubscribeLog();
      unsubscribeAnalytics();
      analytics.destroy();
    };
  }, [observer, transport]);

  return (
    <>
      {analyticsReady ? <ReactRouterNavigationBridge observer={observer} /> : null}
      <nav>
        <Link to="/">Home</Link> <Link to="/users/42">User</Link>{" "}
        <Link to="/search?q=router">Search</Link> <a href="#section">Hash</a>
      </nav>
      <button type="button" onClick={() => window.history.back()}>
        Back
      </button>
      <button type="button" onClick={() => window.history.forward()}>
        Forward
      </button>
      <Outlet />
      <pre data-testid="events">{JSON.stringify(events)}</pre>
      <output data-testid="analytics-events">{JSON.stringify(sentEvents)}</output>
    </>
  );
}

function Home() {
  return <h1>React Router Home</h1>;
}
function User() {
  return <h1>React Router User</h1>;
}
function Search() {
  const navigate = useNavigate();
  return (
    <>
      <h1>React Router Search</h1>
      <button onClick={() => navigate("/users/7", { replace: true })}>Replace</button>
    </>
  );
}

export function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Shell />}>
          <Route index element={<Home />} />
          <Route path="users/:id" element={<User />} />
          <Route path="search" element={<Search />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
