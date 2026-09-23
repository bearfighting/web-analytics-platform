"use client";

import { createAnalytics } from "@web-analytics/analytics-browser";
import { createPlaygroundTransport } from "@web-analytics/playground-support";
import { RouterAnalyticsBridge } from "@web-analytics/router-adapters/next";
import { usePathname, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";

import type { Analytics } from "@web-analytics/analytics-browser";
import type { Transport } from "@web-analytics/analytics-core";
import type { NavigationEvent } from "@web-analytics/observer-core";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";

type NavigationLogEntry = NavigationEvent & {
  timestamp: string;
  previousUrl: string | null;
};

const MAX_LOG_ENTRIES = 20;

interface WorkflowState {
  bufferedEvents: number;
  sentBatches: number;
  sentEvents: number;
  lastBatchSize: number | null;
  lastSentAt: string | null;
  lastError: string | null;
}

const initialWorkflowState: WorkflowState = {
  bufferedEvents: 0,
  sentBatches: 0,
  sentEvents: 0,
  lastBatchSize: null,
  lastSentAt: null,
  lastError: null,
};

export function NavigationDebugPanel() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [hash, setHash] = useState("");
  const [documentTitle, setDocumentTitle] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [logs, setLogs] = useState<NavigationLogEntry[]>([]);
  const [workflow, setWorkflow] = useState(initialWorkflowState);
  const [analytics, setAnalytics] = useState<Analytics | null>(null);

  const handleNavigation = useCallback((event: NavigationEvent) => {
    setCurrentUrl(event.url);
    setDocumentTitle(event.title ?? "");
    setLogs((currentLogs) => {
      const previousUrl = currentLogs[0]?.url ?? null;

      return [
        {
          ...event,
          timestamp: new Date(event.occurredAt).toISOString(),
          previousUrl,
        },
        ...currentLogs,
      ].slice(0, MAX_LOG_ENTRIES);
    });
  }, []);

  useEffect(() => {
    const updateHash = () => {
      setHash(window.location.hash);
      setCurrentUrl(window.location.href);
      setDocumentTitle(document.title);
    };

    updateHash();
    window.addEventListener("hashchange", updateHash);

    return () => window.removeEventListener("hashchange", updateHash);
  }, []);

  useEffect(() => {
    const onSend = (events: readonly AnalyticsEvent[]) => {
      setWorkflow((current) => ({
        ...current,
        sentBatches: current.sentBatches + 1,
        sentEvents: current.sentEvents + events.length,
        lastBatchSize: events.length,
        lastSentAt: new Date().toISOString(),
      }));
    };

    let transport: Transport;
    try {
      transport = createPlaygroundTransport(
        {
          mode: process.env.NEXT_PUBLIC_ANALYTICS_TRANSPORT,
          endpoint: process.env.NEXT_PUBLIC_ANALYTICS_ENDPOINT,
          ingestKey: process.env.NEXT_PUBLIC_ANALYTICS_INGEST_KEY,
        },
        onSend,
      );
    } catch (error) {
      setWorkflow((current) => ({
        ...current,
        lastError: error instanceof Error ? error.message : String(error),
      }));

      return;
    }

    const analytics = createAnalytics({
      consent: "granted",
      siteId: process.env.NEXT_PUBLIC_ANALYTICS_SITE_ID || "site_playground",
      transport,
      onBufferChange: (bufferedEvents) =>
        setWorkflow((current) => ({ ...current, bufferedEvents })),
      onError: (error) =>
        setWorkflow((current) => ({
          ...current,
          lastError: error instanceof Error ? error.message : String(error),
        })),
    });
    setAnalytics(analytics);

    return () => {
      analytics.destroy();
      setAnalytics(null);
    };
  }, []);

  return (
    <>
      {analytics ? (
        <RouterAnalyticsBridge analytics={analytics} onNavigation={handleNavigation} />
      ) : null}
      <section aria-label="Navigation debug panel">
        <h2>Navigation Debug Panel</h2>
        <dl>
          <dt>Current URL</dt>
          <dd>{currentUrl || "Loading..."}</dd>
          <dt>Pathname</dt>
          <dd>{pathname}</dd>
          <dt>Search params</dt>
          <dd>{searchParams.toString() || "(none)"}</dd>
          <dt>Hash</dt>
          <dd>{hash || "(none)"}</dd>
          <dt>Document title</dt>
          <dd>{documentTitle || "Loading..."}</dd>
        </dl>

        <h3>Navigation Log</h3>
        {logs.length === 0 ? (
          <p>No navigation recorded yet.</p>
        ) : (
          <ol data-testid="events">
            {logs.map((entry, index) => (
              <li key={`${entry.timestamp}-${index}`}>
                <time dateTime={entry.timestamp}>{entry.timestamp}</time>{" "}
                <strong>{entry.navigationType}</strong>
                <div>Path: {entry.path}</div>
                <div>From: {entry.previousUrl || "(none)"}</div>
                <div>To: {entry.url}</div>
              </li>
            ))}
          </ol>
        )}
        <output data-testid="events-json">{JSON.stringify(logs)}</output>
      </section>
      <section aria-label="SDK workflow debug panel">
        <h2>SDK Workflow Debug Panel</h2>
        <dl>
          <dt>Buffered events</dt>
          <dd>{workflow.bufferedEvents}</dd>
          <dt>Sent batches</dt>
          <dd>{workflow.sentBatches}</dd>
          <dt>Sent events</dt>
          <dd>{workflow.sentEvents}</dd>
          <output data-testid="analytics-events">{workflow.sentEvents}</output>
          <dt>Last batch size</dt>
          <dd>{workflow.lastBatchSize ?? "(none)"}</dd>
          <dt>Last sent at</dt>
          <dd>{workflow.lastSentAt ?? "(none)"}</dd>
          <dt>Last error</dt>
          <dd>{workflow.lastError ?? "(none)"}</dd>
        </dl>
      </section>
    </>
  );
}
