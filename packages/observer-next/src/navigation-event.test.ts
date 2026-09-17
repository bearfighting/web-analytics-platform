import { describe, expect, it } from "vitest";

import { createNavigationEvent } from "./navigation-event";

describe("createNavigationEvent", () => {
  it("maps a deterministic navigation snapshot", () => {
    expect(
      createNavigationEvent({
        url: "https://example.com/about?source=test",
        path: "/about",
        title: "About",
        referrer: "https://example.com/",
        navigationType: "push",
        occurredAt: 1760000000000,
      }),
    ).toEqual({
      url: "https://example.com/about?source=test",
      path: "/about",
      title: "About",
      referrer: "https://example.com/",
      navigationType: "push",
      occurredAt: 1760000000000,
    });
  });

  it("omits absent optional fields", () => {
    expect(
      createNavigationEvent({
        url: "https://example.com/",
        path: "/",
        navigationType: "initial",
        occurredAt: 1760000000000,
      }),
    ).toEqual({
      url: "https://example.com/",
      path: "/",
      navigationType: "initial",
      occurredAt: 1760000000000,
    });
  });
});
