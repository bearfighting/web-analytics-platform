import type { BrowserContextV1, ContextDimension } from "@web-analytics/protocol-ts";

export interface BrowserContextProvider {
  getContext(): BrowserContextV1;
}

const utmKeys = ["source", "medium", "campaign", "term", "content"] as const;
const unknownDimension: ContextDimension = "unknown";

export function createBrowserContextProvider(): BrowserContextProvider {
  return {
    getContext() {
      const context: BrowserContextV1 = {
        language: "unknown",
        timezone: "unknown",
        viewport_width: unknownDimension,
        viewport_height: unknownDimension,
        screen_width: unknownDimension,
        screen_height: unknownDimension,
        user_agent: "unknown",
      };
      if (typeof window === "undefined" || typeof navigator === "undefined") return context;
      try {
        if (navigator.language) context.language = navigator.language;
      } catch {
        /* optional */
      }
      try {
        if (navigator.userAgent) context.user_agent = navigator.userAgent;
      } catch {
        /* optional */
      }
      try {
        const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        if (timezone) context.timezone = timezone;
      } catch {
        /* optional */
      }
      try {
        if (Number.isFinite(window.innerWidth) && window.innerWidth >= 0)
          context.viewport_width = window.innerWidth;
      } catch {
        /* optional */
      }
      try {
        if (Number.isFinite(window.innerHeight) && window.innerHeight >= 0)
          context.viewport_height = window.innerHeight;
      } catch {
        /* optional */
      }
      try {
        if (Number.isFinite(window.screen?.width) && window.screen.width >= 0)
          context.screen_width = window.screen.width;
      } catch {
        /* optional */
      }
      try {
        if (Number.isFinite(window.screen?.height) && window.screen.height >= 0)
          context.screen_height = window.screen.height;
      } catch {
        /* optional */
      }
      try {
        if (document.referrer) context.referrer = document.referrer;
      } catch {
        /* optional */
      }
      try {
        const params = new URL(window.location.href).searchParams;
        for (const key of utmKeys) {
          const value = params.get(`utm_${key}`);
          if (value) context[`utm_${key}`] = value;
        }
      } catch {
        /* optional */
      }

      return context;
    },
  };
}
