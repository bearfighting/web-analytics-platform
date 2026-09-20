import type { PageViewEventV2 } from "./page-view-event-v2";

export interface EventBatchV2 {
  schema_version: 2;
  events: PageViewEventV2[];
}
