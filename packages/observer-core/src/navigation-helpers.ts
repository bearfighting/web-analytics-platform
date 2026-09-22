import type { NavigationEvent, NavigationType } from "./types";

export interface NavigationEventSnapshot {
  url: string;
  path: string;
  title?: string;
  referrer?: string;
  navigationType: NavigationType;
  occurredAt: number;
}

export function createNavigationEvent(snapshot: NavigationEventSnapshot): NavigationEvent {
  return {
    url: snapshot.url,
    path: snapshot.path,
    ...(snapshot.title === undefined ? {} : { title: snapshot.title }),
    ...(snapshot.referrer === undefined ? {} : { referrer: snapshot.referrer }),
    navigationType: snapshot.navigationType,
    occurredAt: snapshot.occurredAt,
  };
}

export function createRouteIdentity(pathname: string, search: string): string {
  return JSON.stringify([pathname, search]);
}

export function isHashOnlyChange(previousRouteIdentity: string, currentRouteIdentity: string) {
  return previousRouteIdentity === currentRouteIdentity;
}

export function isHashOnlyUrlChange(
  currentUrl: string,
  nextUrl: string | URL | null | undefined,
): boolean {
  if (nextUrl === null || nextUrl === undefined) return true;

  try {
    const current = new URL(currentUrl);
    const next = new URL(nextUrl.toString(), current);

    return current.pathname === next.pathname && current.search === next.search;
  } catch {
    return false;
  }
}

export function resolveNavigationType(
  hasPreviousRoute: boolean,
  pendingNavigationType: NavigationType,
): NavigationType {
  return hasPreviousRoute ? pendingNavigationType : "initial";
}

export function mapNavigationType(value: string | undefined): NavigationType {
  switch (value) {
    case "PUSH":
    case "push":
      return "push";
    case "REPLACE":
    case "replace":
      return "replace";
    case "POP":
    case "pop":
      return "pop";
    default:
      return "unknown";
  }
}
