import { afterEach, describe, expect, it } from "vitest";

import { subscribeToHistoryNavigation } from "./history-patch";

describe("history navigation patch", () => {
  afterEach(() => {
    window.history.replaceState(null, "", "/");
  });

  it("shares one patch and restores it after consumers unsubscribe in either order", () => {
    const originalPushState = window.history.pushState;
    const first: Array<{ type: string; href: string }> = [];
    const second: Array<{ type: string; href: string }> = [];
    const unsubscribeFirst = subscribeToHistoryNavigation((type, href) =>
      first.push({ type, href }),
    );
    const unsubscribeSecond = subscribeToHistoryNavigation((type, href) =>
      second.push({ type, href }),
    );

    expect(window.history.pushState).not.toBe(originalPushState);
    unsubscribeFirst();
    window.history.pushState(null, "", "/next");
    expect(first).toEqual([]);
    expect(second).toHaveLength(1);
    expect(second[0].type).toBe("push");
    expect(new URL(second[0].href).pathname).toBe("/next");

    unsubscribeSecond();
    unsubscribeSecond();
    expect(window.history.pushState).toBe(originalPushState);
  });

  it("reports hash-only history updates as unknown", () => {
    const events: Array<{ type: string; href: string }> = [];
    const unsubscribe = subscribeToHistoryNavigation((type, href) => events.push({ type, href }));

    window.history.pushState(null, "", "/#section");

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("unknown");
    expect(new URL(events[0].href).pathname).toBe("/");
    expect(new URL(events[0].href).hash).toBe("#section");
    unsubscribe();
  });

  it("reports browser back and forward as pop", () => {
    const events: Array<{ type: string; href: string }> = [];
    const unsubscribe = subscribeToHistoryNavigation((type, href) => events.push({ type, href }));

    window.dispatchEvent(new PopStateEvent("popstate"));

    expect(events).toHaveLength(1);
    expect(events[0].type).toBe("pop");
    expect(new URL(events[0].href).pathname).toBe("/");
    unsubscribe();
  });
});
