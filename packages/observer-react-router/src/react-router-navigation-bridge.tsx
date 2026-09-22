"use client";

import {
  createNavigationEvent,
  createRouteIdentity,
  mapNavigationType,
  type NavigationEvent,
  type NavigationEventSink,
} from "@web-analytics/observer-core";
import { useEffect, useRef } from "react";
import { useLocation, useNavigationType } from "react-router-dom";

type BridgeOutput =
  | { observer?: NavigationEventSink; onNavigation: (event: NavigationEvent) => void }
  | { observer: NavigationEventSink; onNavigation?: never };

export type ReactRouterNavigationBridgeProps = BridgeOutput & { now?: () => number };

export function ReactRouterNavigationBridge({
  observer,
  onNavigation,
  now = Date.now,
}: ReactRouterNavigationBridgeProps) {
  const location = useLocation();
  const navigationType = useNavigationType();
  const observerRef = useRef(observer);
  const onNavigationRef = useRef(onNavigation);
  const nowRef = useRef(now);
  const previousRouteIdentityRef = useRef<string | null>(null);

  observerRef.current = observer;
  onNavigationRef.current = onNavigation;
  nowRef.current = now;

  useEffect(() => {
    const routeIdentity = createRouteIdentity(location.pathname, location.search);
    const previousRouteIdentity = previousRouteIdentityRef.current;
    if (previousRouteIdentity !== null && previousRouteIdentity === routeIdentity) return;

    const event = createNavigationEvent({
      url: window.location.href,
      path: location.pathname,
      title: document.title || undefined,
      referrer: document.referrer || undefined,
      navigationType:
        previousRouteIdentity === null ? "initial" : mapNavigationType(navigationType),
      occurredAt: nowRef.current(),
    });
    previousRouteIdentityRef.current = routeIdentity;
    try {
      onNavigationRef.current?.(event);
    } finally {
      observerRef.current?.emit(event);
    }
  }, [location.pathname, location.search, location.hash, navigationType]);

  return null;
}
