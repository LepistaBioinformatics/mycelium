//! JSON-RPC method dispatchers by scope: beginners, managers.

pub(crate) mod beginners;
pub(crate) mod managers;

pub(crate) use beginners::dispatch_beginners;
pub(crate) use managers::dispatch_managers;
