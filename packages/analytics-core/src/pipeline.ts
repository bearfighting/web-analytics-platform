import { createPageViewEvent, type PageViewEventFactoryOptions } from "./event-factory";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { PageViewEvent } from "@web-analytics/protocol-ts";

export type BeforeSend = (event: PageViewEvent) => PageViewEvent | null;

export interface Transport {
  sendBatch(events: readonly PageViewEvent[]): Promise<void>;
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

  return beforeSend(event);
}

export function isSameNavigation(previous: NavigationEvent | null, current: NavigationEvent) {
  return previous?.url === current.url && previous?.path === current.path;
}
