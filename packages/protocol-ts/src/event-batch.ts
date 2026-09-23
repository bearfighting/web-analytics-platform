import type { CustomEvent } from "./custom-event";
import type { PageViewEvent } from "./page-view-event";

export interface EventBatch {
  schema_version: 1;
  events: (PageViewEvent | CustomEvent)[];
}
