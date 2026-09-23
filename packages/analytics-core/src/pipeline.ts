import { createPageViewEvent, type PageViewEventFactoryOptions } from "./event-factory";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { AnalyticsEvent, PageViewEvent } from "@web-analytics/protocol-ts";

export type BeforeSend = (event: AnalyticsEvent) => AnalyticsEvent | null;

export interface Transport {
  sendBatch(events: readonly AnalyticsEvent[], options?: { keepalive?: boolean }): Promise<void>;
}

export interface ProcessNavigationOptions extends PageViewEventFactoryOptions {
  beforeSend?: BeforeSend;
}

export function processNavigation(
  navigation: NavigationEvent,
  options: ProcessNavigationOptions,
): PageViewEvent | null {
  const event = createPageViewEvent(navigation, options);

  return processPageViewEvent(event, options.beforeSend);
}

export function processPageViewEvent(
  event: PageViewEvent,
  beforeSend?: BeforeSend,
): PageViewEvent | null {
  if (!beforeSend) {
    return event;
  }

  const processed = beforeSend(event);
  if (processed === null) return null;
  if (
    processed.schema_version !== event.schema_version ||
    processed.type !== event.type ||
    processed.event_id !== event.event_id ||
    processed.site_id !== event.site_id
  ) {
    throw new TypeError("beforeSend must preserve schema_version, type, event_id, and site_id");
  }

  return processed as PageViewEvent;
}

export function isSameNavigation(previous: NavigationEvent | null, current: NavigationEvent) {
  return previous?.url === current.url && previous?.path === current.path;
}
