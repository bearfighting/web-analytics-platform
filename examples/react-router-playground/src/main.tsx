import { createAnalytics } from "@web-analytics/analytics-browser";
import type { Transport } from "@web-analytics/analytics-core";
import type { Analytics } from "@web-analytics/analytics-browser";
import { createPlaygroundTransport } from "@web-analytics/playground-support";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";
import { useCallback, useEffect, useMemo, useState } from "react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Link, Outlet, Route, Routes, useNavigate } from "react-router-dom";
import { BrowserRouter } from "react-router-dom";

import { RouterAnalyticsBridge } from "@web-analytics/router-adapters/react-router";
import type { NavigationEvent } from "@web-analytics/observer-core";

function Shell() {
  const [events, setEvents] = useState<NavigationEvent[]>([]);
  const [sentEvents, setSentEvents] = useState<AnalyticsEvent[]>([]);
  const [analytics, setAnalytics] = useState<Analytics | null>(null);
  const transport = useMemo<Transport>(
    () =>
      createPlaygroundTransport(
        {
          mode: import.meta.env.NEXT_PUBLIC_ANALYTICS_TRANSPORT,
          endpoint: import.meta.env.NEXT_PUBLIC_ANALYTICS_ENDPOINT,
          ingestKey: import.meta.env.NEXT_PUBLIC_ANALYTICS_INGEST_KEY,
        },
        (batch) => setSentEvents((current) => [...current, ...batch]),
      ),
    [],
  );
  useEffect(() => {
    const instance = createAnalytics({
      consent: "granted",
      siteId: import.meta.env.NEXT_PUBLIC_ANALYTICS_SITE_ID || "react-router-playground",
      transport,
      bufferSize: 1,
    });
    setAnalytics(instance);
    return () => {
      instance.destroy();
      setAnalytics(null);
    };
  }, [transport]);
  const handleNavigation = useCallback(
    (event: NavigationEvent) => setEvents((current) => [...current, event]),
    [],
  );

  return (
    <>
      {analytics ? (
        <RouterAnalyticsBridge analytics={analytics} onNavigation={handleNavigation} />
      ) : null}
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
