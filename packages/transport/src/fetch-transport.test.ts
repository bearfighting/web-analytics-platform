import { readFileSync } from "node:fs";

import { describe, expect, it, vi } from "vitest";

import { FetchTransportError } from "./errors";
import { FetchTransport } from "./fetch-transport";

import type { AnalyticsEvent, PageViewEvent } from "@web-analytics/protocol-ts";

interface HttpFixture {
  request: {
    body: string;
    headers: Record<string, string>;
    method: string;
    path: string;
  };
  expected: {
    body: string;
    status: number;
  };
}

const acceptedFixture = readFixture("accepted-single-event.json");
const acceptedCharsetFixture = readFixture("accepted-charset.json");
const invalidKeyFixture = readFixture("invalid-ingest-key.json");
const rateLimitedFixture = readFixture("rate-limited.json");
const endpoint = "http://localhost:4001/v1/events";

function readFixture(name: string): HttpFixture {
  return JSON.parse(
    readFileSync(new URL(`../../../protocol/http/fixtures/${name}`, import.meta.url), "utf8"),
  ) as HttpFixture;
}

function fixtureEvents(fixture: HttpFixture): readonly PageViewEvent[] {
  return (JSON.parse(fixture.request.body) as { events: PageViewEvent[] }).events;
}

const v2Event: AnalyticsEvent = {
  schema_version: 2,
  event_id: "01J00000000000000000000001",
  type: "page_view",
  site_id: "site_example",
  occurred_at: 1760000000000,
  path: "/v2",
  visitor_id: "550e8400-e29b-41d4-a716-446655440000",
  context_schema_version: 1,
  context: {
    language: "en-CA",
    timezone: "unknown",
    viewport_width: "unknown",
    viewport_height: "unknown",
    screen_width: "unknown",
    screen_height: "unknown",
    user_agent: "unknown",
  },
};

function responseFor(fixture: HttpFixture): Response {
  return new Response(fixture.expected.body, {
    status: fixture.expected.status,
    headers: { "content-type": "application/json" },
  });
}

