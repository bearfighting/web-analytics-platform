import type { CustomEvent } from "./custom-event";
import type { PageViewEvent } from "./page-view-event";
import type { WebVitalEvent } from "./web-vital-event";

export interface EventBatch {
  schema_version: 1;
  events: (PageViewEvent | CustomEvent | WebVitalEvent)[];
}
