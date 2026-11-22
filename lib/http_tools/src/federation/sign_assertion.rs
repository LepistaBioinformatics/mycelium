use super::{AssertionClaims, ASSERTION_TYP};
use crate::models::VersionedEd25519Signer;

use mycelium_base::utils::errors::MappedErrors;

/// Sign an assertion as a compact EdDSA JWS with the issuer's federation key
pub fn sign_assertion(
    signer: &VersionedEd25519Signer,
    claims: &AssertionClaims,
) -> Result<String, MappedErrors> {
    signer.sign_typed_claims(Some(ASSERTION_TYP), claims)
}
