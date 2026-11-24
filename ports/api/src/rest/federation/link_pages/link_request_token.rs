use myc_core::domain::dtos::federation::LinkRequestSecret;
use uuid::Uuid;

/// The `request` parameter of an initiation URL: `<request id>.<secret>`
pub(super) fn read_link_request_token(
    token: &str,
) -> Option<(Uuid, LinkRequestSecret)> {
    let (id, secret) = token.split_once('.')?;
    let id = Uuid::parse_str(id).ok()?;

    match secret.is_empty() {
        true => None,
        false => Some((id, LinkRequestSecret::from_url(secret.to_string()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_splits_into_id_and_secret() {
        let id = Uuid::new_v4();
        let (read_id, secret) =
            read_link_request_token(&format!("{id}.abc")).unwrap();

        assert_eq!(read_id, id);
        assert_eq!(secret.as_str(), "abc");
    }

    #[test]
    fn malformed_tokens_are_refused() {
        for token in ["", "abc", "not-a-uuid.abc", &format!("{}.", Uuid::nil())]
        {
            assert!(read_link_request_token(token).is_none());
        }
    }
}
