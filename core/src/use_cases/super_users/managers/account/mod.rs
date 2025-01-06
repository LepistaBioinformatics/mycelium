/// Create a system account
///
/// The system account creation is restricted to managers only. To perform
/// updates or deletion of system accounts, use subscription accounts role
/// endpoints. Managers can freely use such functionalities.
///
mod create_system_account;

pub use create_system_account::*;
