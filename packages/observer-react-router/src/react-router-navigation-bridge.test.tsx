import { act, render } from "@testing-library/react";
import { MemoryRouter, Outlet, Route, Routes, useNavigate } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { ReactRouterNavigationBridge } from "./react-router-navigation-bridge";

import type { NavigationEvent } from "@web-analytics/observer-core";

function NavigationDriver({
  onNavigate,
  observer,
  onEvent,
}: {
  onNavigate: (navigate: ReturnType<typeof useNavigate>) => void;
  observer?: { emit: (event: NavigationEvent) => void };
  onEvent?: (event: NavigationEvent) => void;
}) {
  const navigate = useNavigate();
  onNavigate(navigate);

  return (
    <>
      <ReactRouterNavigationBridge
        observer={observer}
        onNavigation={onEvent ?? (() => {})}
        now={() => 1}
      />
      <Routes>
        <Route path="*" element={<Outlet />} />
      </Routes>
    </>
  );
}

describe("ReactRouterNavigationBridge", () => {
  it("maps initial, push, replace, pop, dynamic and search navigation", async () => {
    const events: Array<{ navigationType: string; path: string }> = [];
    let navigate!: ReturnType<typeof useNavigate>;
    render(
      <MemoryRouter initialEntries={["/"]}>
        <NavigationDriver
          onNavigate={(value) => (navigate = value)}
          observer={{ emit: (event) => events.push(event) }}
        />
      </MemoryRouter>,
    );

    await act(async () => navigate("/users/42"));
    await act(async () => navigate("/search?q=router", { replace: true }));
    await act(async () => navigate(-1));

    expect(events.map(({ navigationType, path }) => ({ navigationType, path }))).toEqual([
      { navigationType: "initial", path: "/" },
      { navigationType: "push", path: "/users/42" },
      { navigationType: "replace", path: "/search" },
      { navigationType: "pop", path: "/" },
    ]);
  });

  it("ignores hash-only navigation", async () => {
    const emit = vi.fn();
    let navigate!: ReturnType<typeof useNavigate>;
    render(
      <MemoryRouter initialEntries={["/"]}>
        <NavigationDriver onNavigate={(value) => (navigate = value)} observer={{ emit }} />
      </MemoryRouter>,
    );
    await act(async () => navigate("/#section"));
    expect(emit).toHaveBeenCalledTimes(1);
  });

  it("supports multiple consumers and stops after unmount", async () => {
    const observerEvents: NavigationEvent[] = [];
    const callbackEvents: NavigationEvent[] = [];
    let navigate!: ReturnType<typeof useNavigate>;
    const view = render(
      <MemoryRouter initialEntries={["/"]}>
        <NavigationDriver
          onNavigate={(value) => (navigate = value)}
          observer={{ emit: (event) => observerEvents.push(event) }}
          onEvent={(event) => callbackEvents.push(event)}
        />
      </MemoryRouter>,
    );

    await act(async () => navigate("/users/42"));
    expect(observerEvents).toHaveLength(2);
    expect(callbackEvents).toHaveLength(2);

    view.unmount();
    await act(async () => navigate("/search"));
    expect(observerEvents).toHaveLength(2);
    expect(callbackEvents).toHaveLength(2);
  });
});
