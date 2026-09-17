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
  onBufferChange?: (bufferSize: number) => void;
  bufferSize?: number;
  flushIntervalMs?: number;
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
  const buffer: PageViewEvent[] = [];
  const settledErrors: unknown[] = [];
  const bufferSize = normalizePositiveInteger(options.bufferSize, 20);
  const flushIntervalMs = normalizePositiveInteger(options.flushIntervalMs, 5000);
  let flushTimer: ReturnType<typeof setInterval> | undefined;
  let destroyed = false;

  const notifyBufferChange = () => {
    try {
      options.onBufferChange?.(buffer.length);
    } catch {
      // Observability callbacks must not interrupt event processing.
    }
  };

  const reportError = (error: unknown) => {
    settledErrors.push(error);
    try {
      options.onError?.(error);
    } catch {
      // Error reporting must not replace the original SDK error.
    }
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

  const sendBatch = (events: PageViewEvent[]) => {
    try {
      trackSend(Promise.resolve(transport.sendBatch(events)));
    } catch (error) {
      reportError(error);
    }
  };

  const flushBuffer = () => {
    if (buffer.length === 0) {
      return;
    }

    const events = buffer.splice(0, buffer.length);
    notifyBufferChange();
    sendBatch(events);
  };

  const ensureFlushTimer = () => {
    if (flushTimer || typeof window === "undefined") {
      return;
    }

    flushTimer = setInterval(flushBuffer, flushIntervalMs);
  };

  const enqueue = (event: PageViewEvent) => {
    buffer.push(event);
    notifyBufferChange();

    if (buffer.length >= bufferSize) {
      void flushBuffer();
    }
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

      enqueue(processed);
    } catch (error) {
      reportError(error);
    }
  };

  return {
    observe(observer) {
      if (destroyed) {
        return () => {};
      }

      ensureFlushTimer();
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
      ensureFlushTimer();
      const navigation = getCurrentNavigation(now);
      if (navigation) {
        handleNavigation(navigation, navigation.occurredAt);
      }
    },

    async flush() {
      flushBuffer();
      const sends = [...pendingSends];
      await Promise.allSettled(sends);

      if (settledErrors.length > 0) {
        throw settledErrors.shift();
      }
    },

    destroy() {
      if (destroyed) {
        return;
      }
      destroyed = true;
      if (flushTimer) {
        clearInterval(flushTimer);
        flushTimer = undefined;
      }
      buffer.splice(0, buffer.length);
      notifyBufferChange();
      for (const unsubscribe of [...subscriptions]) {
        unsubscribe();
      }
    },
  };
}

function normalizePositiveInteger(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isInteger(value) && value > 0 ? value : fallback;
}
