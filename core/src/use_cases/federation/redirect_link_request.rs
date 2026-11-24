use super::review_link_request::{active_peer_of, spent};
use crate::domain::{
    dtos::federation::{FederationPeer, LinkRequest, LinkRequestSecret},
    entities::{FederationPeerRead, LinkRequestRegistration, UserFetching},
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// What the consumer needs to sign the redirect state: the spent request,
/// its active peer and the email of the user who started the link
pub type LinkRedirect = (LinkRequest, FederationPeer, String);

/// Spend an initiation URL to send the user to the issuer (FED-42, FED-46)
///
/// One conditional update: of two concurrent submissions, one wins. The
/// email is read now, so the issuer compares against the current address.
///
#[tracing::instrument(name = "redirect_link_request", skip_all)]
pub async fn redirect_link_request(
    request_id: Uuid,
    secret: LinkRequestSecret,
    request_repo: Box<&dyn LinkRequestRegistration>,
    peer_read_repo: Box<&dyn FederationPeerRead>,
    user_fetching_repo: Box<&dyn UserFetching>,
) -> Result<LinkRedirect, MappedErrors> {
    let FetchResponseKind::Found(request) =
        request_repo.redirect(request_id, secret.hash()).await?
    else {
        return spent();
    };

    let peer = active_peer_of(&request, *peer_read_repo).await?;
    let FetchResponseKind::Found(user) = user_fetching_repo
        .get_user_by_id(request.local_user_id)
        .await?
    else {
        return use_case_err("the user of this link request is gone")
            .with_exp_true()
            .as_error();
    };

    Ok((request, peer, user.email.email()))
}
