import { ulid } from "ulid";

import type { NavigationEvent } from "@web-analytics/observer-core";
import type { BrowserContextV1, PageViewEvent, PageViewEventV2 } from "@web-analytics/protocol-ts";

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

export interface PageViewEventV2FactoryOptions extends PageViewEventFactoryOptions {
  visitorId?: string;
  context: BrowserContextV1;
}

export function createPageViewEventV2(
  navigation: NavigationEvent,
  options: PageViewEventV2FactoryOptions,
): PageViewEventV2 {
  return {
    schema_version: 2,
    event_id: options.createEventId?.() ?? ulid(),
    type: "page_view",
    site_id: options.siteId,
    ...(options.visitorId === undefined ? {} : { visitor_id: options.visitorId }),
    occurred_at: options.now?.() ?? Date.now(),
    path: navigation.path,
    ...(navigation.url === undefined ? {} : { url: navigation.url }),
    ...(navigation.title === undefined ? {} : { title: navigation.title }),
    ...(navigation.referrer === undefined ? {} : { referrer: navigation.referrer }),
    context_schema_version: 1,
    context: options.context,
  };
}
