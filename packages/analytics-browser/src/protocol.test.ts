import { readFile } from "node:fs/promises";

import { MemoryNavigationObserver } from "@web-analytics/observer-core";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import { describe, expect, it } from "vitest";

import { createAnalytics } from "./analytics";

import type { AnalyticsEvent } from "@web-analytics/protocol-ts";

interface MockTransport {
  batches: AnalyticsEvent[][];
  sendBatch(events: readonly AnalyticsEvent[]): Promise<void>;
}

async function loadPageViewValidator() {
  const schemaUrl = new URL(
    "../../../protocol/events/schemas/page-view-event.schema.json",
    import.meta.url,
  );
  const schema = JSON.parse(await readFile(schemaUrl, "utf8"));
  const contextSchema = JSON.parse(
    await readFile(
      new URL("../../../protocol/contexts/browser-context.schema.json", import.meta.url),
      "utf8",
    ),
  );
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  ajv.addSchema(contextSchema);

  return ajv.compile(schema);
}

describe("generated unified protocol events", () => {
  it("validate with the canonical page-view schema", async () => {
    const transport: MockTransport = {
      batches: [],
      async sendBatch(events) {
        this.batches.push([...events]);
      },
    };
    const analytics = createAnalytics({
      siteId: "site_example",
      consent: "granted",
      transport,
      createEventId: () => "01J00000000000000000000003",
      now: () => 203,
    });
    const observer = new MemoryNavigationObserver();
    analytics.observe(observer);
    observer.emit({
      url: "https://example.test/",
      path: "/",
      navigationType: "initial",
      occurredAt: 203,
    });
    await analytics.flush();

    const validate = await loadPageViewValidator();
    expect(transport.batches).toHaveLength(1);
    expect(validate(transport.batches[0]?.[0])).toBe(true);
  });
});
