import { afterEach, describe, expect, it, vi } from "vitest";

import { createVisitorId, createVisitorIdStore, isCanonicalVisitorId } from "./visitor-id";

const firstId = "550e8400-e29b-41d4-a716-446655440000";
const secondId = "6ba7b810-9dad-41d1-80b4-00c04fd430c8";

afterEach(() => vi.unstubAllGlobals());

describe("visitor id storage", () => {
  it("uses a site-scoped localStorage key and rejects invalid values", () => {
    const values = new Map<string, string>();
    const storage = {
      getItem: vi.fn((key: string) => values.get(key) ?? null),
      setItem: vi.fn((key: string, value: string) => values.set(key, value)),
      removeItem: vi.fn((key: string) => values.delete(key)),
    };
    vi.stubGlobal("window", { localStorage: storage });
    const store = createVisitorIdStore("site_a");

    expect(store.read()).toBeNull();
    expect(store.write(firstId)).toBe(true);
    expect(storage.setItem).toHaveBeenCalledWith("web-analytics:visitor:site_a", firstId);
    expect(store.read()).toBe(firstId);
    expect(store.write(firstId.toUpperCase())).toBe(false);
    expect(isCanonicalVisitorId(firstId)).toBe(true);
    expect(isCanonicalVisitorId(firstId.toUpperCase())).toBe(false);
  });

  it("replaces invalid stored values with a generated UUID", () => {
    const values = new Map([["web-analytics:visitor:site_a", "not-a-uuid"]]);
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      removeItem: (key: string) => values.delete(key),
    };
    vi.stubGlobal("window", { localStorage: storage });
    vi.stubGlobal("crypto", { randomUUID: () => secondId });

    expect(createVisitorId(createVisitorIdStore("site_a"))).toBe(secondId);
    expect(values.get("web-analytics:visitor:site_a")).toBe(secondId);
  });

  it("returns no id when storage or crypto is unavailable", () => {
    vi.stubGlobal("window", {
      get localStorage() {
        throw new Error("storage denied");
      },
    });
    vi.stubGlobal("crypto", { randomUUID: () => firstId });
    expect(createVisitorId(createVisitorIdStore("site_a"))).toBeNull();

    vi.stubGlobal("window", {
      localStorage: {
        getItem: () => null,
        setItem: () => {
          throw new Error("write denied");
        },
      },
    });
    vi.stubGlobal("crypto", {});
    expect(createVisitorId(createVisitorIdStore("site_b"))).toBeNull();
  });
});
