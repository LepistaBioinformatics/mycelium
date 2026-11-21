-- Trace and hourly quota of the downstream service email notifications.
--
-- One row per recipient of `POST /_adm/notifications/email`, written in the
-- same transaction that enqueues the message: `id` is also the
-- `message_queue.id` and the `X-Mycelium-Message-Id` header of that message.
-- The quota of a service is the count of its rows created during the last
-- hour, serialized per `sender_tag` by an advisory lock, so the same table is
-- both the trace and the counter.
--
-- No foreign key to `token`: deleting a connection string must not erase the
-- trace. No foreign key to `message_queue`: delivered messages are deleted.
-- Template variables are not stored, since they may hold personal data.
-- `recipient` is personal data: rows are purged after
-- `[core.serviceNotifications] retentionDays`.
--
-- Unused/empty while no service declares a `notifications` block; harmless to
-- create everywhere.
--
-- Fresh installs get it from `up.sql`. Existing databases must apply this
-- file, otherwise every notification send fails with "relation
-- notification_dispatch does not exist".
--
-- Requires -v db_role, same as 20260722_01. MUST BE APPLIED AS THE TABLE OWNER
-- (normally the superuser that ran up.sql). Every statement is idempotent.

CREATE TABLE IF NOT EXISTS notification_dispatch (
    id                   UUID         PRIMARY KEY,
    sender_tag           VARCHAR(32)  NOT NULL,
    channel              VARCHAR(16)  NOT NULL DEFAULT 'email',
    connection_string_id INT          NOT NULL,
    account_id           UUID         NOT NULL,
    tenant_id            UUID,
    recipient            VARCHAR(320) NOT NULL,
    template             VARCHAR(64)  NOT NULL,
    locale               VARCHAR(16)  NOT NULL,
    created_at           TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS notification_dispatch_tag_created_idx
    ON notification_dispatch (sender_tag, created_at);

GRANT ALL ON notification_dispatch TO :"db_role";
