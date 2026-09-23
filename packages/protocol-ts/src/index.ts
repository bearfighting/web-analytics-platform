import type { CustomEvent } from "./custom-event";
export type { EventBatch } from "./event-batch";
export type { CustomEvent, CustomEventProperty } from "./custom-event";
export { validateCustomEventProperties } from "./custom-event";
export type { PageViewEvent } from "./page-view-event";
export type { BrowserContextV1, ContextDimension, UnknownContextValue } from "./browser-context-v1";
import type { EventBatch } from "./event-batch";
import type { PageViewEvent } from "./page-view-event";

export type AnalyticsEvent = PageViewEvent | CustomEvent;
export type AnalyticsEventBatch = EventBatch;
