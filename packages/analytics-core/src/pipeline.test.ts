import { describe, expect, it } from "vitest";

import { createPageViewEvent } from "./event-factory";
import { isSameNavigation, processNavigation } from "./pipeline";

import type { NavigationEvent } from "@web-analytics/observer-core";

const navigation: NavigationEvent = {
  url: "https://example.com/about?source=test",
  path: "/about",
  title: "About",
  referrer: "https://example.com/",
  navigationType: "push",
  occurredAt: 1760000000000,
};

const factoryOptions = {
  siteId: "site_example",
  createEventId: () => "01J00000000000000000000000",
  now: () => 1760000000000,
};

describe("createPageViewEvent", () => {
  it("maps navigation data to a Protocol V1 PageViewEvent", () => {
    expect(createPageViewEvent(navigation, factoryOptions)).toEqual({
      schema_version: 1,
      event_id: "01J00000000000000000000000",
      type: "page_view",
      site_id: "site_example",
      occurred_at: 1760000000000,
      path: "/about",
      url: "https://example.com/about?source=test",
      title: "About",
      referrer: "https://example.com/",
    });
  });

  it("omits optional fields that are not present", () => {
    const minimalNavigation: NavigationEvent = {
      url: "https://example.com/",
      path: "/",
      navigationType: "initial",
      occurredAt: 1760000000000,
    };

    expect(createPageViewEvent(minimalNavigation, factoryOptions)).toEqual({
      schema_version: 1,
      event_id: "01J00000000000000000000000",
      type: "page_view",
      site_id: "site_example",
      occurred_at: 1760000000000,
      path: "/",
      url: "https://example.com/",
    });
  });
});

describe("processNavigation", () => {
  it("allows beforeSend to transform an event", () => {
    const event = processNavigation(navigation, {
      ...factoryOptions,
      beforeSend: (pageView) => ({ ...pageView, title: "Updated" }),
    });

    expect(event?.title).toBe("Updated");
  });

  it("drops an event when beforeSend returns null", () => {
    expect(
      processNavigation(navigation, {
        ...factoryOptions,
        beforeSend: () => null,
      }),
    ).toBeNull();
  });

  it("propagates beforeSend errors", () => {
    expect(() =>
      processNavigation(navigation, {
        ...factoryOptions,
        beforeSend: () => {
          throw new Error("blocked");
        },
      }),
    ).toThrow("blocked");
  });
});

describe("isSameNavigation", () => {
  it("compares URL and path", () => {
    expect(isSameNavigation(navigation, navigation)).toBe(true);
    expect(isSameNavigation(null, navigation)).toBe(false);
    expect(isSameNavigation(navigation, { ...navigation, path: "/other" })).toBe(false);
    expect(isSameNavigation(navigation, { ...navigation, url: "https://example.com/other" })).toBe(
      false,
    );
  });
});
