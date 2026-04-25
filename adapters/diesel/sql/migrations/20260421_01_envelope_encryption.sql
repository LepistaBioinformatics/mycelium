-- Envelope encryption: per-tenant data-encryption keys (DEK) wrapped by KEK.
--
-- encrypted_dek: NULL until first use; the implementation provisions it lazily
--   via get_or_provision_dek.
-- kek_version: tracks which KEK generation was used to wrap the DEK.
--   Increment after a KEK rotation and run `myc-cli rotate-kek`.

ALTER TABLE tenant
    ADD COLUMN IF NOT EXISTS encrypted_dek TEXT,
    ADD COLUMN IF NOT EXISTS kek_version   INTEGER NOT NULL DEFAULT 1;
