use std::fmt;

/// Why a signed peer request was refused
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerRequestRejection {
    Malformed,
    UnsupportedAlgorithm,
    WrongType,
    /// The caller does not (or no longer) publish this `kid`; the callee may
    /// re-fetch the caller's keys once before refusing
    UnknownKid(String),
    BadSignature,
    WrongAudience,
    Expired,
    IssuedInFuture,
    /// Method, path or body differ from what was signed
    RequestMismatch,
}

impl fmt::Display for PeerRequestRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("malformed peer request signature"),
            Self::UnsupportedAlgorithm => f.write_str("unsupported algorithm"),
            Self::WrongType => f.write_str("not a peer request token"),
            Self::UnknownKid(kid) => write!(f, "unknown key id {kid}"),
            Self::BadSignature => f.write_str("invalid signature"),
            Self::WrongAudience => f.write_str("wrong audience"),
            Self::Expired => f.write_str("expired"),
            Self::IssuedInFuture => f.write_str("issued in the future"),
            Self::RequestMismatch => {
                f.write_str("method, path or body differ from the signature")
            }
        }
    }
}
