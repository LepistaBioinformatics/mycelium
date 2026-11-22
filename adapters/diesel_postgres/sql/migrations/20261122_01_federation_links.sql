-- Instance federation, phase F2 (identity links).
--
-- federation_identity_link: the link between a local user and the same
-- person's account on a peer (D-2), stored on both sides. local_subject_ref
-- is the opaque reference the peer uses to ask about this user;
-- peer_subject_ref is the one this side uses to ask the peer (null while
-- pending). Neither is an email or a raw user id. At most one live (pending
-- or active) link per user and peer. status is validated by the application
-- (pending / active / revoked).
--
-- federation_link_request: consumer side (B). A single-use, short-lived link
-- initiation (FED-42). Only the hash of the URL secret is stored. The id is
-- the jti of the signed redirect state; consumed when the issuer confirms.
--
-- federation_link_consent: issuer side (A). One per accepted redirect state,
-- holding what the confirmation page shows until the user confirms. The
-- unique (peer_id, peer_request_id) makes a state usable once.
--
-- Unused/empty unless the gateway is built with the federation feature;
-- harmless to create everywhere. Fresh installs get these tables from up.sql.
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql). Every statement is idempotent.

CREATE TABLE IF NOT EXISTS federation_identity_link (
    id                UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    local_user_id     UUID         NOT NULL REFERENCES public.user (id) ON DELETE CASCADE,
    local_email       VARCHAR(140) NOT NULL,
    peer_id           UUID         NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    peer_subject_ref  VARCHAR(64),
    local_subject_ref VARCHAR(64)  NOT NULL UNIQUE,
    status            VARCHAR(16)  NOT NULL DEFAULT 'pending',
    created           TIMESTAMPTZ  NOT NULL DEFAULT now(),
    established_at    TIMESTAMPTZ,
    revoked_at        TIMESTAMPTZ,
    revoked_by        JSONB
);

CREATE UNIQUE INDEX IF NOT EXISTS federation_identity_link_live_idx
    ON federation_identity_link (local_user_id, peer_id)
    WHERE status <> 'revoked';

CREATE TABLE IF NOT EXISTS federation_link_request (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    local_user_id UUID        NOT NULL REFERENCES public.user (id) ON DELETE CASCADE,
    peer_id       UUID        NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    secret_hash   VARCHAR(64) NOT NULL,
    created       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL,
    redirected_at TIMESTAMPTZ,
    consumed_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS federation_link_request_expires_idx
    ON federation_link_request (expires_at);

CREATE TABLE IF NOT EXISTS federation_link_consent (
    id              UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    peer_id         UUID          NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    peer_request_id UUID          NOT NULL,
    peer_email      VARCHAR(140)  NOT NULL,
    return_url      VARCHAR(2048) NOT NULL,
    local_user_id   UUID          REFERENCES public.user (id) ON DELETE CASCADE,
    created         TIMESTAMPTZ   NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ   NOT NULL,
    consumed_at     TIMESTAMPTZ,
    UNIQUE (peer_id, peer_request_id)
);

CREATE INDEX IF NOT EXISTS federation_link_consent_expires_idx
    ON federation_link_consent (expires_at);

GRANT ALL ON federation_identity_link TO :"db_role";
GRANT ALL ON federation_link_request TO :"db_role";
GRANT ALL ON federation_link_consent TO :"db_role";
