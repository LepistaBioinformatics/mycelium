/// Shared use cases
///
/// This module contains use cases that are not scoped to a single role and
/// are instead called from many different call sites across the codebase
/// (e.g. audit-trail emission, shared by every write use case).
///
pub mod audit;
