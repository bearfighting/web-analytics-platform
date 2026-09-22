import { isHashOnlyUrlChange } from "@web-analytics/observer-core";

import type { NavigationType } from "@web-analytics/observer-core";

type NavigationTypeListener = (navigationType: NavigationType, href: string) => void;

interface HistoryPatch {
  history: History;
  originalPushState: History["pushState"];
  originalReplaceState: History["replaceState"];
  wrappedPushState: History["pushState"];
  wrappedReplaceState: History["replaceState"];
  handlePopState: () => void;
  listeners: Set<NavigationTypeListener>;
}

let activePatch: HistoryPatch | null = null;

export function subscribeToHistoryNavigation(listener: NavigationTypeListener): () => void {
  if (typeof window === "undefined") return () => {};

  const patch = activePatch ?? installHistoryPatch(window.history);
  patch.listeners.add(listener);

  return () => {
    if (!patch.listeners.delete(listener) || patch.listeners.size > 0) return;

    uninstallHistoryPatch(patch);
    if (activePatch === patch) activePatch = null;
  };
}

function installHistoryPatch(history: History): HistoryPatch {
  const listeners = new Set<NavigationTypeListener>();
  const originalPushState = history.pushState;
  const originalReplaceState = history.replaceState;
  const notify = (navigationType: NavigationType) => {
    listeners.forEach((listener) => listener(navigationType, window.location.href));
  };
  const wrappedPushState: History["pushState"] = function (this: History, data, unused, url) {
    const hashOnly = isHashOnlyUrlChange(window.location.href, url);
    const result = originalPushState.call(this, data, unused, url);
    notify(hashOnly ? "unknown" : "push");

    return result;
  };
  const wrappedReplaceState: History["replaceState"] = function (this: History, data, unused, url) {
    const hashOnly = isHashOnlyUrlChange(window.location.href, url);
    const result = originalReplaceState.call(this, data, unused, url);
    notify(hashOnly ? "unknown" : "replace");

    return result;
  };
  const handlePopState = () => notify("pop");
  const patch: HistoryPatch = {
    history,
    originalPushState,
    originalReplaceState,
    wrappedPushState,
    wrappedReplaceState,
    handlePopState,
    listeners,
  };

  history.pushState = wrappedPushState;
  history.replaceState = wrappedReplaceState;
  window.addEventListener("popstate", handlePopState, true);
  activePatch = patch;

  return patch;
}

function uninstallHistoryPatch(patch: HistoryPatch) {
  if (patch.history.pushState === patch.wrappedPushState) {
    patch.history.pushState = patch.originalPushState;
  }
  if (patch.history.replaceState === patch.wrappedReplaceState) {
    patch.history.replaceState = patch.originalReplaceState;
  }
  window.removeEventListener("popstate", patch.handlePopState, true);
}
