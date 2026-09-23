export type CustomEventProperty =
  string | number | boolean | null | CustomEventProperty[] | { [key: string]: CustomEventProperty };

export interface CustomEvent {
  schema_version: 1;
  event_id: string;
  type: "custom_event";
  site_id: string;
  occurred_at: number;
  event_name: string;
  properties: Record<string, CustomEventProperty>;
  visitor_id?: string;
}

const forbiddenKeys = new Set([
  "email",
  "emailaddress",
  "useremail",
  "phone",
  "phonenumber",
  "name",
  "firstname",
  "lastname",
  "fullname",
  "address",
  "homeaddress",
  "streetaddress",
  "ip",
  "ipaddress",
  "useragent",
  "cookie",
  "password",
  "passwd",
  "token",
  "userid",
  "useridentifier",
]);

export function validateCustomEventProperties(properties: unknown): string | undefined {
  if (!properties || typeof properties !== "object" || Array.isArray(properties)) {
    return "properties must be an object";
  }

  const validateValue = (root: unknown): string | undefined => {
    let keys = 0;
    const ancestors = new WeakSet<object>();
    const visitContainer = (
      value: object,
      visitChildren: () => string | undefined,
    ): string | undefined => {
      if (ancestors.has(value)) return "properties must not be cyclic";
      ancestors.add(value);
      try {
        return visitChildren();
      } finally {
        ancestors.delete(value);
      }
    };
    const visit = (value: unknown, depth: number): string | undefined => {
      if (value === null || typeof value === "boolean") return undefined;
      if (typeof value === "number")
        return Number.isFinite(value) ? undefined : "numbers must be finite";
      if (typeof value === "string")
        return new TextEncoder().encode(value).length <= 256
          ? undefined
          : "strings must be at most 256 UTF-8 bytes";
      if (Array.isArray(value)) {
        if (depth > 4) return "properties nesting exceeds 4 levels";
        if (value.length > 20) return "arrays may contain at most 20 items";

        return visitContainer(value, () => {
          for (const item of value) {
            const error = visit(item, depth + 1);
            if (error) return error;
          }

          return undefined;
        });
      }
      if (typeof value === "object" && value !== null) {
        if (depth > 4) return "properties nesting exceeds 4 levels";

        return visitContainer(value, () => {
          for (const [key, child] of Object.entries(value)) {
            keys += 1;
            if (keys > 32) return "properties may contain at most 32 keys";
            if (!/^[A-Za-z][A-Za-z0-9_.-]{0,63}$/.test(key))
              return "property keys must use the allowed ASCII format";
            if (forbiddenKeys.has(key.toLowerCase().replace(/[_.-]/g, "")))
              return "properties contain a prohibited key";
            const error = visit(child, depth + 1);
            if (error) return error;
          }

          return undefined;
        });
      }

      return "properties must contain JSON values";
    };

    if (!root || typeof root !== "object" || Array.isArray(root)) {
      return "properties must be an object";
    }

    return visit(root, 0);
  };

  // Reject non-JSON input before stringify can rewrite NaN or drop undefined.
  const inputError = validateValue(properties);
  if (inputError) return inputError;

  // Also validate the emitted JSON because toJSON methods and getters can change it.
  let serialized: string | undefined;
  let normalized: unknown;
  try {
    serialized = JSON.stringify(properties);
    if (serialized !== undefined) normalized = JSON.parse(serialized);
  } catch {
    return "properties must contain JSON values";
  }
  if (!serialized) return "properties must be an object";
  const serializedError = validateValue(normalized);
  if (serializedError) return serializedError;
  if (new TextEncoder().encode(serialized).length > 8192) return "properties must be at most 8 KiB";

  return undefined;
}
