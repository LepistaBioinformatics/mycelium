use super::HostedLoginSession;

/// Result of checking the code typed on the hosted login page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedLoginCodeCheck {
    /// The code was right. The session now holds an authorization code.
    Verified(HostedLoginSession),

    /// The code was wrong and attempts remain.
    Rejected(HostedLoginSession),

    /// No usable session: missing, expired, no code sent, or out of
    /// attempts.
    NotFound,
}

/// What a session does with a typed code, decided before persisting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedLoginCodeOutcome {
    Verified,
    Rejected,
    Exhausted,
    NoCodeSent,
}
