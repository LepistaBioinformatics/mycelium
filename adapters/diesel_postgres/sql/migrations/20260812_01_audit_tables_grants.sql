-- Grants the app role access to the two tables that never got any.
--
-- `up.sql`'s blanket `GRANT ALL ON ALL TABLES IN SCHEMA public` is evaluated at
-- execution time and sits at the end of that file, so it only ever covered
-- tables that existed when it ran. `instance_settings` (20260713_01) and
-- `resource_audit_log` (20260713_02) were created afterwards by their own
-- migrations, and neither carried a GRANT -- unlike `kv_artifact`
-- (20260722_01), which does, and is therefore unaffected.
--
-- Result: on any database built from `up.sql` + migrations, the app role can
-- read every table except those two. Deployments where the app connects as a
-- superuser never noticed. Deployments following the README (app connects as
-- `db_user`, a member of `db_role`) fail on the staff-bootstrap claim and on
-- every audit-log write.
--
-- Not needed for fresh 9.0.0 installs: `up.sql` now creates both tables before
-- the grants block. This file exists so databases already running 9.0.0-rc.x
-- converge to the same state.
--
-- Requires -v db_role, same as 20260722_01. GRANT is idempotent.
--
-- MUST BE APPLIED AS THE TABLE OWNER (normally the superuser that ran up.sql).
-- GRANT requires ownership or WITH GRANT OPTION, so running this as the app's
-- own `db_user` fails with "permission denied for table instance_settings" --
-- `db_user` does not own tables the superuser created. This is the same
-- requirement 20260722_01 already has (verified: it fails identically with
-- "must be owner of table kv_artifact"), and the same one up.sql has.

GRANT ALL ON instance_settings   TO :"db_role";
GRANT ALL ON resource_audit_log  TO :"db_role";
