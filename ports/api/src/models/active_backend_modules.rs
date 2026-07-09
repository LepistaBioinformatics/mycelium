//! Re-exports the shaku app-modules selected by the persistence-backend
//! feature (`postgres-backend` vs `standalone`), so the ~50 handler/
//! middleware/dispatcher files that resolve `SqlAppModule`/`KVAppModule`
//! components stay backend-agnostic: they import the module type from here
//! instead of the concrete adapter crate, and never change across backends.

#[cfg(feature = "postgres-backend")]
pub use myc_diesel::repositories::SqlAppModule;

#[cfg(feature = "standalone")]
pub use myc_diesel_sqlite::repositories::SqlAppModule;

#[cfg(feature = "postgres-backend")]
pub use myc_kv::repositories::KVAppModule;

#[cfg(feature = "standalone")]
pub use myc_moka_cache::repositories::KVAppModule;
