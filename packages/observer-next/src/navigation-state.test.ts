import { describe, expect, it } from "vitest";

import {
  createRouteIdentity,
  isHashOnlyChange,
  isHashOnlyUrlChange,
  resolveNavigationType,
} from "./navigation-state";

describe("navigation state", () => {
  it("creates an identity from pathname and search params", () => {
    expect(createRouteIdentity("/search", "q=test")).toBe('["/search","q=test"]');
  });

  it("treats unchanged pathname and search params as hash-only or duplicate state", () => {
    expect(isHashOnlyChange(createRouteIdentity("/", ""), createRouteIdentity("/", ""))).toBe(true);
    expect(isHashOnlyChange(createRouteIdentity("/", ""), createRouteIdentity("/about", ""))).toBe(
      false,
    );
  });

  it("resolves initial and pending navigation types", () => {
    expect(resolveNavigationType(false, "push")).toBe("initial");
    expect(resolveNavigationType(true, "push")).toBe("push");
    expect(resolveNavigationType(true, "replace")).toBe("replace");
    expect(resolveNavigationType(true, "pop")).toBe("pop");
    expect(resolveNavigationType(true, "unknown")).toBe("unknown");
  });

  it("identifies URL changes that only modify the hash", () => {
    expect(isHashOnlyUrlChange("https://example.test/about?q=1#old", "/about?q=1#new")).toBe(true);
    expect(isHashOnlyUrlChange("https://example.test/about?q=1", "/about?q=2")).toBe(false);
    expect(isHashOnlyUrlChange("https://example.test/about", null)).toBe(true);
  });
});
