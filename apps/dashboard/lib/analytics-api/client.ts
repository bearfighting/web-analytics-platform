import { AnalyticsApiClientError } from "./errors";
import {
  DEFAULT_PAGES_LIMIT,
  isAnalyticsApiErrorResponse,
  isOverviewResponse,
  isPagesResponse,
  isRangeOverviewResponse,
  isTimelineResponse,
  overviewPath,
  pagesPath,
  rangeOverviewPath,
  timelinePath,
  type ResponseValidator,
} from "./queries";

import type {
  OverviewResponse,
  PagesResponse,
  RangeOverviewResponse,
  TimelineResponse,
} from "./types";

export interface AnalyticsApiClientOptions {
  baseUrl: string;
  fetch?: typeof globalThis.fetch;
}

export interface AnalyticsApiClient {
  overview(siteId: string): Promise<OverviewResponse>;
  rangeOverview(siteId: string, from: string, to: string): Promise<RangeOverviewResponse>;
  timeline(siteId: string, from: string, to: string): Promise<TimelineResponse>;
  pages(siteId: string, from: string, to: string, limit?: number): Promise<PagesResponse>;
}

const HTTP_ERROR_MESSAGE = "Analytics API failed to complete the request";
const RESPONSE_ERROR_MESSAGE = "Analytics API returned an invalid response";

export function createAnalyticsApiClient(options: AnalyticsApiClientOptions): AnalyticsApiClient {
  const baseUrl = normalizeBaseUrl(options.baseUrl);
  const injectedFetch = options.fetch ?? globalThis.fetch;

  return {
    overview: (siteId) => requestJson(`${baseUrl}${overviewPath(siteId)}`, isOverviewResponse),
    rangeOverview: (siteId, from, to) =>
      requestJson(`${baseUrl}${rangeOverviewPath(siteId, from, to)}`, isRangeOverviewResponse),
    timeline: (siteId, from, to) =>
      requestJson(`${baseUrl}${timelinePath(siteId, from, to)}`, isTimelineResponse),
    pages: (siteId, from, to, limit = DEFAULT_PAGES_LIMIT) =>
      requestJson(`${baseUrl}${pagesPath(siteId, from, to, limit)}`, isPagesResponse),
  };

  async function requestJson<T>(url: string, validate: ResponseValidator<T>): Promise<T> {
    let response: Response;

    try {
      response = await injectedFetch(url, {
        method: "GET",
        cache: "no-store",
      });
    } catch (cause) {
      throw new AnalyticsApiClientError("Analytics API request failed.", {
        kind: "network",
        cause,
      });
    }

    let body: unknown;
    try {
      body = await response.json();
    } catch (cause) {
      if (!response.ok) {
        throw new AnalyticsApiClientError(HTTP_ERROR_MESSAGE, {
          kind: "http",
          status: response.status,
          cause,
        });
      }

      throw new AnalyticsApiClientError(RESPONSE_ERROR_MESSAGE, {
        kind: "response",
        status: response.status,
        cause,
      });
    }

    if (!response.ok) {
      if (isAnalyticsApiErrorResponse(body)) {
        throw new AnalyticsApiClientError(body.error.message, {
          kind: "http",
          status: response.status,
          code: body.error.code,
        });
      }

      throw new AnalyticsApiClientError(HTTP_ERROR_MESSAGE, {
        kind: "http",
        status: response.status,
      });
    }

    if (!validate(body)) {
      throw new AnalyticsApiClientError(RESPONSE_ERROR_MESSAGE, {
        kind: "response",
        status: response.status,
      });
    }

    return body;
  }
}

function normalizeBaseUrl(value: string): string {
  let url: URL;

  try {
    url = new URL(value);
  } catch (cause) {
    throw new AnalyticsApiClientError("Analytics API baseUrl must be an absolute HTTP(S) URL.", {
      kind: "config",
      cause,
    });
  }

  if (!/^https?:$/.test(url.protocol) || url.search || url.hash) {
    throw new AnalyticsApiClientError("Analytics API baseUrl must be an absolute HTTP(S) URL.", {
      kind: "config",
    });
  }

  return url.toString().replace(/\/+$/, "");
}
