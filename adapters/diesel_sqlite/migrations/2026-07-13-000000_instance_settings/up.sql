-- Generalized instance-wide settings store. Mirrors the Postgres
-- `instance_settings` table (Jsonb -> TEXT, per SM-R4). Each row is one
-- named configuration entry; `value`'s shape is defined and validated by
-- the application layer, not by this schema.

CREATE TABLE instance_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_by TEXT,
    updated_by TEXT,
    created TEXT NOT NULL,
    updated TEXT
);
