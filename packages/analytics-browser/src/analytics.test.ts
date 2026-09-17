import { MemoryNavigationObserver } from "@web-analytics/observer-core";
import { describe, expect, it, vi } from "vitest";

import { createAnalytics } from "./analytics";

import type { Transport } from "@web-analytics/analytics-core";
import type { PageViewEvent } from "@web-analytics/protocol-ts";

class MockTransport implements Transport {
  batches: PageViewEvent[][] = [];

  async sendBatch(events: Parameters<Transport["sendBatch"]>[0]) {
    this.batches.push([...events]);
  }
}

const navigation = {
  url: "https://example.test/about?source=x",
  path: "/about",
  title: "About",
  referrer: "https://example.test/",
  navigationType: "push" as const,
  occurredAt: 100,
};

describe("createAnalytics", () => {
  it("creates a manual page view from the browser location", async () => {
    vi.stubGlobal("window", {
      location: { href: "https://example.test/manual?utm_source=test", pathname: "/manual" },
    });
    vi.stubGlobal("document", {
      title: "Manual page",
      referrer: "https://referrer.test/",
    });
    const transport = new MockTransport();
    const clock = vi.fn(() => 202);
    const analytics = createAnalytics({
      siteId: "site_example",
      transport,
      createEventId: () => "01J00000000000000000000002",
      now: clock,
      contextProvider: { getContext: () => ({}) },
    });

    analytics.pageview();
    await analytics.flush();
    vi.unstubAllGlobals();

    expect(transport.batches[0]?.[0]).toMatchObject({
      url: "https://example.test/manual?utm_source=test",
      path: "/manual",
      title: "Manual page",
      referrer: "https://referrer.test/",
      occurred_at: 202,
    });
    expect(clock).toHaveBeenCalledOnce();
  });

  it("converts observed navigation and merges context", async () => {
    const transport = new MockTransport();
    const analytics = createAnalytics({
      siteId: "site_example",
      transport,
      createEventId: () => "01J00000000000000000000000",
      now: () => 200,
      contextProvider: { getContext: () => ({ language: "en-CA" }) },
    });
    const observer = new MemoryNavigationObserver();

    analytics.observe(observer);
    observer.emit(navigation);
    await analytics.flush();

    expect(transport.batches).toEqual([
      [
        {
          schema_version: 1,
          event_id: "01J00000000000000000000000",
          type: "page_view",
          site_id: "site_example",
          occurred_at: 200,
          url: navigation.url,
          path: navigation.path,
          title: navigation.title,
          referrer: navigation.referrer,
          context: { language: "en-CA" },
        },
      ],
    ]);
  });

  it("supports beforeSend modification and dropping", async () => {
    const transport = new MockTransport();
    const analytics = createAnalytics({
      siteId: "site_example",
      transport,
      beforeSend: (event) => ({ ...event, title: "Changed" }),
      createEventId: () => "01J00000000000000000000001",
      now: () => 201,
      contextProvider: { getContext: () => ({}) },
    });
    const observer = new MemoryNavigationObserver();
    analytics.observe(observer);
    observer.emit(navigation);
    await analytics.flush();
    expect(transport.batches[0]?.[0].title).toBe("Changed");

    const dropped = createAnalytics({
      siteId: "site_example",
      transport,
      beforeSend: () => null,
      contextProvider: { getContext: () => ({}) },
    });
    const droppedObserver = new MemoryNavigationObserver();
    dropped.observe(droppedObserver);
    droppedObserver.emit(navigation);
    await dropped.flush();
    expect(transport.batches).toHaveLength(1);
  });

  it("reports errors and rejects flush", async () => {
    const error = new Error("transport failed");
    const onError = vi.fn();
    const transport: Transport = { sendBatch: vi.fn(async () => Promise.reject(error)) };
    const analytics = createAnalytics({ siteId: "site_example", transport, onError });
    const observer = new MemoryNavigationObserver();
    analytics.observe(observer);
    observer.emit(navigation);

    await expect(analytics.flush()).rejects.toThrow("transport failed");
    expect(onError).toHaveBeenCalledWith(error);
  });

  it("waits for in-flight sends", async () => {
    let resolveSend!: () => void;
    const sendPromise = new Promise<void>((resolve) => {
      resolveSend = resolve;
    });
    const transport: Transport = { sendBatch: vi.fn(() => sendPromise) };
    const analytics = createAnalytics({ siteId: "site_example", transport });
    const observer = new MemoryNavigationObserver();
    analytics.observe(observer);
    observer.emit(navigation);

    let flushed = false;
    const flushPromise = analytics.flush().then(() => {
      flushed = true;
    });
    await Promise.resolve();
    expect(flushed).toBe(false);

    resolveSend();
    await flushPromise;
    expect(flushed).toBe(true);
  });

  it("waits for sends created while flushing", async () => {
    const resolvers: Array<() => void> = [];
    const transport: Transport = {
      sendBatch: vi.fn(
        () =>
          new Promise<void>((resolve) => {
            resolvers.push(resolve);
          }),
      ),
    };
    const analytics = createAnalytics({ siteId: "site_example", transport });
    const observer = new MemoryNavigationObserver();
    analytics.observe(observer);
    observer.emit(navigation);

    let flushed = false;
    const flushPromise = analytics.flush().then(() => {
      flushed = true;
    });
    await Promise.resolve();

    observer.emit({ ...navigation, path: "/next" });
    resolvers[0]?.();
    await Promise.resolve();
    expect(flushed).toBe(false);

    resolvers[1]?.();
    await flushPromise;
    expect(flushed).toBe(true);
  });

  it("reports beforeSend errors through onError and flush", async () => {
    const error = new Error("beforeSend failed");
    const onError = vi.fn();
    const transport = new MockTransport();
    const analytics = createAnalytics({
      siteId: "site_example",
      transport,
      beforeSend: () => {
        throw error;
      },
      onError,
    });
    const observer = new MemoryNavigationObserver();
    analytics.observe(observer);
    observer.emit(navigation);

    await expect(analytics.flush()).rejects.toThrow("beforeSend failed");
    expect(onError).toHaveBeenCalledWith(error);
    expect(transport.batches).toHaveLength(0);
  });

  it("is safe in Node and stops after destroy", async () => {
    const transport = new MockTransport();
    const analytics = createAnalytics({ siteId: "site_example", transport });
    const observer = new MemoryNavigationObserver();
    const unsubscribe = analytics.observe(observer);
    unsubscribe();
    unsubscribe();
    observer.emit(navigation);
    analytics.pageview();
    analytics.destroy();
    await analytics.flush();
    expect(transport.batches).toHaveLength(0);
  });
});
