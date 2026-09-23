import { MemoryNavigationObserver } from "@web-analytics/observer-core";
import { useEffect, useMemo, useRef, useState } from "react";

import type { Analytics } from "@web-analytics/analytics-browser";
import type { NavigationEvent } from "@web-analytics/observer-core";

export interface RouterAnalyticsBridgeProps {
  analytics: Analytics;
  onNavigation?: (event: NavigationEvent) => void;
  now?: () => number;
}

export function useRouterAnalyticsBridge(
  analytics: Analytics,
  onNavigation: ((event: NavigationEvent) => void) | undefined,
) {
  const observer = useMemo(() => new MemoryNavigationObserver(), []);
  const onNavigationRef = useRef(onNavigation);
  const [ready, setReady] = useState(false);
  const [bindingVersion, setBindingVersion] = useState(0);
  onNavigationRef.current = onNavigation;

  useEffect(() => {
    const unsubscribeNavigation = observer.subscribe((event) => {
      onNavigationRef.current?.(event);
    });
    const unsubscribe = analytics.observe(observer);
    setBindingVersion((version) => version + 1);
    setReady(true);

    return () => {
      setReady(false);
      unsubscribe();
      unsubscribeNavigation();
    };
  }, [analytics, observer]);

  return { bindingVersion, observer, ready };
}
