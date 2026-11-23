//! In-code native error codes merged with the custom codes stored in the
//! database
//!
//! The code is the source of truth for the native codes: listing and reading
//! serve them from memory and ignore native rows stored in the database, and
//! writes that would act on a native through the database are refused.

mod list_native_then_custom_error_codes;
mod match_native_error_codes;
mod refuse_native_error_code_writes;

pub use list_native_then_custom_error_codes::*;
pub use match_native_error_codes::*;
pub use refuse_native_error_code_writes::*;
