-- Postgres-backed key-value artifact cache for `postgres-only` mode (no Redis).
--
-- Mirrors the Redis SETEX cache used in full mode: `set_encoded_artifact`
-- upserts (key, value, expires_at = now() + ttl); `get_encoded_artifact` reads
-- only rows where `expires_at > now()` (lazy expiry). A periodic in-process
-- sweeper deletes rows where `expires_at <= now()` to reclaim space -- lazy
-- expiry on read remains the correctness source of truth.
--
-- Unused/empty in full and standalone modes (they use Redis / moka); harmless
-- to create everywhere so the migration is mode-agnostic.

CREATE TABLE IF NOT EXISTS kv_artifact (
    key        TEXT        PRIMARY KEY,
    value      TEXT        NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kv_artifact_expires_at
    ON kv_artifact (expires_at);

GRANT ALL ON kv_artifact TO :"db_role";
