use super::{LinkStateClaims, LINK_STATE_TYP};
use crate::models::VersionedEd25519Signer;

use mycelium_base::utils::errors::MappedErrors;

/// Sign a link redirect state as a compact EdDSA JWS with the consumer's
/// federation key
pub fn sign_link_state(
    signer: &VersionedEd25519Signer,
    claims: &LinkStateClaims,
) -> Result<String, MappedErrors> {
    signer.sign_typed_claims(Some(LINK_STATE_TYP), claims)
}
