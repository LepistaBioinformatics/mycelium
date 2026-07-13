-- Generalized instance-wide settings store. Each row is one named
-- configuration entry; `value`'s shape is defined and validated by the
-- application layer, not by this schema. Row *presence* under a given key
-- is itself meaningful for existence-based flags -- e.g. the staff
-- bootstrap claim (see feature staff-bootstrap) stores nothing but a
-- `STAFF_BOOTSTRAP_KEY` row once claimed; its absence means still pending.

CREATE TABLE instance_settings (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    created_by JSONB DEFAULT '{}'::JSONB,
    updated_by JSONB DEFAULT '{}'::JSONB,
    created TIMESTAMPTZ DEFAULT now(),
    updated TIMESTAMPTZ DEFAULT NULL
);
