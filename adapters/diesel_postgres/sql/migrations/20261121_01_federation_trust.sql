-- Instance federation, phase F1 (trust between peers).
--
-- federation_peer: peers declared by the operator through myc-cli only
-- (FED-07), never by a remote call. instance_id is the identity the operator
-- expects; a handshake that sees another one marks the peer untrusted
-- instead of re-pinning it (FED-08). status is validated by the application
-- (declared / handshaking / active / degraded / unreachable / quarantined /
-- untrusted).
--
-- federation_peer_key: the public keys read from a peer's self-description,
-- one row per kid. Public material only.
--
-- federation_request_nonce: the jti of every accepted signed peer request
-- (D-9). The primary key makes a replay fail on every pod at once; rows are
-- purged after expires_at.
--
-- Unused/empty unless the gateway is built with the federation feature;
-- harmless to create everywhere. Fresh installs get these tables from up.sql.
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql). Every statement is idempotent.

CREATE TABLE IF NOT EXISTS federation_peer (
    id               UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    instance_id      UUID          NOT NULL UNIQUE,
    base_url         VARCHAR(2048) NOT NULL,
    display_name     VARCHAR(255),
    status           VARCHAR(32)   NOT NULL DEFAULT 'declared',
    consume_kinds    JSONB         NOT NULL DEFAULT '[]'::JSONB,
    serve_kinds      JSONB         NOT NULL DEFAULT '[]'::JSONB,
    negotiated_kinds JSONB         NOT NULL DEFAULT '[]'::JSONB,
    policy           JSONB         NOT NULL DEFAULT '{}'::JSONB,
    last_contact_at  TIMESTAMPTZ,
    failure_count    INT           NOT NULL DEFAULT 0,
    created          TIMESTAMPTZ   NOT NULL DEFAULT now(),
    created_by       JSONB,
    updated          TIMESTAMPTZ,
    updated_by       JSONB
);

CREATE TABLE IF NOT EXISTS federation_peer_key (
    peer_id    UUID         NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    kid        VARCHAR(16)  NOT NULL,
    alg        VARCHAR(16)  NOT NULL,
    public_key VARCHAR(64)  NOT NULL,
    fetched_at TIMESTAMPTZ  NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    PRIMARY KEY (peer_id, kid)
);

CREATE TABLE IF NOT EXISTS federation_request_nonce (
    jti        VARCHAR(128) PRIMARY KEY,
    peer_id    UUID         NOT NULL REFERENCES federation_peer (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ  NOT NULL
);

CREATE INDEX IF NOT EXISTS federation_request_nonce_expires_idx
    ON federation_request_nonce (expires_at);

GRANT ALL ON federation_peer TO :"db_role";
GRANT ALL ON federation_peer_key TO :"db_role";
GRANT ALL ON federation_request_nonce TO :"db_role";
