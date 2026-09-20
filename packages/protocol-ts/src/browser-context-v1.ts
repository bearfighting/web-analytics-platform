export type UnknownContextValue = "unknown";
export type ContextDimension = number | UnknownContextValue;

export interface BrowserContextV1 {
  language: string;
  timezone: string;
  viewport_width: ContextDimension;
  viewport_height: ContextDimension;
  screen_width: ContextDimension;
  screen_height: ContextDimension;
  utm_source?: string;
  utm_medium?: string;
  utm_campaign?: string;
  utm_term?: string;
  utm_content?: string;
  referrer?: string;
  user_agent: string;
}
