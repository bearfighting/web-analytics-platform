import { createAnalytics } from "@web-analytics/analytics-browser";
import type { Transport } from "@web-analytics/analytics-core";
import type { Analytics } from "@web-analytics/analytics-browser";
import { createPlaygroundTransport } from "@web-analytics/playground-support";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";
import { StrictMode, useCallback, useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  Link,
  Outlet,
  RouterProvider,
  createRootRoute,
  createRoute,
  createRouter,
  useNavigate,
} from "@tanstack/react-router";

import type { NavigationEvent } from "@web-analytics/observer-core";
import { RouterAnalyticsBridge } from "@web-analytics/router-adapters/tanstack-router";

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
      siteId: import.meta.env.NEXT_PUBLIC_ANALYTICS_SITE_ID || "tanstack-router-playground",
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
        <Link to="/">Home</Link>{" "}
        <Link to="/users/$id" params={{ id: "42" }}>
          User
        </Link>{" "}
        <Link to="/search" search={{ q: "router" }}>
          Search
        </Link>{" "}
        <a href="#section">Hash</a>
      </nav>
      <button onClick={() => window.history.back()}>Back</button>
      <button onClick={() => window.history.forward()}>Forward</button>
      <Outlet />
      <pre data-testid="events">{JSON.stringify(events)}</pre>
      <output data-testid="analytics-events">{JSON.stringify(sentEvents)}</output>
    </>
  );
}
function Home() {
  return <h1>TanStack Router Home</h1>;
}
function User() {
  return <h1>TanStack Router User</h1>;
}
function Search() {
  const navigate = useNavigate();
  return (
    <>
      <h1>TanStack Router Search</h1>
      <button onClick={() => navigate({ to: "/users/$id", params: { id: "7" }, replace: true })}>
        Replace
      </button>
    </>
  );
}

const rootRoute = createRootRoute({ component: Shell });
const indexRoute = createRoute({ getParentRoute: () => rootRoute, path: "/", component: Home });
const userRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/users/$id",
  component: User,
});
const searchRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/search",
  validateSearch: (search) => ({ q: String(search.q ?? "") }),
  component: Search,
});
const routeTree = rootRoute.addChildren([indexRoute, userRoute, searchRoute]);
const router = createRouter({ routeTree });
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
