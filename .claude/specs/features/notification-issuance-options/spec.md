# Feature: Notification issuance options

**Status:** Specified — executing
**Size:** Medium (one use case + DTOs + one JSON-RPC method; design inline)
**Origin:** webapp#45 (DEN-22, P3 webapp issuance UI of `downstream-email-notifications`)

## Problem

`beginners.tokens.createNotification` needs a `senderTag`, an optional `tenantId` (required when
the service `recipientPolicy` is `tenantMembers`) and an `expiration` capped by
`serviceNotifications.maxConnectionStringTtlInDays`. None of this is visible to the issuer: the
services' notification blocks and the TTL cap live only in the gateway configuration, so a UI
cannot offer a service picker, know when a tenant is required, or cap the expiration.

## Decision (user, 2026-11-23)

- **D-01** Add a JSON-RPC method to the gateway (no REST) that returns, for the caller, the
  services they can issue notification strings for, plus the instance TTL cap.

## Requirements

- **NIO-01** DTOs `IssuableNotificationService { senderTag, displayName, recipientPolicy,
  tenantIds, allowsNoTenant }` and `NotificationIssuanceOptions {
  maxConnectionStringTtlInDays, services }` (camelCase, `ToSchema`).
- **NIO-02** Use case `list_notification_issuance_options(profile, notifications_settings,
  service_read_repo)` in `core/src/use_cases/role_scoped/beginner/token/`:
  - lists the services with a notifications block;
  - `tenantIds`: tenants from the caller's licensed resources for which
    `check_notification_issuer(profile, Some(tenant), config)` passes (the same rule
    issuance applies);
  - `allowsNoTenant`: `recipientPolicy != tenantMembers` and
    `check_notification_issuer(profile, None, config)` passes;
  - a service is returned only when it allows no tenant or has at least one tenant;
  - sorted by `displayName`; `templates`, `issuerRoles`, quotas and domains are not exposed.
- **NIO-03** JSON-RPC `beginners.tokens.listNotificationServices` (no params) returning
  `NotificationIssuanceOptions`; documented in the OpenRPC spec (test) and in the book
  (chapter 14 method table, chapter 6 issuance paragraph) + pt-BR po.
- **NIO-04** Unit tests: service without notifications skipped; issuer role on a tenant →
  that tenant listed; `tenantMembers` → `allowsNoTenant = false`; `issuerTenants` restricts
  the tenants; caller without issuer role → service omitted; inactive account → no services;
  TTL cap echoed.

## Verification

Full gateway gate (fmt, workspace build, workspace tests, standalone and postgres-only builds);
call the method on the standalone test instance.
