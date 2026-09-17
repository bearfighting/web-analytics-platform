import {
  createPageViewEvent,
  processPageViewEvent,
  type BeforeSend,
  type Transport,
} from "@web-analytics/analytics-core";

import { createBrowserContextProvider, type BrowserContextProvider } from "./browser-context";
import { getCurrentNavigation } from "./navigation";

import type { NavigationEvent, NavigationObserver } from "@web-analytics/observer-core";
import type { PageViewEvent } from "@web-analytics/protocol-ts";

export interface AnalyticsOptions {
  siteId: string;
  transport?: Transport;
  beforeSend?: BeforeSend;
  contextProvider?: BrowserContextProvider;
  createEventId?: () => string;
  now?: () => number;
  onError?: (error: unknown) => void;
}

export interface Analytics {
  observe(observer: NavigationObserver): () => void;
  pageview(): void;
  flush(): Promise<void>;
  destroy(): void;
}

const noOpTransport: Transport = {
  async sendBatch() {},
};

export function createAnalytics(options: AnalyticsOptions): Analytics {
  const transport = options.transport ?? noOpTransport;
  const contextProvider = options.contextProvider ?? createBrowserContextProvider();
  const now = options.now ?? Date.now;
  const subscriptions = new Set<() => void>();
  const pendingSends = new Set<Promise<void>>();
  const settledErrors: unknown[] = [];
  let destroyed = false;

  const reportError = (error: unknown) => {
    settledErrors.push(error);
    options.onError?.(error);
  };

  const trackSend = (promise: Promise<void>) => {
    const tracked = promise.catch((error: unknown) => {
      reportError(error);

      throw error;
    });
    pendingSends.add(tracked);
    void tracked.then(
      () => pendingSends.delete(tracked),
      () => pendingSends.delete(tracked),
    );
  };

  const handleNavigation = (navigation: NavigationEvent, occurredAt?: number) => {
    if (destroyed) {
      return;
    }

    try {
      const baseEvent = createPageViewEvent(navigation, {
        siteId: options.siteId,
        createEventId: options.createEventId,
        now: occurredAt === undefined ? options.now : () => occurredAt,
      });
      const context = contextProvider.getContext();
      const event: PageViewEvent =
        Object.keys(context).length === 0 ? baseEvent : { ...baseEvent, context };
      const processed = processPageViewEvent(event, options.beforeSend);

      if (processed === null) {
        return;
      }

      try {
        trackSend(Promise.resolve(transport.sendBatch([processed])));
      } catch (error) {
        reportError(error);
      }
    } catch (error) {
      reportError(error);
    }
  };

  return {
    observe(observer) {
      if (destroyed) {
        return () => {};
      }

      let active = true;
      const unsubscribeObserver = observer.subscribe(handleNavigation);
      const unsubscribe = () => {
        if (!active) {
          return;
        }
        active = false;
        subscriptions.delete(unsubscribe);
        unsubscribeObserver();
      };
      subscriptions.add(unsubscribe);

      return unsubscribe;
    },

    pageview() {
      if (destroyed) {
        return;
      }
      const navigation = getCurrentNavigation(now);
      if (navigation) {
        handleNavigation(navigation, navigation.occurredAt);
      }
    },

    async flush() {
      while (pendingSends.size > 0) {
        const sends = [...pendingSends];
        await Promise.allSettled(sends);
      }

      if (settledErrors.length > 0) {
        throw settledErrors.shift();
      }
    },

    destroy() {
      if (destroyed) {
        return;
      }
      destroyed = true;
      for (const unsubscribe of [...subscriptions]) {
        unsubscribe();
      }
    },
  };
}
