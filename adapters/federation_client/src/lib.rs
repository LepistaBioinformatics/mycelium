//! Outbound HTTP calls from this instance to its federation peers
//!
//! Implements the `PeerClient` port of `myc-core` with `reqwest`. Every call
//! has explicit connect and total timeouts (C-9): a slow peer must never hold
//! a gateway worker.

mod http_peer_client;
mod read_peer_response;

pub use http_peer_client::*;
