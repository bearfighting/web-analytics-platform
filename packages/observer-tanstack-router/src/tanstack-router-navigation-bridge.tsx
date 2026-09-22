"use client";

import { useRouter, useRouterState } from "@tanstack/react-router";
import {
  createNavigationEvent,
  createRouteIdentity,
  mapNavigationType,
  type NavigationEvent,
  type NavigationEventSink,
  type NavigationType,
} from "@web-analytics/observer-core";
import { useEffect, useRef } from "react";

import { subscribeToHistoryNavigation } from "./history-patch";

interface PendingNavigationType {
  navigationType: NavigationType;
  routeIdentity: string;
}

type BridgeOutput =
  | { observer?: NavigationEventSink; onNavigation: (event: NavigationEvent) => void }
  | { observer: NavigationEventSink; onNavigation?: never };

export type TanStackRouterNavigationBridgeProps = BridgeOutput & { now?: () => number };

export function TanStackRouterNavigationBridge({
  observer,
  onNavigation,
  now = Date.now,
}: TanStackRouterNavigationBridgeProps) {
  const router = useRouter();
  const location = useRouterState({ select: (state) => state.location });
  const observerRef = useRef(observer);
  const onNavigationRef = useRef(onNavigation);
  const nowRef = useRef(now);
  const pendingNavigationTypesRef = useRef<PendingNavigationType[]>([]);
  const previousRouteIdentityRef = useRef<string | null>(null);

  observerRef.current = observer;
  onNavigationRef.current = onNavigation;
  nowRef.current = now;

  useEffect(() => {
    return subscribeToHistoryNavigation((navigationType, href) => {
      const url = new URL(href, window.location.href);
      pendingNavigationTypesRef.current.push({
        navigationType,
        routeIdentity: createRouteIdentity(url.pathname, url.search),
      });
    });
  }, []);

  useEffect(() => {
    const initialLocation = location;
    const pendingEmissions = new Set<ReturnType<typeof setTimeout>>();
    const emitLocation = (nextLocation: typeof initialLocation, initial = false) => {
      const routeIdentity = createRouteIdentity(nextLocation.pathname, nextLocation.searchStr);
      const previousRouteIdentity = previousRouteIdentityRef.current;
      if (previousRouteIdentity !== null && previousRouteIdentity === routeIdentity) {
        discardPendingNavigationTypes(pendingNavigationTypesRef.current, routeIdentity);

        return;
      }

      const navigationType = initial
        ? "initial"
        : mapNavigationType(
            takePendingNavigationType(pendingNavigationTypesRef.current, routeIdentity),
          );

      const event = createNavigationEvent({
        url: nextLocation.href,
        path: nextLocation.pathname,
        title: typeof document === "undefined" ? undefined : document.title || undefined,
        referrer: typeof document === "undefined" ? undefined : document.referrer || undefined,
        navigationType,
        occurredAt: nowRef.current(),
      });
      previousRouteIdentityRef.current = routeIdentity;
      try {
        onNavigationRef.current?.(event);
      } finally {
        observerRef.current?.emit(event);
      }
    };

    const unsubscribe = router.subscribe("onResolved", ({ toLocation }) => {
      const timer = setTimeout(() => {
        pendingEmissions.delete(timer);
        emitLocation(toLocation, previousRouteIdentityRef.current === null);
      }, 0);
      pendingEmissions.add(timer);
    });

    if (router.state.status === "idle") {
      emitLocation(router.state.resolvedLocation ?? initialLocation, true);
    }

    return () => {
      unsubscribe();
      pendingEmissions.forEach((timer) => clearTimeout(timer));
      pendingEmissions.clear();
      pendingNavigationTypesRef.current = [];
    };
  }, [router]);

  return null;
}

function takePendingNavigationType(
  pendingNavigationTypes: PendingNavigationType[],
  routeIdentity: string,
): NavigationType | undefined {
  const matchingIndex = pendingNavigationTypes.findIndex(
    (pending) => pending.routeIdentity === routeIdentity,
  );
  if (matchingIndex === -1) return undefined;

  const [matching] = pendingNavigationTypes.splice(matchingIndex, 1);
  discardPendingNavigationTypes(pendingNavigationTypes, routeIdentity);

  return matching.navigationType;
}

function discardPendingNavigationTypes(
  pendingNavigationTypes: PendingNavigationType[],
  routeIdentity: string,
) {
  for (let index = pendingNavigationTypes.length - 1; index >= 0; index -= 1) {
    if (pendingNavigationTypes[index].routeIdentity === routeIdentity) {
      pendingNavigationTypes.splice(index, 1);
    }
  }
}
