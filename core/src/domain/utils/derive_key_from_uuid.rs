use ring::digest;
use uuid::Uuid;

#[tracing::instrument(name = "derive_key_from_uuid", skip_all)]
pub(crate) fn derive_key_from_uuid(uuid: &Uuid) -> [u8; 32] {
    let uuid_bytes = uuid.as_bytes();
    let digest = digest::digest(&digest::SHA256, uuid_bytes);
    let mut key = [0u8; 32];
    key.copy_from_slice(digest.as_ref());
    key
}
