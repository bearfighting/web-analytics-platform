import { ulid } from "ulid";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { BrowserContextV1, PageViewEvent } from "@web-analytics/protocol-ts";

export interface PageViewEventFactoryOptions {
  siteId: string;
  visitorId?: string;
  context?: BrowserContextV1;
  createEventId?: () => string;
  now?: () => number;
}

export function createPageViewEvent(
  navigation: NavigationEvent,
  options: PageViewEventFactoryOptions,
): PageViewEvent {
  const identity =
    options.context === undefined
      ? {}
      : { context: options.context, context_schema_version: 1 as const };

  return {
    schema_version: 1,
    event_id: options.createEventId?.() ?? ulid(),
    type: "page_view",
    site_id: options.siteId,
    occurred_at: options.now?.() ?? Date.now(),
    path: navigation.path,
    ...(navigation.url === undefined ? {} : { url: navigation.url }),
    ...(navigation.title === undefined ? {} : { title: navigation.title }),
    ...(navigation.referrer === undefined ? {} : { referrer: navigation.referrer }),
    ...(options.visitorId === undefined ? {} : { visitor_id: options.visitorId }),
    ...identity,
  };
}
