import { afterEach, describe, expect, it, vi } from "vitest";

import { createBrowserContextProvider } from "./browser-context";

describe("createBrowserContextProvider", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("collects available browser fields", () => {
    vi.stubGlobal("window", {
      innerWidth: 1200,
      innerHeight: 800,
      screen: { width: 1920, height: 1080 },
      location: { href: "https://example.test/about?utm_source=newsletter&utm_term=launch" },
    });
    vi.stubGlobal("navigator", { language: "en-CA", userAgent: "test-agent" });

    const context = createBrowserContextProvider().getContext();

    expect(context).toMatchObject({
      language: "en-CA",
      viewport_width: 1200,
      viewport_height: 800,
      screen_width: 1920,
      screen_height: 1080,
      user_agent: "test-agent",
      utm_source: "newsletter",
      utm_term: "launch",
    });
  });

  it("returns unknown values without browser globals", () => {
    expect(createBrowserContextProvider().getContext()).toMatchObject({
      language: "unknown",
      timezone: "unknown",
      viewport_width: "unknown",
      user_agent: "unknown",
    });
  });

  it("omits only browser fields whose APIs fail", () => {
    const window = {
      location: { href: "https://example.test/" },
      screen: { height: 1080 },
    } as { location: { href: string }; screen: { height: number } };
    Object.defineProperty(window, "innerWidth", {
      get() {
        throw new Error("width unavailable");
      },
    });
    Object.defineProperty(window, "innerHeight", { value: 800 });
    vi.stubGlobal("window", window);
    vi.stubGlobal("navigator", {
      language: "en-CA",
      get userAgent() {
        throw new Error("user agent unavailable");
      },
    });

    expect(createBrowserContextProvider().getContext()).toMatchObject({
      language: "en-CA",
      viewport_height: 800,
      screen_height: 1080,
    });
    expect(createBrowserContextProvider().getContext()).toMatchObject({
      viewport_width: "unknown",
      user_agent: "unknown",
    });
  });
});
