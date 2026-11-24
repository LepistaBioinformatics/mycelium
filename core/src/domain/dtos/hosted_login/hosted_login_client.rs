use serde::{Deserialize, Serialize};

/// A front-end application allowed to start the hosted login flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostedLoginClient {
    pub client_id: String,

    /// Where the gateway may send the browser back after login. Matched
    /// exactly: no prefix, wildcard or normalization.
    pub redirect_uris: Vec<String>,
}

/// The registry of clients allowed to use the hosted login flow.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostedLoginClients(pub Vec<HostedLoginClient>);

impl HostedLoginClients {
    pub fn allows(&self, client_id: &str, redirect_uri: &str) -> bool {
        self.0.iter().any(|client| {
            client.client_id == client_id
                && client.redirect_uris.iter().any(|uri| uri == redirect_uri)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clients() -> HostedLoginClients {
        HostedLoginClients(vec![HostedLoginClient {
            client_id: "webapp".to_string(),
            redirect_uris: vec!["https://app.example.com/callback".to_string()],
        }])
    }

    #[test]
    fn allows_registered_client_and_exact_redirect_uri() {
        assert!(clients().allows("webapp", "https://app.example.com/callback"));
    }

    #[test]
    fn rejects_unknown_client_or_non_exact_redirect_uri() {
        let clients = clients();

        assert!(!clients.allows("other", "https://app.example.com/callback"));
        assert!(!clients.allows("webapp", "https://app.example.com/callback/x"));
        assert!(!clients.allows("webapp", "https://app.example.com/callback?"));
        assert!(!clients.allows("webapp", "https://app.example.com"));
        assert!(!HostedLoginClients::default()
            .allows("webapp", "https://app.example.com/callback"));
    }
}
