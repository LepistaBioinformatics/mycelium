-- Webhook execution events as super users see them: dispatching state and
-- delivery counts only. The payload, the propagation URLs and the response
-- bodies stay in `webhook_execution` (see issue #113).
CREATE VIEW webhook_execution_event AS
SELECT
    we.id,
    we.trigger,
    we.payload_id,
    COALESCE(we.status, 'unknown') AS status,
    COALESCE(we.attempts, 0) AS attempts,
    we.created,
    we.attempted,
    (
        SELECT count(*) FROM json_each(
            CASE WHEN json_valid(we.propagations)
                AND json_type(we.propagations) = 'array'
            THEN we.propagations ELSE '[]' END
        )
    ) AS propagations_total,
    (
        SELECT count(*) FROM json_each(
            CASE WHEN json_valid(we.propagations)
                AND json_type(we.propagations) = 'array'
            THEN we.propagations ELSE '[]' END
        ) AS p
        WHERE CAST(json_extract(p.value, '$.status') AS INTEGER)
            NOT BETWEEN 200 AND 299
    ) AS propagations_failed,
    (
        SELECT CAST(json_extract(p.value, '$.status') AS INTEGER)
        FROM json_each(
            CASE WHEN json_valid(we.propagations)
                AND json_type(we.propagations) = 'array'
            THEN we.propagations ELSE '[]' END
        ) AS p
        ORDER BY p.key DESC
        LIMIT 1
    ) AS last_response_status
FROM webhook_execution AS we;
