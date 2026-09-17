import { describe, expect, it, vi } from "vitest";

import { MemoryNavigationObserver } from "./memory-observer";

import type { NavigationEvent } from "./types";

const event: NavigationEvent = {
  url: "https://example.com/about",
  path: "/about",
  navigationType: "initial",
  occurredAt: 1760000000000,
};

describe("MemoryNavigationObserver", () => {
  it("notifies listeners in registration order", () => {
    const observer = new MemoryNavigationObserver();
    const calls: string[] = [];

    observer.subscribe(() => calls.push("first"));
    observer.subscribe(() => calls.push("second"));

    observer.emit(event);

    expect(calls).toEqual(["first", "second"]);
  });

  it("stops notifying after unsubscribe and allows repeated unsubscribe", () => {
    const observer = new MemoryNavigationObserver();
    const listener = vi.fn();
    const unsubscribe = observer.subscribe(listener);

    unsubscribe();
    unsubscribe();
    observer.emit(event);

    expect(listener).not.toHaveBeenCalled();
  });

  it("continues notifying when one listener throws", () => {
    const observer = new MemoryNavigationObserver();
    const nextListener = vi.fn();

    observer.subscribe(() => {
      throw new Error("listener failure");
    });
    observer.subscribe(nextListener);

    expect(() => observer.emit(event)).not.toThrow();
    expect(nextListener).toHaveBeenCalledWith(event);
  });
});
