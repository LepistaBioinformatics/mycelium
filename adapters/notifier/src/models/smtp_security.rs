use serde::Deserialize;

/// Transport security for the SMTP connection.
///
/// Selects which `lettre` builder is used: implicit TLS (SMTPS, the classic
/// port 465) versus STARTTLS (the modern submission standard, port 587 --
/// RFC 6409/8314). See issue #178: providers such as Azure Communication
/// Services and Office 365 only accept STARTTLS on 587.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SmtpSecurity {
    /// TLS negotiated up front on connect (`SmtpTransport::relay`, port 465).
    Implicit,

    /// Plaintext connect, upgraded via the `STARTTLS` command
    /// (`SmtpTransport::starttls_relay`, port 587).
    StartTls,
}

impl SmtpSecurity {
    /// Auto-select the security mode when `[smtp] security` is omitted: 587 is
    /// the STARTTLS submission port; every other port (notably 465) keeps
    /// implicit TLS, preserving the pre-#178 behavior for existing deployments.
    pub(super) fn from_port(port: u16) -> Self {
        match port {
            587 => Self::StartTls,
            _ => Self::Implicit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_port_selects_starttls_for_587() {
        assert_eq!(SmtpSecurity::from_port(587), SmtpSecurity::StartTls);
    }

    #[test]
    fn from_port_defaults_to_implicit_for_465_and_others() {
        assert_eq!(SmtpSecurity::from_port(465), SmtpSecurity::Implicit);
        assert_eq!(SmtpSecurity::from_port(25), SmtpSecurity::Implicit);
        assert_eq!(SmtpSecurity::from_port(2525), SmtpSecurity::Implicit);
    }

    #[test]
    fn deserializes_lowercase_variants() {
        let implicit: SmtpSecurity =
            serde_json::from_str("\"implicit\"").unwrap();
        let starttls: SmtpSecurity =
            serde_json::from_str("\"starttls\"").unwrap();

        assert_eq!(implicit, SmtpSecurity::Implicit);
        assert_eq!(starttls, SmtpSecurity::StartTls);
    }
}
