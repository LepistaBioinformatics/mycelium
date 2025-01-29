-- TENANT RELATED MODELS

CREATE TABLE tenant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    meta JSONB,
    status JSONB[],
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);

CREATE TABLE owner_on_tenant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    owner_id UUID NOT NULL,
    guest_by VARCHAR NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,

    CONSTRAINT unique_tenant_owner UNIQUE (tenant_id, owner_id),
    CONSTRAINT fk_tenant FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    CONSTRAINT fk_owner FOREIGN KEY (owner_id) REFERENCES user(id) ON DELETE CASCADE
);

CREATE TABLE manager_account_on_tenant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    account_id UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,

    CONSTRAINT unique_tenant UNIQUE (tenant_id),
    CONSTRAINT unique_account UNIQUE (account_id),
    CONSTRAINT unique_tenant_account UNIQUE (tenant_id, account_id),
    CONSTRAINT fk_tenant_manager FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE,
    CONSTRAINT fk_account_manager FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE
);

CREATE TABLE tenant_tag (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    value VARCHAR(64) NOT NULL,
    meta JSONB,
    tenant_id UUID NOT NULL,

    CONSTRAINT unique_tenant_tag UNIQUE (value, tenant_id, meta),
    CONSTRAINT fk_tenant_tag FOREIGN KEY (tenant_id) REFERENCES tenant(id) ON DELETE CASCADE
);

-- USER RELATED MODELS

CREATE TABLE user (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(140) NOT NULL,
    email VARCHAR(140) NOT NULL,
    first_name VARCHAR(140) NOT NULL,
    last_name VARCHAR(140) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    is_principal BOOLEAN DEFAULT FALSE,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    mfa JSONB,
    account_id UUID DEFAULT NULL,

    CONSTRAINT unique_email_account UNIQUE (email, account_id),
    CONSTRAINT fk_user_account FOREIGN KEY (account_id) REFERENCES account(id)
);

CREATE TABLE identity_provider (
    user_id UUID PRIMARY KEY,
    name VARCHAR(255) DEFAULT NULL,
    password_hash VARCHAR(255) DEFAULT NULL,

    CONSTRAINT unique_user_password_hash UNIQUE (user_id, password_hash),
    CONSTRAINT unique_user_name UNIQUE (user_id, name),
    CONSTRAINT fk_identity_user FOREIGN KEY (user_id) REFERENCES user(id) ON DELETE CASCADE
);

CREATE TABLE guest_role (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(140) NOT NULL,
    slug VARCHAR(140) NOT NULL,
    description VARCHAR(255),
    permission INT DEFAULT 0,

    CONSTRAINT unique_guest_role_name UNIQUE (name, permission),
    CONSTRAINT unique_guest_role_slug UNIQUE (slug, permission)
);

CREATE TABLE guest_role_children (
    parent_id UUID NOT NULL,
    child_role_id UUID NOT NULL,

    PRIMARY KEY (parent_id, child_role_id),
    CONSTRAINT fk_parent_role FOREIGN KEY (parent_id) REFERENCES guest_role(id),
    CONSTRAINT fk_child_role FOREIGN KEY (child_role_id) REFERENCES guest_role(id)
);

CREATE TABLE guest_user (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR NOT NULL,
    guest_role_id UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    was_verified BOOLEAN DEFAULT FALSE,

    CONSTRAINT unique_guest_user UNIQUE (email, guest_role_id),
    CONSTRAINT fk_guest_user_role FOREIGN KEY (guest_role_id) REFERENCES guest_role(id)
);

CREATE TABLE guest_user_on_account (
    guest_user_id UUID NOT NULL,
    account_id UUID NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),

    PRIMARY KEY (guest_user_id, account_id),
    CONSTRAINT fk_guest_user FOREIGN KEY (guest_user_id) REFERENCES guest_user(id) ON DELETE CASCADE,
    CONSTRAINT fk_guest_account FOREIGN KEY (account_id) REFERENCES account(id)
);

-- ACCOUNT RELATED MODELS

CREATE TABLE account (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(256) NOT NULL,
    slug VARCHAR(256) NOT NULL,
    meta JSONB,
    account_type JSONB DEFAULT '{}'::JSONB,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    is_checked BOOLEAN DEFAULT FALSE,
    is_archived BOOLEAN DEFAULT FALSE,
    is_default BOOLEAN DEFAULT FALSE,
    tenant_id UUID DEFAULT NULL,

    CONSTRAINT unique_account_name UNIQUE (name, tenant_id),
    CONSTRAINT unique_account_slug UNIQUE (slug, tenant_id),
    CONSTRAINT fk_account_tenant FOREIGN KEY (tenant_id) REFERENCES tenant(id)
);

CREATE TABLE account_tag (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    value VARCHAR(64) NOT NULL,
    meta JSONB,
    account_id UUID NOT NULL,

    CONSTRAINT unique_account_tag UNIQUE (value, account_id, meta),
    CONSTRAINT fk_account_tag FOREIGN KEY (account_id) REFERENCES account(id) ON DELETE CASCADE
);

-- ERROR CODE RELATED MODELS

CREATE TABLE error_code (
    code SERIAL NOT NULL,
    prefix VARCHAR NOT NULL,
    message VARCHAR(255) NOT NULL,
    details VARCHAR(255),
    is_internal BOOLEAN DEFAULT FALSE,
    is_native BOOLEAN DEFAULT FALSE,

    PRIMARY KEY (prefix, code)
);

-- WEBHOOK MODELS

CREATE TABLE webhook (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(140) NOT NULL,
    description VARCHAR(255),
    url VARCHAR NOT NULL,
    trigger VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL,
    secret JSONB,

    CONSTRAINT unique_webhook UNIQUE (name, url, trigger)
);

CREATE TABLE webhook_execution (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    correspondence_id UUID NOT NULL,
    trigger VARCHAR(255) NOT NULL,
    artifact TEXT NOT NULL,
    created TIMESTAMPTZ DEFAULT now(),
    execution_details JSONB
);
