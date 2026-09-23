import type { CustomEvent } from "./custom-event";
import type { WebVitalEvent } from "./web-vital-event";
export type { EventBatch } from "./event-batch";
export type { CustomEvent, CustomEventProperty } from "./custom-event";
export { validateCustomEventProperties } from "./custom-event";
export type { PageViewEvent } from "./page-view-event";
export type {
  WebVitalEvent,
  WebVitalMetric,
  WebVitalRating,
  WebVitalNavigationType,
} from "./web-vital-event";
export { WEB_VITAL_METRICS, isWebVitalEvent, webVitalRating } from "./web-vital-event";
export type { BrowserContextV1, ContextDimension, UnknownContextValue } from "./browser-context-v1";
import type { EventBatch } from "./event-batch";
import type { PageViewEvent } from "./page-view-event";

export type AnalyticsEvent = PageViewEvent | CustomEvent | WebVitalEvent;
export type AnalyticsEventBatch = EventBatch;
