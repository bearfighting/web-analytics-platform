# @web-analytics/observer-next

Next.js App Router adapter for converting browser navigation into the framework-neutral `NavigationEvent` contract.

The adapter is exposed as the `NextNavigationBridge` Client Component. It does not create analytics events, send data, or access browser APIs during module import or SSR.
