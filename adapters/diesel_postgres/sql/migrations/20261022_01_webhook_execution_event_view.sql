-- Read-only view behind the webhook execution events endpoints (issue #113).
--
-- Fresh installs get it from `up.sql`. Existing databases must apply this file,
-- otherwise `GET /_adm/system-manager/webhooks/executions` (and the matching
-- JSON-RPC methods) fail with "relation webhook_execution_event does not exist".
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql): the app role only needs SELECT.

-- Webhook execution events as super users see them: dispatching state and
-- delivery counts only. The payload, the propagation URLs and the response
-- bodies stay in `webhook_execution` (see issue #113).
CREATE OR REPLACE VIEW webhook_execution_event AS
SELECT
    we.id,
    we.trigger,
    we.payload_id,
    COALESCE(we.status, 'unknown') AS status,
    COALESCE(we.attempts, 0) AS attempts,
    we.created,
    we.attempted,
    COALESCE(deliveries.total, 0) AS propagations_total,
    COALESCE(deliveries.failed, 0) AS propagations_failed,
    deliveries.last_status AS last_response_status
FROM webhook_execution we
LEFT JOIN LATERAL (
    SELECT
        count(*) AS total,
        count(*) FILTER (
            WHERE (p.value ->> 'status')::int NOT BETWEEN 200 AND 299
        ) AS failed,
        (array_agg((p.value ->> 'status')::int ORDER BY p.ordinality DESC))[1]
            AS last_status
    FROM jsonb_array_elements(
        CASE
            WHEN jsonb_typeof(we.propagations) = 'array' THEN we.propagations
            ELSE '[]'::jsonb
        END
    ) WITH ORDINALITY AS p(value, ordinality)
) deliveries ON true;

GRANT SELECT ON webhook_execution_event TO :"db_role";
