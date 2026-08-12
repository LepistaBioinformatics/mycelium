-- =============================================================================
-- Mycelium -- complete Postgres schema (9.0.0)
-- =============================================================================
--
-- This file is the ONE-SHOT installer: it creates the database, the role/user,
-- and the entire schema in a single psql invocation. Every migration under
-- `sql/migrations/` is already folded in here.
--
--   psql postgres://postgres:postgres@localhost:5432/postgres \
--        -f up.sql \
--        -v db_password='myc-password'
--
-- Optional variables (defaults shown): -v db_name='mycelium-dev'
--                                      -v db_user='mycelium-user'
--                                      -v db_role='service-role-mycelium'
--
-- FRESH INSTALLS ONLY. Re-running against a database that already has the
-- schema exits early without touching anything (see GUARD below). To upgrade an
-- existing 9.0.0-rc.x database, apply `sql/migrations/*.sql` in chronological
-- order instead.
--
-- MAINTAINERS: a new migration goes in `sql/migrations/` *and* is folded into
-- this file in the same commit, so both paths stay equivalent.
--
-- Structure:
--   Phase A (no transaction) -- CREATE DATABASE cannot run inside one, and \c
--                               reconnects. Ends with the already-installed guard.
--   Phase B (BEGIN/COMMIT)   -- all DDL, so a failure leaves nothing behind.
-- =============================================================================


-- #############################################################################
-- PHASE A -- database creation and guard (not transactional)
-- #############################################################################

--------------------------------------------------------------------------------
-- EXTERNAL VALUES
--------------------------------------------------------------------------------

-- Check if the db_password variable is provided
\if :{?db_password}
    \echo "Using the provided password."
\else
    \echo "ERROR: The db_password variable is required. Use -v db_password='your_password' when executing."
    \quit
\endif

-- Set default values only if variables were not provided via -v
\if :{?db_name}
    \echo "Using provided database name: :'db_name'"
\else
    \set db_name 'mycelium-dev'
    \echo "Using default database name: :'db_name'"
\endif

\if :{?db_user}
    \echo "Using provided database user: :'db_user'"
\else
    \set db_user 'mycelium-user'
    \echo "Using default database user: :'db_user'"
\endif

\if :{?db_role}
    \echo "Using provided database role: :'db_role'"
\else
    \set db_role 'service-role-mycelium'
    \echo "Using default database role: :'db_role'"
\endif

--------------------------------------------------------------------------------
-- DATABASE
--
-- Create database if it doesn't exist
--
--------------------------------------------------------------------------------

SELECT 'CREATE DATABASE "' || :'db_name' || '"'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'db_name')\gexec

\c :"db_name"

--------------------------------------------------------------------------------
-- GUARD
--
-- Postgres has no `ALTER TABLE ... ADD CONSTRAINT IF NOT EXISTS`, so the ~40
-- constraint statements below cannot be made individually re-runnable without
-- wrapping each in a DO block. Probing for one table the schema cannot exist
-- without achieves the same thing in three lines: a second run is a clean
-- no-op with a message instead of a cascade of "already exists" errors.
--
--------------------------------------------------------------------------------

SELECT (to_regclass('public.account') IS NOT NULL) AS schema_already_installed \gset

\if :schema_already_installed
    \echo ''
    \echo '>> SKIP: the Mycelium schema is already installed in this database.'
    \echo '>>       up.sql only performs fresh installs. To upgrade an existing'
    \echo '>>       database, apply sql/migrations/*.sql in chronological order.'
    \echo ''
    \quit
\endif


-- #############################################################################
-- PHASE B -- schema (all-or-nothing)
-- #############################################################################

BEGIN;

--------------------------------------------------------------------------------
-- ROLES
--
-- Roles are cluster-global, not per-database: an unguarded CREATE ROLE fails
-- when a second Mycelium database is installed on the same Postgres cluster.
-- The GUARD above does not cover that case -- there the role exists but this
-- database's schema does not.
--
--------------------------------------------------------------------------------

SELECT NOT EXISTS (
    SELECT FROM pg_roles WHERE rolname = :'db_role'
) AS must_create_role \gset

