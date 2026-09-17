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