describe("FetchTransport", () => {
  it("splits mixed V1 and V2 events into versioned requests", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(responseFor(acceptedFixture));
    const transport = new FetchTransport({
      endpoint,
      ingestKey: "public-key-example",
      fetch: fetchMock,
    });
    await transport.sendBatch([...fixtureEvents(acceptedFixture), v2Event]);

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(
      fetchMock.mock.calls.map(([, init]) => JSON.parse(String(init?.body)).schema_version),
    ).toEqual([1, 2]);
  });

  it("sends the canonical EventBatch request without manually setting Origin", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(responseFor(acceptedFixture));
    const transport = new FetchTransport({
      endpoint,
      ingestKey: acceptedFixture.request.headers["x-ingest-key"],
      fetch: fetchMock,
    });

    await transport.sendBatch(fixtureEvents(acceptedFixture));

    expect(fetchMock).toHaveBeenCalledOnce();
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe(endpoint);
    expect(init).toMatchObject({
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Ingest-Key": acceptedFixture.request.headers["x-ingest-key"],
      },
    });
    expect((init?.headers as Record<string, string>).Origin).toBeUndefined();
    expect(JSON.parse(String(init?.body))).toEqual(JSON.parse(acceptedFixture.request.body));
  });

  it("treats the canonical accepted fixture as success", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(responseFor(acceptedFixture));
    const transport = new FetchTransport({
      endpoint,
      ingestKey: "public-key-example",
      fetch: fetchMock,
    });

    await expect(transport.sendBatch(fixtureEvents(acceptedFixture))).resolves.toBeUndefined();
  });

  it("invokes the default browser fetch with globalThis as its receiver", async () => {
    const fetchMock = vi.fn(function (this: typeof globalThis) {
      if (this !== globalThis) {
        throw new TypeError("Illegal invocation");
      }

      return Promise.resolve(responseFor(acceptedFixture));
    }) as typeof globalThis.fetch;
    vi.stubGlobal("fetch", fetchMock);

    try {
      const transport = new FetchTransport({
        endpoint,
        ingestKey: "public-key-example",
      });

      await expect(transport.sendBatch(fixtureEvents(acceptedFixture))).resolves.toBeUndefined();
      expect(fetchMock).toHaveBeenCalledOnce();
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("accepts the canonical charset fixture body", async () => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(responseFor(acceptedCharsetFixture));
    const transport = new FetchTransport({
      endpoint,
      ingestKey: acceptedCharsetFixture.request.headers["x-ingest-key"],
      fetch: fetchMock,
    });

    await expect(
      transport.sendBatch(fixtureEvents(acceptedCharsetFixture)),
    ).resolves.toBeUndefined();
    expect(JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body))).toEqual(
      JSON.parse(acceptedCharsetFixture.request.body),
    );
  });

  it("does not send an empty batch", async () => {
    const fetchMock = vi.fn<typeof fetch>();
    const transport = new FetchTransport({
      endpoint,
      ingestKey: "public-key-example",
      fetch: fetchMock,
    });

    await transport.sendBatch([]);

    expect(fetchMock).not.toHaveBeenCalled();
  });

  it.each([
    [400, "invalid_event_batch", "collector"],
    [401, "invalid_ingest_key", "collector"],
    [403, "origin_not_allowed", "collector"],
    [413, "payload_too_large", "collector"],
    [429, "rate_limited", "collector"],
    [500, "collector_error", "collector"],
  ] as const)("maps HTTP %i with code %s", async (status, code, kind) => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(
      new Response(JSON.stringify({ error: { code, message: "fixture error" } }), {
        status,
        headers: status === 429 ? { "retry-after": "60" } : undefined,
      }),
    );
    const transport = new FetchTransport({
      endpoint,
      ingestKey: "public-key-example",
      fetch: fetchMock,
    });

    const error = await transport.sendBatch(fixtureEvents(acceptedFixture)).catch((value) => value);

    expect(error).toBeInstanceOf(FetchTransportError);
    expect(error).toMatchObject({ kind, status, code });
    if (status === 429) {
      expect(error).toMatchObject({ retryAfter: "60" });
    }
  });

  it("maps invalid-key and rate-limited canonical fixtures", async () => {
    for (const fixture of [invalidKeyFixture, rateLimitedFixture]) {
      const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(responseFor(fixture));
      const transport = new FetchTransport({
        endpoint,
        ingestKey: "public-key-example",
        fetch: fetchMock,
      });

      await expect(transport.sendBatch(fixtureEvents(acceptedFixture))).rejects.toMatchObject({
        status: fixture.expected.status,
        code: JSON.parse(fixture.expected.body).error.code,
      });
    }
  });

  it("maps non-JSON HTTP errors without leaking the parser error", async () => {
    const fetchMock = vi
      .fn<typeof fetch>()
      .mockResolvedValue(new Response("upstream failure", { status: 502 }));
    const transport = new FetchTransport({
      endpoint,
      ingestKey: "public-key-example",
      fetch: fetchMock,
    });

    await expect(transport.sendBatch(fixtureEvents(acceptedFixture))).rejects.toMatchObject({
      kind: "http",
      status: 502,
      code: undefined,
    });
  });

  it("maps fetch failures and does not retry", async () => {
    const failure = new Error("connection refused");
    const fetchMock = vi.fn<typeof fetch>().mockRejectedValue(failure);
    const transport = new FetchTransport({
      endpoint,
      ingestKey: "public-key-example",
      fetch: fetchMock,
    });

    await expect(transport.sendBatch(fixtureEvents(acceptedFixture))).rejects.toMatchObject({
      kind: "network",
      cause: failure,
      message: "Fetch request failed: connection refused",
    });
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it.each(["http://localhost:4001", "not-a-url", "ftp://localhost:4001/v1/events"])(
    "rejects incomplete or non-HTTP endpoint %s",
    (invalidEndpoint) => {
      expect(
        () => new FetchTransport({ endpoint: invalidEndpoint, ingestKey: "public-key-example" }),
      ).toThrow(TypeError);
    },
  );
});
