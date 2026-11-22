use super::ensure_federation_operator;
use crate::domain::{
    dtos::{
        federation::{PeerVocabularyAnswer, GUEST_ROLE_VOCABULARY_KIND},
        profile::Profile,
    },
    entities::{FederationPeerRead, PeerVocabularyReader},
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// A peer's guest-role vocabulary, for staff (FED-40)
///
/// Asked only of a peer that negotiated `guest-role-vocabulary.v1`; any
/// other is `Unknown` without a call. Nothing is stored.
///
#[tracing::instrument(name = "read_peer_vocabulary", skip_all)]
pub async fn read_peer_vocabulary(
    profile: &Profile,
    instance_id: Uuid,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    reader: Box<&dyn PeerVocabularyReader>,
) -> Result<PeerVocabularyAnswer, MappedErrors> {
    ensure_federation_operator(profile)?;

    let FetchResponseKind::Found(peer) =
        peer_read_repo.get_by_instance_id(instance_id).await?
    else {
        return use_case_err(format!("no peer with instance id {instance_id}"))
            .with_exp_true()
            .as_error();
    };

    if !peer
        .negotiated_kinds
        .iter()
        .any(|kind| kind == GUEST_ROLE_VOCABULARY_KIND)
    {
        return Ok(PeerVocabularyAnswer::Unknown);
    }

    reader.read(&peer).await
}
