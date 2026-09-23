import { FetchTransport } from "@web-analytics/transport";

import type { Transport } from "@web-analytics/analytics-core";
import type { AnalyticsEvent } from "@web-analytics/protocol-ts";

export interface PlaygroundTransportConfig {
  mode?: string;
  endpoint?: string;
  ingestKey?: string;
}

export function createPlaygroundTransport(
  config: PlaygroundTransportConfig,
  onSend: (events: readonly AnalyticsEvent[]) => void,
): Transport {
  const mode = config.mode || "mock";
  if (mode === "mock") {
    return {
      async sendBatch(events) {
        onSend(events);
      },
    };
  }

  if (mode !== "fetch") {
    throw new Error(`Unsupported analytics transport '${mode}'`);
  }
  if (!config.endpoint || !config.ingestKey) {
    throw new Error(
      "FetchTransport requires NEXT_PUBLIC_ANALYTICS_ENDPOINT and NEXT_PUBLIC_ANALYTICS_INGEST_KEY",
    );
  }

  const fetchTransport = new FetchTransport({
    endpoint: config.endpoint,
    ingestKey: config.ingestKey,
  });

  return {
    async sendBatch(events, options) {
      await fetchTransport.sendBatch(events, options);
      onSend(events);
    },
  };
}
