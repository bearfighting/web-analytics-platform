import type { NavigationEvent } from "@web-analytics/observer-core";

export function getCurrentNavigation(now: () => number = Date.now): NavigationEvent | null {
  if (typeof window === "undefined" || typeof document === "undefined") {
    return null;
  }

  return {
    url: window.location.href,
    path: window.location.pathname,
    title: document.title || undefined,
    referrer: document.referrer || undefined,
    navigationType: "unknown",
    occurredAt: now(),
  };
}
