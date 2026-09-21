import { createPageViewEvent, type PageViewEventFactoryOptions } from "./event-factory";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";

export type BeforeSend = (event: AnalyticsEvent) => AnalyticsEvent | null;

export interface Transport {
  sendBatch(events: readonly AnalyticsEvent[]): Promise<void>;
}

export interface ProcessNavigationOptions extends PageViewEventFactoryOptions {
  beforeSend?: BeforeSend;
}

export function processNavigation(
  navigation: NavigationEvent,
  options: ProcessNavigationOptions,
): AnalyticsEvent | null {
  const event = createPageViewEvent(navigation, options);

  return processPageViewEvent(event, options.beforeSend);
}

export function processPageViewEvent(
  event: AnalyticsEvent,
  beforeSend?: BeforeSend,
): AnalyticsEvent | null {
  if (!beforeSend) {
    return event;
  }

  return beforeSend(event);
}

export function isSameNavigation(previous: NavigationEvent | null, current: NavigationEvent) {
  return previous?.url === current.url && previous?.path === current.path;
}
