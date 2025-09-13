/// These module contains the MappedErrors base type implementation.
mod base;
pub use base::*;

/// This module contains the MappedErrors `Results` factories used to construct
/// errors. These factories are used to standardize errors codes.
mod factories;
pub use factories::*;
