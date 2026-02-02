//! JSON-RPC param DTOs with JSON Schema support for OpenRPC discovery.
//! Grouped by scope: beginners, managers.

pub(crate) mod beginners;
pub(crate) mod managers;

pub(crate) use beginners::{
    CreateDefaultAccountParams, DeleteMyAccountParams, UpdateOwnAccountNameParams,
};
pub(crate) use managers::{
    CreateSystemAccountParams, CreateTenantParams, DeleteTenantParams,
    ExcludeTenantOwnerParams, IncludeTenantOwnerParams, ListTenantParams,
};
