import type { NavigationEvent, NavigationEventSink, NavigationObserver } from "./types";

export class MemoryNavigationObserver implements NavigationEventSink, NavigationObserver {
  private readonly listeners = new Set<(event: NavigationEvent) => void>();

  subscribe(listener: (event: NavigationEvent) => void) {
    this.listeners.add(listener);

    return () => {
      this.listeners.delete(listener);
    };
  }

  emit(event: NavigationEvent) {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch {
        // An observer must not let one listener break the remaining listeners.
      }
    }
  }
}