\if :must_create_role
    CREATE ROLE :"db_role";
\else
    \echo 'Role' :'db_role' 'already exists -- reusing it.'
\endif

-- An existing user keeps its current password: silently rotating a credential
-- the operator did not ask to rotate is worse than doing nothing.
SELECT NOT EXISTS (
    SELECT FROM pg_roles WHERE rolname = :'db_user'
) AS must_create_user \gset

\if :must_create_user
    CREATE USER :"db_user" WITH PASSWORD :'db_password';
\else
    \echo 'User' :'db_user' 'already exists -- password left unchanged.'
\endif

-- Idempotent: re-granting an existing membership is a no-op.
GRANT :"db_role" TO :"db_user";

--------------------------------------------------------------------------------
-- Create extension for UUID generation
--
-- Extension is used to generate UUIDs for tables that require them
--
--------------------------------------------------------------------------------

CREATE EXTENSION IF NOT EXISTS pgcrypto;

--------------------------------------------------------------------------------
-- TABLES
--------------------------------------------------------------------------------

-- Tenant table
--
-- encrypted_dek/kek_version implement envelope encryption: a per-tenant
-- data-encryption key (DEK) wrapped by the KEK. encrypted_dek is NULL until
-- first use -- the implementation provisions it lazily via get_or_provision_dek.
-- kek_version tracks which KEK generation wrapped the DEK; increment it after a
-- KEK rotation and run `myc-cli rotate-kek`. See migration 20260421_01.
--
-- Column order matters: the migration appends these two, so they must stay last
-- for a folded install to match a migrated one.
CREATE TABLE tenant (
    id UUID DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    meta JSONB,
    status JSONB[],
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    encrypted_dek TEXT,
    kek_version INTEGER NOT NULL DEFAULT 1
);

-- Account table
CREATE TABLE account (
    id UUID DEFAULT gen_random_uuid(),
    name VARCHAR(256) NOT NULL,
    slug VARCHAR(256) NOT NULL,
    meta JSONB,
    account_type JSONB DEFAULT '{}'::JSONB,
    created TIMESTAMPTZ DEFAULT now(),
    created_by JSONB DEFAULT '{}'::JSONB,
    updated TIMESTAMPTZ DEFAULT NULL,
    updated_by JSONB DEFAULT '{}'::JSONB,
    is_active BOOLEAN DEFAULT TRUE,
    is_checked BOOLEAN DEFAULT FALSE,
    is_archived BOOLEAN DEFAULT FALSE,
    is_deleted BOOLEAN DEFAULT FALSE,
    is_default BOOLEAN DEFAULT FALSE,
    tenant_id UUID DEFAULT NULL
);

-- Account tag table
CREATE TABLE account_tag (
    id UUID DEFAULT gen_random_uuid(),
    value VARCHAR(64) NOT NULL,
    meta JSONB,
    account_id UUID NOT NULL
);

-- Public user table
CREATE TABLE public.user (
    id UUID DEFAULT gen_random_uuid(),
    username VARCHAR(140) NOT NULL,
    email VARCHAR(140) NOT NULL,
    first_name VARCHAR(140) NOT NULL,
    last_name VARCHAR(140) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    is_principal BOOLEAN DEFAULT FALSE,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    mfa JSONB,
    account_id UUID DEFAULT NULL
);

-- Identity provider table
CREATE TABLE identity_provider (
    user_id UUID,
    name VARCHAR(255) DEFAULT NULL,
    password_hash VARCHAR(255) DEFAULT NULL
);

-- Owner on tenant table
CREATE TABLE owner_on_tenant (
    id UUID DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    owner_id UUID NOT NULL,
    guest_by VARCHAR NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);

-- Manager account on tenant table
CREATE TABLE manager_account_on_tenant (
    id UUID DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    account_id UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);

-- Tenant tag table
CREATE TABLE tenant_tag (
    id UUID DEFAULT gen_random_uuid(),
    value VARCHAR(64) NOT NULL,
    meta JSONB,
    tenant_id UUID NOT NULL
);

