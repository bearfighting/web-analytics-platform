"use client";

import { usePathname, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

type NavigationKind = "initial" | "push" | "replace" | "pop" | "hash" | "unknown";

type NavigationLogEntry = {
  timestamp: string;
  previousUrl: string | null;
  currentUrl: string;
  detectedChange: NavigationKind;
};

const MAX_LOG_ENTRIES = 20;

function getCurrentUrl() {
  return window.location.href;
}

export function NavigationDebugPanel() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const previousUrlRef = useRef<string | null>(null);
  const navigationKindRef = useRef<NavigationKind>("unknown");
  const [currentUrl, setCurrentUrl] = useState("");
  const [hash, setHash] = useState("");
  const [documentTitle, setDocumentTitle] = useState("");
  const [logs, setLogs] = useState<NavigationLogEntry[]>([]);

  const recordNavigation = useCallback((detectedChange: NavigationKind, nextUrl: string) => {
    const previousUrl = previousUrlRef.current;

    if (previousUrl === nextUrl) {
      return;
    }

    previousUrlRef.current = nextUrl;
    setCurrentUrl(nextUrl);
    setLogs((currentLogs) =>
      [
        {
          timestamp: new Date().toISOString(),
          previousUrl,
          currentUrl: nextUrl,
          detectedChange,
        },
        ...currentLogs,
      ].slice(0, MAX_LOG_ENTRIES),
    );
  }, []);

  useEffect(() => {
    const originalPushState = window.history.pushState;
    const originalReplaceState = window.history.replaceState;

    window.history.pushState = (...args) => {
      navigationKindRef.current = "push";

      return originalPushState.apply(window.history, args);
    };

    window.history.replaceState = (...args) => {
      navigationKindRef.current = "replace";

      return originalReplaceState.apply(window.history, args);
    };

    const handlePopState = () => {
      navigationKindRef.current = "pop";
    };

    const handleHashChange = () => {
      setHash(window.location.hash);
      recordNavigation("hash", getCurrentUrl());
    };

    window.addEventListener("popstate", handlePopState);
    window.addEventListener("hashchange", handleHashChange);
    setHash(window.location.hash);
    setDocumentTitle(document.title);

    return () => {
      window.history.pushState = originalPushState;
      window.history.replaceState = originalReplaceState;
      window.removeEventListener("popstate", handlePopState);
      window.removeEventListener("hashchange", handleHashChange);
    };
  }, [recordNavigation]);

  useEffect(() => {
    const nextUrl = getCurrentUrl();
    const detectedChange = previousUrlRef.current ? navigationKindRef.current : "initial";

    recordNavigation(detectedChange, nextUrl);
    navigationKindRef.current = "unknown";
  }, [pathname, recordNavigation, searchParams]);

  return (
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
              <strong>{entry.detectedChange}</strong>
              <div>From: {entry.previousUrl || "(none)"}</div>
              <div>To: {entry.currentUrl}</div>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}
