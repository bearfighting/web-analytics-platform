import {
  Outlet,
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { act, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { TanStackRouterNavigationBridge } from "./tanstack-router-navigation-bridge";

import type { NavigationEvent } from "@web-analytics/observer-core";

describe("TanStackRouterNavigationBridge", () => {
  it("emits initial and resolved browser-history navigation events", async () => {
    const events: NavigationEvent[] = [];
    window.scrollTo = vi.fn();
    const rootRoute = createRootRoute({
      component: () => (
        <>
          <TanStackRouterNavigationBridge
            onNavigation={(event) => events.push(event)}
            now={() => 1}
          />
          <Outlet />
        </>
      ),
    });
    const indexRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/",
      component: () => <div />,
    });
    const userRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/users/$id",
      component: () => <div />,
    });
    const routeTree = rootRoute.addChildren([indexRoute, userRoute]);
    const router = createRouter({
      routeTree,
      history: createMemoryHistory({ initialEntries: ["/"] }),
    });

    render(<RouterProvider router={router} />);
    expect(events).toEqual([]);
    await act(async () => {
      await router.load();
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    await act(async () => {
      await router.navigate({ to: "/users/$id", params: { id: "42" } });
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(events.map(({ navigationType, path, url }) => ({ navigationType, path, url }))).toEqual([
      { navigationType: "initial", path: "/", url: "/" },
      { navigationType: "unknown", path: "/users/42", url: "/users/42" },
    ]);
  });

  it("keeps URL and path aligned across rapid resolved navigations", async () => {
    const events: NavigationEvent[] = [];
    window.scrollTo = vi.fn();
    const rootRoute = createRootRoute({
      component: () => (
        <>
          <TanStackRouterNavigationBridge
            onNavigation={(event) => events.push(event)}
            now={() => 1}
          />
          <Outlet />
        </>
      ),
    });
    const indexRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/",
      component: () => <div />,
    });
    const userRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/users/$id",
      component: () => <div />,
    });
    const routeTree = rootRoute.addChildren([indexRoute, userRoute]);
    const router = createRouter({
      routeTree,
      history: createMemoryHistory({ initialEntries: ["/"] }),
    });

    render(<RouterProvider router={router} />);
    await act(async () => {
      await router.load();
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
    await act(async () => {
      await Promise.allSettled([
        router.navigate({ to: "/users/$id", params: { id: "1" } }),
        router.navigate({ to: "/users/$id", params: { id: "2" } }),
      ]);
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(
      events
        .slice(1)
        .every((event) => new URL(event.url, "http://localhost").pathname === event.path),
    ).toBe(true);
  });
});
