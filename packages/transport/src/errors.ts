export type FetchTransportErrorKind = "network" | "http" | "collector";

export interface FetchTransportErrorOptions {
  kind: FetchTransportErrorKind;
  status?: number;
  code?: string;
  retryAfter?: string;
  cause?: unknown;
}

export class FetchTransportError extends Error {
  readonly kind: FetchTransportErrorKind;
  readonly status?: number;
  readonly code?: string;
  readonly retryAfter?: string;

  constructor(message: string, options: FetchTransportErrorOptions) {
    super(message, { cause: options.cause });
    this.name = "FetchTransportError";
    this.kind = options.kind;
    this.status = options.status;
    this.code = options.code;
    this.retryAfter = options.retryAfter;
  }
}
