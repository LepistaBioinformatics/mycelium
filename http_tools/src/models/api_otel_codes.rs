use serde::Serialize;
use std::fmt::Display;

/// This enum contains the telemetry codes for the API crate telemetry
///
/// See below the list of telemetry prefixes:
///
/// - END: Endpoint
/// - MID: Middleware
/// - ROUTE: Route
/// - HC: Health Check
///
#[derive(Clone, Debug, Serialize)]
#[serde(untagged, rename_all = "UPPERCASE")]
pub enum APIOtelCodes {
    /// Health Check
    ///
    /// Used on start the health check dispatcher.
    ///
    HC00001 = 1,

    /// Health Check dispatcher cicle start
    ///
    /// Dispached when a new health check dispatcher cicle is started.
    ///
    HC00002,

    /// Health Check dispatcher cicle end
    ///
    /// Dispached when a health check dispatcher cicle is finished.
    ///
    HC00003,

    /// Single service health check start
    ///
    /// Dispached when a single service health check is started.
    ///
    HC00004,

    /// Single service health check end
    ///
    /// Dispached when a single service health check is finished.
    ///
    HC00005,

    /// Single host health check start
    ///
    /// Dispached when a single host health check is started.
    ///
    HC00006,

    /// Single host health check end
    ///
    /// Dispached when a single host health check is finished.
    ///
    HC00007,
}

impl Display for APIOtelCodes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
