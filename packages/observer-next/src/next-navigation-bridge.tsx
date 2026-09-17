"use client";

import { usePathname, useSearchParams } from "next/navigation";
import { useEffect, useRef } from "react";

import { createNavigationEvent } from "./navigation-event";
import { createRouteIdentity, isHashOnlyChange, resolveNavigationType } from "./navigation-state";

import type { NavigationEvent, NavigationType } from "@web-analytics/observer-core";

export interface NextNavigationBridgeProps {
  onNavigation: (event: NavigationEvent) => void;
  now?: () => number;
}

export function NextNavigationBridge({ onNavigation, now = Date.now }: NextNavigationBridgeProps) {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const onNavigationRef = useRef(onNavigation);
  const nowRef = useRef(now);
  const pendingNavigationTypeRef = useRef<NavigationType>("unknown");
  const previousRouteIdentityRef = useRef<string | null>(null);

  onNavigationRef.current = onNavigation;
  nowRef.current = now;

  useEffect(() => {
    const history = window.history;
    const originalPushState = history.pushState;
    const originalReplaceState = history.replaceState;

    const wrappedPushState: History["pushState"] = function (this: History, data, unused, url) {
      pendingNavigationTypeRef.current = "push";

      return originalPushState.call(this, data, unused, url);
    };
    const wrappedReplaceState: History["replaceState"] = function (
      this: History,
      data,
      unused,
      url,
    ) {
      pendingNavigationTypeRef.current = "replace";

      return originalReplaceState.call(this, data, unused, url);
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
    onNavigationRef.current(event);
  }, [pathname, searchParams]);

  return null;
}
