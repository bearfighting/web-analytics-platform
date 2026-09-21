import { FetchTransportError } from "./errors";

import type { Transport } from "@web-analytics/analytics-core";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";

export interface FetchTransportOptions {
  endpoint: string;
  ingestKey: string;
  fetch?: typeof globalThis.fetch;
}

interface CollectorErrorBody {
  error?: {
    code?: unknown;
    message?: unknown;
  };
}

export class FetchTransport implements Transport {
  private readonly endpoint: string;
  private readonly ingestKey: string;
  private readonly injectedFetch: typeof globalThis.fetch | undefined;

  constructor(options: FetchTransportOptions) {
    validateEndpoint(options.endpoint);
    if (options.ingestKey.length === 0) {
      throw new TypeError("FetchTransport ingestKey must not be empty");
    }

    this.endpoint = options.endpoint;
    this.ingestKey = options.ingestKey;
    this.injectedFetch = options.fetch;
  }

  async sendBatch(events: readonly AnalyticsEvent[]): Promise<void> {
    if (events.length === 0) {
      return;
    }

    const versions = [1, 2].filter((version) =>
      events.some((event) => event.schema_version === version),
    );
    for (const version of versions) {
      const versionEvents = events.filter((event) => event.schema_version === version);
      let response: Response;
      try {
        const request = {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Ingest-Key": this.ingestKey,
          },
          body: JSON.stringify({ schema_version: version, events: versionEvents }),
        };
        response = await (this.injectedFetch
          ? this.injectedFetch(this.endpoint, request)
          : globalThis.fetch(this.endpoint, request));
      } catch (cause) {
        throw new FetchTransportError(`Fetch request failed: ${errorMessage(cause)}`, {
          kind: "network",
          cause,
        });
      }

      if (response.status === 202) continue;

      const body = await readErrorBody(response);
      const code = typeof body?.error?.code === "string" ? body.error.code : undefined;
      const message =
        typeof body?.error?.message === "string"
          ? body.error.message
          : `Collector returned HTTP ${response.status}`;

      throw new FetchTransportError(message, {
        kind: code ? "collector" : "http",
        status: response.status,
        code,
        retryAfter: response.headers.get("retry-after") ?? undefined,
      });
    }
  }
}

function validateEndpoint(endpoint: string): void {
  let parsed: URL;
  try {
    parsed = new URL(endpoint);
  } catch {
    throw new TypeError("FetchTransport endpoint must be an absolute URL");
  }

  if (!/^https?:$/.test(parsed.protocol) || parsed.pathname !== "/v1/events") {
    throw new TypeError("FetchTransport endpoint must be an HTTP(S) /v1/events URL");
  }
}

async function readErrorBody(response: Response): Promise<CollectorErrorBody | undefined> {
  try {
    const body: unknown = await response.json();

    return isCollectorErrorBody(body) ? body : undefined;
  } catch {
    return undefined;
  }
}

function isCollectorErrorBody(value: unknown): value is CollectorErrorBody {
  return typeof value === "object" && value !== null && "error" in value;
}

function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}
