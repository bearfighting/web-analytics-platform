export type NavigationType = "initial" | "push" | "replace" | "pop" | "unknown";

export interface NavigationEvent {
  url: string;
  path: string;
  title?: string;
  referrer?: string;
  navigationType: NavigationType;
  occurredAt: number;
}

export interface NavigationObserver {
  subscribe(listener: (event: NavigationEvent) => void): () => void;
}
