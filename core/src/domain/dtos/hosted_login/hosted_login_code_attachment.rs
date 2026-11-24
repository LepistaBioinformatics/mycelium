/// Result of attaching an emailed code to a hosted login session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostedLoginCodeAttachment {
    Attached,

    /// The session already received a code. Sending another one means
    /// starting a new session, which counts against the per-email cap.
    AlreadySent,

    /// Too many sessions hold an unused code for this email.
    EmailCapReached,

    /// The session does not exist or has expired.
    NotFound,
}
