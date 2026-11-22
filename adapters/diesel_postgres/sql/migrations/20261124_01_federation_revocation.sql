-- Instance federation, phase F5 (revocation).
--
-- federation_tombstone: consumer side (B). The latest revocation each issuer
-- pushed for a link (FED-23). An assertion not issued after it is never
-- stored, so a refresh in flight cannot bring revoked grants back.
--
-- federation_outbox: issuer side (A). Revocations to push, one row per peer
-- and subject, retried with backoff until delivered (FED-24). Claimed with
-- FOR UPDATE SKIP LOCKED; a delivered row is deleted.
--
-- Unused/empty unless the gateway is built with the federation feature;
-- harmless to create everywhere. Fresh installs get these from up.sql.
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql). Every statement is idempotent.

CREATE TABLE IF NOT EXISTS federation_tombstone (
    link_id     UUID        PRIMARY KEY REFERENCES federation_identity_link (id) ON DELETE CASCADE,
    peer_id     UUID        NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    reason      VARCHAR(32) NOT NULL,
    issued_at   TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS federation_outbox (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    peer_id         UUID        NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    subject_ref     VARCHAR(64) NOT NULL,
    reason          VARCHAR(32) NOT NULL,
    issued_at       TIMESTAMPTZ NOT NULL,
    attempts        INTEGER     NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status          VARCHAR(16) NOT NULL DEFAULT 'pending',
    last_error      TEXT
);

CREATE INDEX IF NOT EXISTS federation_outbox_due_idx
    ON federation_outbox (status, next_attempt_at);

GRANT ALL ON federation_tombstone TO :"db_role";
GRANT ALL ON federation_outbox TO :"db_role";
