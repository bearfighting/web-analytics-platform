import { createPageViewEvent, type PageViewEventFactoryOptions } from "./event-factory";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { PageViewEvent } from "@web-analytics/protocol-ts";

export type BeforeSend = (event: PageViewEvent) => PageViewEvent | null;

export interface ProcessNavigationOptions extends PageViewEventFactoryOptions {
  beforeSend?: BeforeSend;
}

export function processNavigation(
  navigation: NavigationEvent,
  options: ProcessNavigationOptions,
): PageViewEvent | null {
  const event = createPageViewEvent(navigation, options);

  if (!options.beforeSend) {
    return event;
  }

  return options.beforeSend(event);
}

export function isSameNavigation(previous: NavigationEvent | null, current: NavigationEvent) {
  return previous?.url === current.url && previous?.path === current.path;
}
