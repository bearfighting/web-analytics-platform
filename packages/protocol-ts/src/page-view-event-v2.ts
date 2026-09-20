import type { BrowserContextV1 } from "./browser-context-v1";

interface PageViewEventV2Base {
  schema_version: 2;
  event_id: string;
  type: "page_view";
  site_id: string;
  visitor_id?: string;
  occurred_at: number;
  path: string;
  url?: string;
  title?: string;
  referrer?: string;
}

export type PageViewEventV2 = PageViewEventV2Base &
  (
    | { context: BrowserContextV1; context_schema_version: 1 }
    | { context?: never; context_schema_version?: never }
  );
