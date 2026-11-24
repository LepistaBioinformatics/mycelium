-- Instance federation, phase F6b (translation).
--
-- federation_grant_translation: consumer side (B). Operator rules turning a
-- peer's grant into a local one at read time (D-4, FED-48 … FED-52). A rule
-- targets a pre-existing local account and guest role; deleting either
-- deletes the rule, so a rule never yields a grant on something gone.
--
-- resource_audit_log.resource_type gains 'federation_translation'.
--
-- Unused/empty unless the gateway is built with the federation feature;
-- harmless to create everywhere. Fresh installs get these from up.sql.
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql). Every statement is idempotent.

CREATE TABLE IF NOT EXISTS federation_grant_translation (
    id                  UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    peer_id             UUID         NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    issuer_tenant_id    UUID         NOT NULL,
    issuer_role         VARCHAR(140) NOT NULL,
    issuer_perm         INTEGER      NOT NULL,
    local_account_id    UUID         NOT NULL REFERENCES account (id) ON DELETE CASCADE,
    local_guest_role_id UUID         NOT NULL REFERENCES guest_role (id) ON DELETE CASCADE,
    local_perm          INTEGER      NOT NULL,
    enabled             BOOLEAN      NOT NULL DEFAULT true,
    created             TIMESTAMPTZ  NOT NULL DEFAULT now(),
    created_by          JSONB,
    updated             TIMESTAMPTZ,
    updated_by          JSONB
);

CREATE INDEX IF NOT EXISTS federation_grant_translation_peer_idx
    ON federation_grant_translation (peer_id) WHERE enabled;

ALTER TABLE resource_audit_log
    DROP CONSTRAINT IF EXISTS resource_audit_log_resource_type_check;

ALTER TABLE resource_audit_log
    ADD CONSTRAINT resource_audit_log_resource_type_check CHECK (resource_type IN
        ('account', 'account_meta', 'user', 'tenant', 'tenant_meta', 'guest_role', 'webhook',
         'federation_resolution', 'federation_translation'));

GRANT ALL ON federation_grant_translation TO :"db_role";
