DROP TRIGGER IF EXISTS trg_resource_audit_log_no_delete;
DROP TRIGGER IF EXISTS trg_resource_audit_log_no_update;
DROP INDEX IF EXISTS idx_resource_audit_log_tenant;
DROP INDEX IF EXISTS idx_resource_audit_log_resource;
DROP TABLE IF EXISTS resource_audit_log;
