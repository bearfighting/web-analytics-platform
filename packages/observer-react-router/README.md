# @web-analytics/observer-react-router

React Router 7 adapter for the framework-agnostic `NavigationObserver` contract.

Mount `ReactRouterNavigationBridge` below a React Router provider. It observes
initial, push, replace, pop, pathname and search navigations. Hash-only changes
are intentionally ignored as standard navigation events.
