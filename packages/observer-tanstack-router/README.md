# @web-analytics/observer-tanstack-router

TanStack Router v1 adapter for the framework-agnostic `NavigationObserver`
contract. Mount `TanStackRouterNavigationBridge` below `RouterProvider`.

The bridge observes resolved route state, maps browser History navigation when
available, and ignores hash-only changes as standard navigation events.
