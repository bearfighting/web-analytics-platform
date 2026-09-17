import { describe, expect, it } from "vitest";

import type { EventBatch, PageViewEvent } from "./index";

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
});
