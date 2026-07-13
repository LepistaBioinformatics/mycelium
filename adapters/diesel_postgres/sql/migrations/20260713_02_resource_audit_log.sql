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
