-- Instance federation, phase F3 (resolution).
--
-- federation_assertion_cache: consumer side (B). The authoritative store of
-- verified assertions received from peers (D-6, FED-15), one per link and
-- kind. A row past exp is never served and is purged by the dispatcher.
--
-- federation_namespace_claim: consumer side (B). Which peer first asserted a
-- foreign tenant or account id (I-1, FED-54). The unique foreign_id makes the
-- first claim win.
--
-- resource_audit_log.resource_type gains 'federation_resolution' (FED-16).
--
-- Unused/empty unless the gateway is built with the federation feature;
-- harmless to create everywhere. Fresh installs get these from up.sql.
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql). Every statement is idempotent.

CREATE TABLE IF NOT EXISTS federation_assertion_cache (
    link_id     UUID        NOT NULL REFERENCES federation_identity_link (id) ON DELETE CASCADE,
    peer_id     UUID        NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    kind        VARCHAR(64) NOT NULL,
    jws         TEXT        NOT NULL,
    payload     JSONB       NOT NULL,
    iat         TIMESTAMPTZ NOT NULL,
    exp         TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (link_id, kind)
);

CREATE INDEX IF NOT EXISTS federation_assertion_cache_exp_idx
    ON federation_assertion_cache (exp);

CREATE TABLE IF NOT EXISTS federation_namespace_claim (
    foreign_id    UUID        PRIMARY KEY,
    peer_id       UUID        NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    kind          VARCHAR(16) NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE resource_audit_log
    DROP CONSTRAINT IF EXISTS resource_audit_log_resource_type_check;

ALTER TABLE resource_audit_log
    ADD CONSTRAINT resource_audit_log_resource_type_check CHECK (resource_type IN
        ('account', 'account_meta', 'user', 'tenant', 'tenant_meta', 'guest_role', 'webhook',
         'federation_resolution'));

GRANT ALL ON federation_assertion_cache TO :"db_role";
GRANT ALL ON federation_namespace_claim TO :"db_role";
