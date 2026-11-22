use crate::domain::dtos::{
    federation::{IdentityLink, NewIdentityLink, SubjectRef},
    written_by::WrittenBy,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{CreateResponseKind, UpdatingResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

/// Changes to identity links (FED-12, FED-47)
#[async_trait]
pub trait IdentityLinkWrite: Interface + Send + Sync {
    /// Record a link with a freshly minted `local_subject_ref`. `NotCreated`
    /// when the user already has a pending or active link with the peer.
    async fn record(
        &self,
        link: NewIdentityLink,
    ) -> Result<CreateResponseKind<IdentityLink>, MappedErrors>;

    /// Turn a pending link active once the peer acknowledged it.
    /// `NotUpdated` unless the link is pending.
    async fn activate(
        &self,
        link_id: Uuid,
        peer_subject_ref: SubjectRef,
    ) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors>;

    /// `NotUpdated` when the link is already revoked
    async fn revoke(
        &self,
        link_id: Uuid,
        revoked_by: Option<WrittenBy>,
    ) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors>;
}
