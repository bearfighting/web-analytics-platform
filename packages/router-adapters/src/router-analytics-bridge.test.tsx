import { act, render, waitFor } from "@testing-library/react";
import { StrictMode } from "react";
import { MemoryRouter, Route, Routes, useNavigate, type NavigateFunction } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { RouterAnalyticsBridge as NextRouterAnalyticsBridge } from "./next";
import { RouterAnalyticsBridge as ReactRouterAnalyticsBridge } from "./react-router";
import { RouterAnalyticsBridge as TanStackRouterAnalyticsBridge } from "./tanstack-router";

import type { Analytics } from "@web-analytics/analytics-browser";
import type { NavigationEvent, NavigationObserver } from "@web-analytics/observer-core";

function createAnalytics() {
  const events: NavigationEvent[] = [];
  const analytics = {
    observe: vi.fn((observer: NavigationObserver) => {
      const unsubscribe = observer.subscribe((event) => events.push(event));

      return unsubscribe;
    }),
  } as unknown as Analytics;

  return { analytics, events };
}

describe("RouterAnalyticsBridge facade", () => {
  it("exports the same bridge name from all Router subpaths", () => {
    expect(NextRouterAnalyticsBridge).toBeTypeOf("function");
    expect(ReactRouterAnalyticsBridge).toBeTypeOf("function");
    expect(TanStackRouterAnalyticsBridge).toBeTypeOf("function");
  });

  it("subscribes analytics before mounting the React Router adapter", async () => {
    const { analytics, events } = createAnalytics();
    const callbackEvents: NavigationEvent[] = [];
    let navigate!: NavigateFunction;

    render(
      <MemoryRouter initialEntries={["/"]}>
        <ReactRouterAnalyticsBridge
          analytics={analytics}
          onNavigation={(event) => callbackEvents.push(event)}
          now={() => 1}
        />
        <Routes>
          <Route
            path="*"
            element={<NavigationDriver onNavigate={(value) => (navigate = value)} />}
          />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => expect(events).toHaveLength(1));
    expect(analytics.observe).toHaveBeenCalledOnce();
    expect(callbackEvents[0]).toMatchObject({ navigationType: "initial", path: "/" });
    expect(callbackEvents[0]).toBe(events[0]);

    await act(async () => navigate("/next"));
    expect(events).toHaveLength(2);
    expect(callbackEvents[1]).toMatchObject({ navigationType: "push", path: "/next" });
  });

  it("unsubscribes analytics when the facade unmounts", async () => {
    const { analytics, events } = createAnalytics();
    const callbackEvents: NavigationEvent[] = [];
    const view = render(
      <MemoryRouter initialEntries={["/"]}>
        <ReactRouterAnalyticsBridge
          analytics={analytics}
          onNavigation={(event) => callbackEvents.push(event)}
        />
      </MemoryRouter>,
    );

    await waitFor(() => expect(events).toHaveLength(1));
    expect(callbackEvents).toHaveLength(1);
    view.unmount();
    expect(analytics.observe).toHaveBeenCalledOnce();
    expect(callbackEvents).toHaveLength(1);
  });

  it("updates the callback without rebinding analytics", async () => {
    const { analytics, events } = createAnalytics();
    const firstCallback = vi.fn();
    const secondCallback = vi.fn();
    const view = render(
      <MemoryRouter initialEntries={["/"]}>
        <ReactRouterAnalyticsBridge analytics={analytics} onNavigation={firstCallback} />
      </MemoryRouter>,
    );

    await waitFor(() => expect(events).toHaveLength(1));
    view.rerender(
      <MemoryRouter initialEntries={["/"]}>
        <ReactRouterAnalyticsBridge analytics={analytics} onNavigation={secondCallback} />
      </MemoryRouter>,
    );

    expect(analytics.observe).toHaveBeenCalledOnce();
    expect(firstCallback).toHaveBeenCalledOnce();
    expect(secondCallback).not.toHaveBeenCalled();
    view.unmount();
  });

  it("rebinds when the analytics instance changes", async () => {
    const first = createAnalytics();
    const second = createAnalytics();
    let navigate!: NavigateFunction;
    const view = render(
      <MemoryRouter initialEntries={["/"]}>
        <ReactRouterAnalyticsBridge analytics={first.analytics} />
        <Routes>
          <Route
            path="*"
            element={<NavigationDriver onNavigate={(value) => (navigate = value)} />}
          />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => expect(first.events).toHaveLength(1));
    view.rerender(
      <MemoryRouter initialEntries={["/"]}>
        <ReactRouterAnalyticsBridge analytics={second.analytics} />
        <Routes>
          <Route
            path="*"
            element={<NavigationDriver onNavigate={(value) => (navigate = value)} />}
          />
        </Routes>
      </MemoryRouter>,
    );

    await waitFor(() => expect(second.events).toHaveLength(1));
    await act(async () => navigate("/next"));
    expect(first.events).toHaveLength(1);
    expect(second.events).toHaveLength(2);
    view.unmount();
  });

  it("does not duplicate the initial event in React Strict Mode", async () => {
    const { analytics, events } = createAnalytics();
    render(
      <StrictMode>
        <MemoryRouter initialEntries={["/"]}>
          <ReactRouterAnalyticsBridge analytics={analytics} />
        </MemoryRouter>
      </StrictMode>,
    );

    await waitFor(() => expect(events).toHaveLength(1));
    expect(analytics.observe).toHaveBeenCalled();
  });
});

function NavigationDriver({ onNavigate }: { onNavigate: (navigate: NavigateFunction) => void }) {
  onNavigate(useNavigate());

  return null;
}
