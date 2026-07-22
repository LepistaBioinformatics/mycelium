//! Re-exports the shaku app-modules selected by the persistence-backend
//! feature (`full` / `postgres-only` / `standalone`), so the ~50 handler/
//! middleware/dispatcher files that resolve `SqlAppModule`/`KVAppModule`
//! components stay backend-agnostic: they import the module type from here
//! instead of the concrete adapter crate, and never change across backends.

#[cfg(any(feature = "full", feature = "postgres-only"))]
pub use myc_diesel::repositories::SqlAppModule;

#[cfg(feature = "standalone")]
pub use myc_diesel_sqlite::repositories::SqlAppModule;

#[cfg(feature = "full")]
pub use myc_kv::repositories::KVAppModule;

#[cfg(feature = "postgres-only")]
pub use myc_postgres_kv::repositories::KVAppModule;

#[cfg(feature = "standalone")]
pub use myc_moka_cache::repositories::KVAppModule;
