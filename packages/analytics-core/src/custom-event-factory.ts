import { validateCustomEventProperties } from "@web-analytics/protocol-ts";
import { ulid } from "ulid";

import type { CustomEvent, CustomEventProperty } from "@web-analytics/protocol-ts";

export interface CustomEventFactoryOptions {
  siteId: string;
  name: string;
  properties?: Record<string, CustomEventProperty>;
  visitorId?: string;
  createEventId?: () => string;
  now?: () => number;
}

export function createCustomEvent(options: CustomEventFactoryOptions): CustomEvent {
  if (!/^[A-Za-z][A-Za-z0-9_.-]{0,63}$/.test(options.name)) {
    throw new TypeError("Custom event name must match the protocol format.");
  }
  const properties = options.properties ?? {};
  const error = validateCustomEventProperties(properties);
  if (error) throw new TypeError(error);
  let propertiesSnapshot: Record<string, CustomEventProperty>;
  try {
    const serializedProperties = JSON.stringify(properties);
    if (serializedProperties === undefined) {
      throw new TypeError("Custom event properties must contain JSON values.");
    }
    propertiesSnapshot = JSON.parse(serializedProperties) as Record<string, CustomEventProperty>;
  } catch {
    throw new TypeError("Custom event properties must contain JSON values.");
  }
  const snapshotError = validateCustomEventProperties(propertiesSnapshot);
  if (snapshotError) throw new TypeError(snapshotError);

  return {
    schema_version: 1,
    event_id: options.createEventId?.() ?? ulid(),
    type: "custom_event",
    site_id: options.siteId,
    occurred_at: options.now?.() ?? Date.now(),
    event_name: options.name,
    properties: propertiesSnapshot,
    ...(options.visitorId === undefined ? {} : { visitor_id: options.visitorId }),
  };
}
