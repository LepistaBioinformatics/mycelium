mod shared;

pub mod guest_role_deletion;
pub mod guest_role_fetching;
pub mod guest_role_registration;
pub mod guest_role_updating;

pub use guest_role_deletion::*;
pub use guest_role_fetching::*;
pub use guest_role_registration::*;
pub use guest_role_updating::*;
