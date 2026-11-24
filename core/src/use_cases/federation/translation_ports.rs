use crate::domain::entities::{
    FederationPeerRead, GrantTranslationRead, GrantTranslationWrite,
    ResourceAuditLogRegistration,
};

/// What managing grant translation rules reads and writes
#[derive(Clone, Copy)]
pub struct TranslationPorts<'a> {
    pub read: &'a dyn GrantTranslationRead,
    pub write: &'a dyn GrantTranslationWrite,
    pub peers: &'a dyn FederationPeerRead,
    pub audit: &'a dyn ResourceAuditLogRegistration,
}
