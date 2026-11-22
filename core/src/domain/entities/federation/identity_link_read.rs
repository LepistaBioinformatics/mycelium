use crate::domain::dtos::federation::{IdentityLink, SubjectRef};

use async_trait::async_trait;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;
use uuid::Uuid;

/// Reads of identity links (FED-12)
#[async_trait]
pub trait IdentityLinkRead: Interface + Send + Sync {
    async fn get(
        &self,
        link_id: Uuid,
    ) -> Result<FetchResponseKind<IdentityLink, Uuid>, MappedErrors>;

    /// The pending or active link of the user with the peer, if any
    async fn find_live(
        &self,
        local_user_id: Uuid,
        peer_id: Uuid,
    ) -> Result<FetchResponseKind<IdentityLink, Uuid>, MappedErrors>;

    /// The link the peer names with `local_subject_ref`, whatever its status
    async fn find_by_local_ref(
        &self,
        peer_id: Uuid,
        local_subject_ref: SubjectRef,
    ) -> Result<FetchResponseKind<IdentityLink, SubjectRef>, MappedErrors>;

    /// Every link, or only the user's, newest first
    async fn list(
        &self,
        local_user_id: Option<Uuid>,
    ) -> Result<Vec<IdentityLink>, MappedErrors>;

    /// Active links of the subjects with these local emails
    async fn list_active_by_emails(
        &self,
        emails: &[String],
    ) -> Result<Vec<IdentityLink>, MappedErrors>;
}
