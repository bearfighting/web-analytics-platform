import type { NavigationEvent, NavigationType } from "@web-analytics/observer-core";

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
