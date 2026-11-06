use myc_http_tools::models::{ProfileSigningState, PublishedProfileKeys};

/// What boot preparation of `[core.profileSigning]` produces
///
/// The primary signer (used on every forwarded request) and the JWKS that
/// publishes the public half of every configured key. Both are registered as
/// `web::Data`; both are disabled when the section is absent.
///
#[derive(Clone, Debug)]
pub(crate) struct PreparedProfileSigning {
    pub(crate) state: ProfileSigningState,
    pub(crate) published_keys: PublishedProfileKeys,
}

impl PreparedProfileSigning {
    pub(crate) fn disabled() -> Self {
        Self {
            state: ProfileSigningState::Disabled,
            published_keys: PublishedProfileKeys::Disabled,
        }
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }
}