-- Guest role table
CREATE TABLE guest_role (
    id UUID DEFAULT gen_random_uuid(),
    name VARCHAR(140) NOT NULL,
    slug VARCHAR(140) NOT NULL,
    description VARCHAR(255),
    permission INT DEFAULT 0,
    system BOOLEAN DEFAULT FALSE NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);

-- Guest role children table
CREATE TABLE guest_role_children (
    parent_id UUID NOT NULL,
    child_role_id UUID NOT NULL,
    created_by UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);

-- Guest user table
CREATE TABLE guest_user (
    id UUID DEFAULT gen_random_uuid(),
    email VARCHAR NOT NULL,
    guest_role_id UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    was_verified BOOLEAN DEFAULT FALSE
);

-- Guest user on account table
CREATE TABLE guest_user_on_account (
    guest_user_id UUID NOT NULL,
    account_id UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    permit_flags JSONB[],
    deny_flags JSONB[]
);

-- Error code table
CREATE TABLE error_code (
    code SERIAL NOT NULL,
    prefix VARCHAR NOT NULL,
    message VARCHAR(255) NOT NULL,
    details VARCHAR(255),
    is_internal BOOLEAN DEFAULT FALSE,
    is_native BOOLEAN DEFAULT FALSE
);

-- Webhook table
CREATE TABLE webhook (
    id UUID DEFAULT gen_random_uuid(),
    name VARCHAR(140) NOT NULL,
    description VARCHAR(255),
    url VARCHAR NOT NULL,
    trigger VARCHAR(255) NOT NULL,
    method VARCHAR(12) DEFAULT 'POST',
    is_active BOOLEAN DEFAULT TRUE,
    created TIMESTAMPTZ DEFAULT now(),
    created_by JSONB DEFAULT '{}'::JSONB,
    updated TIMESTAMPTZ DEFAULT NULL,
    updated_by JSONB DEFAULT '{}'::JSONB,
    secret JSONB
);

-- Webhook execution table
CREATE TABLE webhook_execution (
    id UUID DEFAULT gen_random_uuid(),
    trigger VARCHAR(255) NOT NULL,
    payload TEXT NOT NULL,
    payload_id VARCHAR(255) NOT NULL,
    encrypted BOOLEAN DEFAULT FALSE,
    attempts INT DEFAULT 0,
    created TIMESTAMPTZ DEFAULT now(),
    attempted TIMESTAMPTZ DEFAULT NULL,
    status VARCHAR(100) DEFAULT NULL,
    propagations JSONB
);

-- Token table
CREATE TABLE token (
    id SERIAL PRIMARY KEY,
    meta JSONB NOT NULL,
    expiration TIMESTAMPTZ NOT NULL
);

-- Message queue table
CREATE TABLE message_queue (
    id UUID DEFAULT gen_random_uuid(),
    message TEXT NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    attempted TIMESTAMPTZ DEFAULT NULL,
    status VARCHAR(100) NOT NULL,
    attempts INT DEFAULT 0,
    error TEXT DEFAULT NULL
);

-- Backs the multi-pod-safe email claim (status-filtered, created-ordered scan
-- for `FOR UPDATE SKIP LOCKED`). See migration 20260722_02.
CREATE INDEX IF NOT EXISTS idx_message_queue_claim
    ON message_queue (status, created);

