# 005 — Creating a tenant management account fails on SQLite

**Type:** bug fix (quick mode)

## Symptom

Webapp → Namespace → Managers → "Create management account" does nothing. The gateway logs:

```
tenantOwner.accounts.createManagementAccount →
Database error: NOT NULL constraint failed: manager_account_on_tenant.created
```

## Cause

`AccountRegistrationSqlDbRepository::get_or_create_tenant_management_account`
(`adapters/diesel_sqlite/src/repositories/account/account_registration.rs`) inserts the
`manager_account_on_tenant` link without `created`. The SQLite schema declares the column
`NOT NULL` with no default; Postgres declares `DEFAULT now()`, so only the standalone (SQLite)
mode is affected. The transaction rolls back, so the account is not created either.

## Fix

Insert `created` with the new account's own `created` timestamp.

## Verification

- New test `creates_the_tenant_management_account` (SQLite temp DB): fails before the fix with
  the same NOT NULL error, passes after.
- Full gateway gate.
