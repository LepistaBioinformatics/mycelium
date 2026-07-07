// ? ----------------------------------------------------------------------------
// ? Postgres backend (full mode)
// ?
// ? The existing schema, models, migrations and repository impls are
// ? Postgres-specific (PgConnection, Jsonb/Array/Uuid/Timestamptz SQL types) and
// ? are gated behind the `postgres` feature. The `sqlite` backend adds parallel
// ? modules in later standalone-mode tasks.
// ? ----------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub mod migration;
#[cfg(feature = "postgres")]
pub mod models;
#[cfg(feature = "postgres")]
pub mod repositories;
#[cfg(feature = "postgres")]
mod schema;

// ? ----------------------------------------------------------------------------
// ? SQLite backend (standalone mode)
// ?
// ? Parallel module tree mirroring the Postgres one, with SQLite-compatible
// ? types (TEXT-backed Uuid/Json/Array/Timestamp) and embedded migrations.
// ? ----------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
pub mod sqlite;
