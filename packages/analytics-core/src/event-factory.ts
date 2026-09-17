import { ulid } from "ulid";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { PageViewEvent } from "@web-analytics/protocol-ts";

export interface PageViewEventFactoryOptions {
  siteId: string;
  createEventId?: () => string;
  now?: () => number;
}

export function createPageViewEvent(
  navigation: NavigationEvent,
  options: PageViewEventFactoryOptions,
): PageViewEvent {
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
  };
}
