-- Multi-pod-safe claim for the webhook dispatch queue.
--
-- The dispatcher now claims execution events the way the email queue does
-- (`FOR UPDATE SKIP LOCKED` inside a transaction that marks the batch), rather
-- than relying on the random start-up jitter that only shifted the odds of two
-- replicas picking up the same event. See issue #192.
--
-- `claimed_at` is the lease clock and is deliberately NOT `attempted`: the two
-- answer different questions. `attempted` is when the last real attempt
-- happened and drives the exponential back-off; `claimed_at` is when a pod took
-- ownership and drives crash recovery. Conflating them -- as the email queue
-- had to, having no spare column -- makes the reclaim deadline fall in the past
-- for any event whose previous attempt is already older than the window, which
-- is exactly the backlog case where two pods would race.

ALTER TABLE webhook_execution
    ADD COLUMN IF NOT EXISTS claimed_at TIMESTAMPTZ DEFAULT NULL;

-- Backs the claim's selector: status filter, attempt-tier back-off comparison
-- and the stale-claim branch, all on the same scan.
CREATE INDEX IF NOT EXISTS idx_webhook_execution_claim
    ON webhook_execution (status, attempts, attempted);
