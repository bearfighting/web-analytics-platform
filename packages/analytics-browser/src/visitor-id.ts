export interface VisitorIdStore {
  read(): string | null;
  write(visitorId: string): boolean;
  remove(): void;
}

const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

export function isCanonicalVisitorId(value: unknown): value is string {
  return typeof value === "string" && UUID_V4.test(value);
}

export function createVisitorIdStore(siteId: string): VisitorIdStore {
  const key = `web-analytics:visitor:${siteId}`;
  const storage = () => {
    try {
      return typeof window === "undefined" ? null : window.localStorage;
    } catch {
      return null;
    }
  };

  return {
    read() {
      try {
        const value = storage()?.getItem(key) ?? null;

        return isCanonicalVisitorId(value) ? value : null;
      } catch {
        return null;
      }
    },
    write(visitorId) {
      if (!isCanonicalVisitorId(visitorId)) return false;
      try {
        const target = storage();
        if (!target) return false;
        target.setItem(key, visitorId);

        return true;
      } catch {
        return false;
      }
    },
    remove() {
      try {
        storage()?.removeItem(key);
      } catch {
        /* optional */
      }
    },
  };
}

export function createVisitorId(store: VisitorIdStore): string | null {
  const existing = store.read();
  if (existing) return existing;
  try {
    const value = globalThis.crypto?.randomUUID?.();
    if (!value || !isCanonicalVisitorId(value)) return null;

    return store.write(value) ? value : null;
  } catch {
    return null;
  }
}
