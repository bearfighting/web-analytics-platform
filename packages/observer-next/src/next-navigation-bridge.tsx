"use client";

import {
  createNavigationEvent,
  createRouteIdentity,
  isHashOnlyChange,
  isHashOnlyUrlChange,
  resolveNavigationType,
} from "@web-analytics/observer-core";
import { usePathname, useSearchParams } from "next/navigation";
import { useEffect, useRef } from "react";

import type {
  NavigationEvent,
  NavigationEventSink,
  NavigationType,
} from "@web-analytics/observer-core";

type NextNavigationBridgeOutput =
  | {
      observer?: NavigationEventSink;
      onNavigation: (event: NavigationEvent) => void;
    }
  | {
      observer: NavigationEventSink;
      onNavigation?: never;
    };

export type NextNavigationBridgeProps = NextNavigationBridgeOutput & {
  now?: () => number;
};

export function NextNavigationBridge({
  observer,
  onNavigation,
  now = Date.now,
}: NextNavigationBridgeProps) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const observerRef = useRef(observer);
  const onNavigationRef = useRef(onNavigation);
  const nowRef = useRef(now);
  const pendingNavigationTypeRef = useRef<NavigationType>("unknown");
  const previousRouteIdentityRef = useRef<string | null>(null);

  observerRef.current = observer;
  onNavigationRef.current = onNavigation;
  nowRef.current = now;

  useEffect(() => {
    const history = window.history;
    const originalPushState = history.pushState;
    const originalReplaceState = history.replaceState;

    const wrappedPushState: History["pushState"] = function (this: History, data, unused, url) {
      const hashOnly = isHashOnlyUrlChange(window.location.href, url);

      try {
        const result = originalPushState.call(this, data, unused, url);
        pendingNavigationTypeRef.current = hashOnly ? "unknown" : "push";

        return result;
      } catch (error) {
        pendingNavigationTypeRef.current = "unknown";

        throw error;
      }
    };
    const wrappedReplaceState: History["replaceState"] = function (
      this: History,
      data,
      unused,
      url,
    ) {
      const hashOnly = isHashOnlyUrlChange(window.location.href, url);

      try {
        const result = originalReplaceState.call(this, data, unused, url);
        pendingNavigationTypeRef.current = hashOnly ? "unknown" : "replace";

        return result;
      } catch (error) {
        pendingNavigationTypeRef.current = "unknown";

        throw error;
      }
    };
    const handlePopState = () => {
      pendingNavigationTypeRef.current = "pop";
    };

    history.pushState = wrappedPushState;
    history.replaceState = wrappedReplaceState;
    window.addEventListener("popstate", handlePopState);

    return () => {
      if (history.pushState === wrappedPushState) {
        history.pushState = originalPushState;
      }
      if (history.replaceState === wrappedReplaceState) {
        history.replaceState = originalReplaceState;
      }
      window.removeEventListener("popstate", handlePopState);
    };
  }, []);

  useEffect(() => {
    const routeIdentity = createRouteIdentity(pathname, searchParams.toString());
    const previousRouteIdentity = previousRouteIdentityRef.current;

    if (previousRouteIdentity !== null && isHashOnlyChange(previousRouteIdentity, routeIdentity)) {
      pendingNavigationTypeRef.current = "unknown";

      return;
    }

    const navigationType = resolveNavigationType(
      previousRouteIdentity !== null,
      pendingNavigationTypeRef.current,
    );
    const event = createNavigationEvent({
      url: window.location.href,
      path: pathname,
      title: document.title || undefined,
      referrer: document.referrer || undefined,
      navigationType,
      occurredAt: nowRef.current(),
    });

    previousRouteIdentityRef.current = routeIdentity;
    pendingNavigationTypeRef.current = "unknown";
    try {
      onNavigationRef.current?.(event);
    } finally {
      observerRef.current?.emit(event);
    }
  }, [pathname, searchParams]);

  return null;
}
