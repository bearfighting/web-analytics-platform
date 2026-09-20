import { describe, expect, it } from "vitest";

import type {
  BrowserContextV1,
  EventBatch,
  EventBatchV2,
  PageViewEvent,
  PageViewEventV2,
} from "./index";

describe("protocol types", () => {
  it("models a minimal PageViewEvent", () => {
    const event: PageViewEvent = {
      schema_version: 1,
      event_id: "01J00000000000000000000000",
      type: "page_view",
      site_id: "site_example",
      occurred_at: 1760000000000,
      path: "/about",
    };

    expect(event.type).toBe("page_view");
    expect(event.schema_version).toBe(1);
  });

  it("models a batch of PageViewEvents", () => {
    const batch: EventBatch = {
      schema_version: 1,
      events: [],
    };

    expect(batch.schema_version).toBe(1);
    expect(batch.events).toEqual([]);
  });

  it("does not allow non-PageViewEvent items in a batch", () => {
    // @ts-expect-error EventBatch only accepts PageViewEvent items.
    const invalidBatch: EventBatch = {
      schema_version: 1,
      events: [{ type: "custom" }],
    };

    expect(invalidBatch.events).toHaveLength(1);
  });

  it("models the isolated Phase 5 V2 contract without changing V1 types", () => {
    const context: BrowserContextV1 = {
      language: "en-CA",
      timezone: "America/Toronto",
      viewport_width: 1440,
      viewport_height: 900,
      screen_width: 2560,
      screen_height: 1440,
      user_agent: "unknown",
    };
    const event: PageViewEventV2 = {
      schema_version: 2,
      event_id: "01J00000000000000000000001",
      type: "page_view",
      site_id: "site_example",
      visitor_id: "550e8400-e29b-41d4-a716-446655440000",
      context_schema_version: 1,
      occurred_at: 1760000000000,
      path: "/about",
      context,
    };
    const batch: EventBatchV2 = { schema_version: 2, events: [event] };

    expect(batch.events[0].schema_version).toBe(2);
    expect(event.context?.user_agent).toBe("unknown");
  });
});