-- Postgres-backed key-value artifact cache for `postgres-only` mode (no Redis).
-- Empty/unused in full and standalone modes. See migration 20260722_01.
CREATE TABLE kv_artifact (
    key        TEXT        PRIMARY KEY,
    value      TEXT        NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kv_artifact_expires_at
    ON kv_artifact (expires_at);

-- Generalized instance-wide settings store. Each row is one named
-- configuration entry; `value`'s shape is defined and validated by the
-- application layer, not by this schema. Row *presence* under a given key
-- is itself meaningful for existence-based flags -- e.g. the staff
-- bootstrap claim (see feature staff-bootstrap) stores nothing but a
-- `STAFF_BOOTSTRAP_KEY` row once claimed; its absence means still pending.
-- See migration 20260713_01.
CREATE TABLE instance_settings (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    created_by JSONB DEFAULT '{}'::JSONB,
    updated_by JSONB DEFAULT '{}'::JSONB,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);

--------------------------------------------------------------------------------
-- CONSTRAINTS
--------------------------------------------------------------------------------

-- Tenant table constraints
ALTER TABLE tenant ADD CONSTRAINT tenant_pk PRIMARY KEY (id);
ALTER TABLE tenant ADD CONSTRAINT tenant_name_unique UNIQUE (name);

-- Account table constraints
ALTER TABLE account ADD CONSTRAINT account_pk PRIMARY KEY (id);
ALTER TABLE account ADD CONSTRAINT unique_account_name UNIQUE (name, tenant_id);
ALTER TABLE account ADD CONSTRAINT unique_account_slug UNIQUE (slug, tenant_id);
ALTER TABLE account ADD CONSTRAINT fk_account_tenant FOREIGN KEY (tenant_id) REFERENCES tenant(id);

-- Account tag table constraints
ALTER TABLE account_tag ADD CONSTRAINT account_tag_pk PRIMARY KEY (id);
ALTER TABLE account_tag ADD CONSTRAINT unique_account_tag UNIQUE (value, account_id);
ALTER TABLE account_tag ADD CONSTRAINT fk_account_tag FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE;

-- Public user table constraints
ALTER TABLE public.user ADD CONSTRAINT user_pk PRIMARY KEY (id);
ALTER TABLE public.user ADD CONSTRAINT unique_email_account UNIQUE (email, account_id);
ALTER TABLE public.user ADD CONSTRAINT fk_user_account FOREIGN KEY (account_id) REFERENCES account(id);

-- Identity provider table constraints
ALTER TABLE identity_provider ADD CONSTRAINT identity_provider_pk PRIMARY KEY (user_id);
ALTER TABLE identity_provider ADD CONSTRAINT unique_user_password_hash UNIQUE (user_id, password_hash);
ALTER TABLE identity_provider ADD CONSTRAINT unique_user_name UNIQUE (user_id, name);
ALTER TABLE identity_provider ADD CONSTRAINT fk_identity_user FOREIGN KEY (user_id) REFERENCES public.user(id) ON DELETE CASCADE;

-- Owner on tenant table constraints
ALTER TABLE owner_on_tenant ADD CONSTRAINT owner_on_tenant_pk PRIMARY KEY (id);
ALTER TABLE owner_on_tenant ADD CONSTRAINT unique_tenant_owner UNIQUE (tenant_id, owner_id);
ALTER TABLE owner_on_tenant ADD CONSTRAINT fk_tenant FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE;
ALTER TABLE owner_on_tenant ADD CONSTRAINT fk_owner FOREIGN KEY (owner_id) REFERENCES public.user(id) ON DELETE CASCADE;

-- Manager account on tenant table constraints
ALTER TABLE manager_account_on_tenant ADD CONSTRAINT manager_account_on_tenant_pk PRIMARY KEY (id);
ALTER TABLE manager_account_on_tenant ADD CONSTRAINT unique_tenant UNIQUE (tenant_id);
ALTER TABLE manager_account_on_tenant ADD CONSTRAINT unique_account UNIQUE (account_id);
ALTER TABLE manager_account_on_tenant ADD CONSTRAINT unique_tenant_account UNIQUE (tenant_id, account_id);
ALTER TABLE manager_account_on_tenant ADD CONSTRAINT fk_tenant_manager FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE;
ALTER TABLE manager_account_on_tenant ADD CONSTRAINT fk_account_manager FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE;

-- Tenant tag table constraints
ALTER TABLE tenant_tag ADD CONSTRAINT tenant_tag_pk PRIMARY KEY (id);
ALTER TABLE tenant_tag ADD CONSTRAINT unique_tenant_tag UNIQUE (value, tenant_id);
ALTER TABLE tenant_tag ADD CONSTRAINT fk_tenant_tag FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE;

-- Guest role table constraints
ALTER TABLE guest_role ADD CONSTRAINT guest_role_pk PRIMARY KEY (id);
ALTER TABLE guest_role ADD CONSTRAINT unique_guest_role_name UNIQUE (name, permission);
ALTER TABLE guest_role ADD CONSTRAINT unique_guest_role_slug UNIQUE (slug, permission);

-- Guest role children table constraints
ALTER TABLE guest_role_children ADD CONSTRAINT guest_role_children_pk PRIMARY KEY (parent_id, child_role_id);
ALTER TABLE guest_role_children ADD CONSTRAINT fk_parent_role FOREIGN KEY (parent_id) REFERENCES guest_role(id);
ALTER TABLE guest_role_children ADD CONSTRAINT fk_child_role FOREIGN KEY (child_role_id) REFERENCES guest_role(id);

-- Guest user table constraints
ALTER TABLE guest_user ADD CONSTRAINT guest_user_pk PRIMARY KEY (id);
ALTER TABLE guest_user ADD CONSTRAINT unique_guest_user UNIQUE (email, guest_role_id);
ALTER TABLE guest_user ADD CONSTRAINT fk_guest_user_role FOREIGN KEY (guest_role_id) REFERENCES guest_role(id);

-- Guest user on account table constraints
ALTER TABLE guest_user_on_account ADD CONSTRAINT guest_user_on_account_pk PRIMARY KEY (guest_user_id, account_id);
ALTER TABLE guest_user_on_account ADD CONSTRAINT fk_guest_user FOREIGN KEY (guest_user_id) REFERENCES guest_user(id) ON DELETE CASCADE;
ALTER TABLE guest_user_on_account ADD CONSTRAINT fk_guest_account FOREIGN KEY (account_id) REFERENCES account(id);

-- Error code table constraints
ALTER TABLE error_code ADD CONSTRAINT error_code_pk PRIMARY KEY (prefix, code);

-- Webhook table constraints
ALTER TABLE webhook ADD CONSTRAINT webhook_pk PRIMARY KEY (id);
ALTER TABLE webhook ADD CONSTRAINT unique_webhook UNIQUE (name, url, trigger);

-- Webhook execution table constraints
ALTER TABLE webhook_execution ADD CONSTRAINT webhook_execution_pk PRIMARY KEY (id);

-- Message queue table constraints
ALTER TABLE message_queue ADD CONSTRAINT message_queue_pk PRIMARY KEY (id);

--------------------------------------------------------------------------------
-- VIEWS
--------------------------------------------------------------------------------

-- Licensed resources view
CREATE OR REPLACE VIEW licensed_resources AS
SELECT DISTINCT
	ac.id AS acc_id,
	ac.name AS acc_name,
	ac.is_default AS is_acc_std,
	gr.id AS gr_id,
	gr.slug AS gr_slug,
	gr.permission AS gr_perm,
	gu.email AS gu_email,
	gu.was_verified AS gu_verified,
	ac.tenant_id AS tenant_id,
    ga.permit_flags AS permit_flags,
    ga.deny_flags AS deny_flags
FROM
	guest_user_on_account AS ga
JOIN
	guest_user AS gu
ON
	ga.guest_user_id = gu.id
JOIN
	guest_role AS gr
ON
	gr.id = gu.guest_role_id
JOIN
	account AS ac
ON
	ac.id = ga.account_id
WHERE
	ac.is_deleted = FALSE
ORDER BY
    gu_email, gr_slug, acc_id, gr_id;

-- Public connection string info view
CREATE OR REPLACE VIEW public_connection_string_info AS
SELECT
    id,
    meta->'id' as innerId,
    meta->'accountId' as accountId,
    meta->'email' as email,
    meta->'name' as name,
    expiration,
    meta->'createdAt' as createdAt,
    meta->'scope' as scope
FROM
    token
WHERE
    meta ? 'token'
AND
    meta ? 'name'
AND
    meta ? 'id'
ORDER BY id DESC;

--------------------------------------------------------------------------------
-- TELEGRAM IDP
--------------------------------------------------------------------------------

-- GIN index on account.meta for fast JSONB reverse-lookup (used by Telegram IdP
-- and any future platform IdP that stores identity in account.meta).
--
-- Built non-CONCURRENTLY: this file only ever runs on a fresh, empty database
-- (see GUARD), so there is no concurrent write traffic to avoid locking -- and
-- CONCURRENTLY cannot run inside the transaction that makes phase B atomic.
-- The CONCURRENTLY form, plus the DROP of the pre-c79c1f5d
-- idx_account_meta_telegram_user_id_per_tenant index it replaced, stay in
-- sql/migrations/ for databases being upgraded in place.
CREATE INDEX IF NOT EXISTS idx_account_meta_gin
ON account USING GIN (meta jsonb_path_ops);

-- Unique index: one Telegram from.id globally. Telegram identity links to a
-- personal account (user/manager/staff), which has no tenant_id. A Telegram ID
-- maps to at most one personal account across all tenants.
CREATE UNIQUE INDEX IF NOT EXISTS
    idx_account_meta_telegram_user_id_global
ON account ((meta -> 'telegram_user' ->> 'id'))
WHERE meta ? 'telegram_user';

-- Audit trail for all Telegram identity lifecycle events.
CREATE TABLE IF NOT EXISTS telegram_identity_audit (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID        NOT NULL,
    account_id  UUID        NOT NULL,
    event       TEXT        NOT NULL CHECK (event IN ('linked', 'unlinked', 'login_ok', 'login_fail')),
    telegram_id BIGINT,
    ip          INET,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_telegram_audit_tenant_account
    ON telegram_identity_audit (tenant_id, account_id);

CREATE INDEX IF NOT EXISTS idx_telegram_audit_created_at
    ON telegram_identity_audit (created_at);

--------------------------------------------------------------------------------
-- RESOURCE AUDIT LOG
--
-- See migration 20260713_02.
--
--------------------------------------------------------------------------------

-- Immutable audit trail for lifecycle events (created/updated/deleted) across
-- account, tenant, user, guest_role, and webhook resources.
--
-- created_at has no DEFAULT now() -- it is always supplied by the
-- application (captured synchronously at the moment the triggering use case
-- succeeded), not derived from insert time, since the write is dispatched
-- asynchronously through a channel.
--
-- No foreign keys on resource_id/tenant_id: the row must outlive the
-- resource it describes (e.g. a deleted account keeps its audit history) --
-- same posture as telegram_identity_audit.
--
-- Immutability is enforced both by omitting Updating/Deletion ports in the
-- application layer AND by the trigger below, which closes the gap left by
-- direct DB access under the app's own role.

CREATE TABLE IF NOT EXISTS resource_audit_log (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    resource_type TEXT        NOT NULL CHECK (resource_type IN
                       ('account', 'account_meta', 'user', 'tenant', 'tenant_meta', 'guest_role', 'webhook')),
    resource_id   UUID        NOT NULL,
    tenant_id     UUID,
    event         TEXT        NOT NULL CHECK (event IN ('created', 'updated', 'deleted')),
    performed_by  JSONB       NOT NULL,
    metadata      JSONB       NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_resource_audit_log_resource ON resource_audit_log (resource_id, created_at DESC);
CREATE INDEX idx_resource_audit_log_tenant   ON resource_audit_log (tenant_id, created_at DESC)
    WHERE tenant_id IS NOT NULL;

CREATE OR REPLACE FUNCTION prevent_resource_audit_log_mutation() RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'resource_audit_log is immutable: % not allowed', TG_OP;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_resource_audit_log_immutable
BEFORE UPDATE OR DELETE ON resource_audit_log
FOR EACH ROW EXECUTE FUNCTION prevent_resource_audit_log_mutation();

--------------------------------------------------------------------------------
-- PERMISSIONS
--
-- Must stay last: `ON ALL TABLES` is evaluated at execution time, so any table
-- created after this block would silently receive no grants.
--
--------------------------------------------------------------------------------

GRANT CONNECT ON DATABASE :"db_name" TO :"db_role";

GRANT USAGE ON SCHEMA public TO :"db_role";

GRANT ALL ON ALL TABLES IN SCHEMA public TO :"db_role";

GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO :"db_role";

COMMIT;

\echo ''
\echo '>> OK: Mycelium 9.0.0 schema installed in database' :'db_name'
\echo ''
