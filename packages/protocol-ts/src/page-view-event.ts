import type { BrowserContextV1 } from "./browser-context-v1";

interface PageViewEventBase {
  schema_version: 1;
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

export type PageViewEvent = PageViewEventBase &
  (
    | { context: BrowserContextV1; context_schema_version: 1 }
    | { context?: never; context_schema_version?: never }
  );
