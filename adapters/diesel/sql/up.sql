-- Example usage:
-- psql -v db_password='myc-password' -f create_tables.sql

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

\set db_name 'mycelium-v7-dev'
\set db_user 'mycelium-v7-user'
\set db_role 'service-role-mycelium-v7'

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
-- ROLES
--------------------------------------------------------------------------------

CREATE ROLE :"db_role";

CREATE USER :"db_user" WITH PASSWORD :'db_password';

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
CREATE TABLE tenant (
    id UUID DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    meta JSONB,
    status JSONB[],
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
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
    created TIMESTAMPTZ DEFAULT now()
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
	ac.tenant_id AS tenant_id
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
-- PERMISSIONS
--------------------------------------------------------------------------------

GRANT CONNECT ON DATABASE :"db_name" TO :"db_role";

GRANT USAGE ON SCHEMA public TO :"db_role";

GRANT ALL ON ALL TABLES IN SCHEMA public TO :"db_role";

GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO :"db_role";
