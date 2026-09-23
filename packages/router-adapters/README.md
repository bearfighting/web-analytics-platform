# @web-analytics/router-adapters

Unified Router entrypoints for the browser Analytics SDK.

Import the `RouterAnalyticsBridge` from the subpath matching the Router used by
the application:

```tsx
import { RouterAnalyticsBridge } from "@web-analytics/router-adapters/react-router";

<BrowserRouter>
  <RouterAnalyticsBridge analytics={analytics} />
  <Routes />
</BrowserRouter>;
```

The bridge must be mounted below the matching Router provider. It binds the
provided Analytics instance to the existing Router adapter and optionally
forwards navigation events through `onNavigation`. Router detection and
multiple implicit bridges are intentionally unsupported.
