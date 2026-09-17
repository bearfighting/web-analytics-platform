import type { NavigationType } from "@web-analytics/observer-core";

export function createRouteIdentity(pathname: string, search: string): string {
  return JSON.stringify([pathname, search]);
}

export function resolveNavigationType(
  hasPreviousRoute: boolean,
  pendingNavigationType: NavigationType,
): NavigationType {
  if (!hasPreviousRoute) {
    return "initial";
  }

  return pendingNavigationType;
}

export function isHashOnlyChange(previousRouteIdentity: string, currentRouteIdentity: string) {
  return previousRouteIdentity === currentRouteIdentity;
}

export function isHashOnlyUrlChange(
  currentUrl: string,
  nextUrl: string | URL | null | undefined,
): boolean {
  if (nextUrl === null || nextUrl === undefined) {
    return true;
  }

  try {
    const current = new URL(currentUrl);
    const next = new URL(nextUrl.toString(), current);

    return current.pathname === next.pathname && current.search === next.search;
  } catch {
    return false;
  }
}
