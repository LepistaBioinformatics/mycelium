# Feature Spec: Webhook execution events visible to super users (#113)

**Scope:** Gateway (core, adapters, REST + JSON-RPC) and webapp screen
**Status:** In progress

## Problem

Webhook dispatching writes one `webhook_execution` row per event (trigger,
attempts, status, propagations), but nobody can see them without database
access. Super users need to check the dispatcher's health.

## Requirements

- **WEE-01** List execution events, newest first, paginated, filtered by
  trigger and status. Fields: id, trigger, payload id, status, attempts,
  created, last attempt, number of propagations, number of failed
  propagations, last response status. Never the payload, the propagation
  URLs or response bodies.
- **WEE-02** Summary aggregated by trigger and status (count, latest event),
  optionally since a timestamp, without loading every row.
- **WEE-03** Same access rule as webhook management: system account with
  the `SystemManager` role (read), staff and managers included as today.
- **WEE-04** Sensitive columns are hidden in the database itself: both
  queries read a `webhook_execution_event` view that exposes only the
  fields above (Postgres + SQLite).
- **WEE-05** REST: `GET /_adm/system-manager/webhooks/executions`
  (`trigger`, `status`, `skip`, `pageSize`) and
  `GET /_adm/system-manager/webhooks/executions/summary` (`since`).
  JSON-RPC: `systemManager.webhooks.listExecutions`,
  `systemManager.webhooks.summarizeExecutions`.
- **WEE-06** Webapp: an "Execution events" view on the Webhooks screen with
  the per-trigger summary and the paginated, filterable event list
  (skeletons while loading, empty state).

## Out of scope

Retrying or deleting events from the UI; showing payloads.
