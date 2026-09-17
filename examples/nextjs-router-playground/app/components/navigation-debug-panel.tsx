"use client";

import { NextNavigationBridge } from "@web-analytics/observer-next";
import { usePathname, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";

import type { NavigationEvent } from "@web-analytics/observer-core";

type NavigationLogEntry = NavigationEvent & {
  timestamp: string;
  previousUrl: string | null;
};

const MAX_LOG_ENTRIES = 20;

export function NavigationDebugPanel() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [hash, setHash] = useState("");
  const [documentTitle, setDocumentTitle] = useState("");
  const [currentUrl, setCurrentUrl] = useState("");
  const [logs, setLogs] = useState<NavigationLogEntry[]>([]);

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

  return (
    <>
      <NextNavigationBridge onNavigation={handleNavigation} />
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
          <ol>
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
      </section>
    </>
  );
}
