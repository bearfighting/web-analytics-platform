export interface PageViewEvent {
  schema_version: 1;
  event_id: string;
  type: "page_view";
  site_id: string;
  occurred_at: number;
  path: string;
  url?: string;
  title?: string;
  referrer?: string;
  context?: Record<string, unknown>;
}
