export interface BrowserContextProvider {
  getContext(): Record<string, unknown>;
}

const utmKeys = ["source", "medium", "campaign", "term", "content"] as const;

export function createBrowserContextProvider(): BrowserContextProvider {
  return {
    getContext() {
      if (typeof window === "undefined" || typeof navigator === "undefined") {
        return {};
      }

      const context: Record<string, unknown> = {};
      const add = (key: string, value: unknown) => {
        if (value !== undefined && value !== null && value !== "") {
          context[key] = value;
        }
      };

      try {
        add("language", navigator.language);
      } catch {
        // Language detection is optional.
      }

      try {
        add("user_agent", navigator.userAgent);
      } catch {
        // User-agent detection is optional.
      }

      try {
        add("timezone", Intl.DateTimeFormat().resolvedOptions().timeZone);
      } catch {
        // Timezone detection is optional.
      }

      try {
        add("viewport_width", window.innerWidth);
      } catch {
        // Viewport width is optional.
      }

      try {
        add("viewport_height", window.innerHeight);
      } catch {
        // Viewport height is optional.
      }

      try {
        add("screen_width", window.screen?.width);
      } catch {
        // Screen width is optional.
      }

      try {
        add("screen_height", window.screen?.height);
      } catch {
        // Screen height is optional.
      }

      try {
        const params = new URL(window.location.href).searchParams;
        for (const key of utmKeys) {
          add(`utm_${key}`, params.get(`utm_${key}`) ?? undefined);
        }
      } catch {
        // Campaign parameters are optional.
      }

      return context;
    },
  };
}
