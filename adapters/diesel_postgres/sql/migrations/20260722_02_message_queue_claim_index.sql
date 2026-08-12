-- Supports the multi-pod-safe email claim query on `message_queue`.
--
-- The dispatcher now claims pending messages with
-- `... WHERE status = 'Queued' ORDER BY created DESC ... FOR UPDATE SKIP LOCKED`
-- (see adapters/diesel_postgres/.../local_message_read.rs). This index backs
-- the status-filtered, created-ordered scan so concurrent pods claim disjoint
-- batches efficiently. Applies to full and postgres-only modes alike.

CREATE INDEX IF NOT EXISTS idx_message_queue_claim
    ON message_queue (status, created);
