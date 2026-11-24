# Feature Spec: Retry policy when routing to downstream services (#137)

**Scope:** Gateway router
**Status:** In progress

## Problem

The router sends each request to the downstream service once; a transient
failure (connection refused/reset, 502/503/504) reaches the client
immediately.

## Requirements

- **RRP-01** Opt-in per route: `retry = { maxAttempts, baseDelayMs,
  maxDelayMs, retryOnStatus, retryNonIdempotent }` in the route config.
  Routes without it behave exactly as today (single attempt, streamed body).
- **RRP-02** Quadratic back-off between attempts:
  `delay(n) = min(baseDelayMs * n², maxDelayMs)` for the n-th retry.
- **RRP-03** Retry on connection failures (refused, reset, timeout) and on
  the configured response statuses (default 502, 503, 504). Never on other
  client/server errors.
- **RRP-04** Only idempotent methods are retried (GET, HEAD, OPTIONS, PUT,
  DELETE, TRACE) unless the route sets `retryNonIdempotent = true`.
- **RRP-05** The body must be replayable: on retry-enabled routes the
  request body is buffered (same 512 KB limit as body identity routes).
- **RRP-06** Defaults: 3 attempts in total, 100 ms base, 2 s max delay;
  `maxAttempts` capped at 10. Each retry is logged at `warn` with the
  attempt number and reason.
