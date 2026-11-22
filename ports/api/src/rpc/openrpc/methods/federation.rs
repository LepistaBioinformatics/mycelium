use super::super::schema;
use crate::rpc::{method_names, params};

fn method(
    name: &str,
    summary: &str,
    description: &str,
    params_schema: Option<serde_json::Value>,
) -> serde_json::Value {
    let params = match params_schema {
        Some(schema) => {
            serde_json::json!([{ "name": "params", "required": true, "schema": schema }])
        }
        None => serde_json::json!([]),
    };

    serde_json::json!({
        "name": name,
        "summary": summary,
        "description": description,
        "tags": [{ "name": "federation" }],
        "params": params,
        "result": { "name": "result", "schema": { "type": "object" } },
        "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
    })
}

pub fn methods() -> Vec<serde_json::Value> {
    let peer =
        || Some(schema::param_schema_value::<params::FederationPeerParams>());
    let link = || Some(schema::param_schema_value::<params::LinkParams>());

    vec![
        method(
            method_names::FEDERATION_PEERS_LIST,
            "List federation peers",
            "Every declared peer with its status, negotiated assertion kinds, stored key ids, last contact and failure count. Requires Staff privileges.",
            None,
        ),
        method(
            method_names::FEDERATION_PEERS_QUARANTINE,
            "Quarantine a peer",
            "Stops serving the peer from its next request. Requires Staff privileges.",
            peer(),
        ),
        method(
            method_names::FEDERATION_PEERS_RELEASE,
            "Release a peer",
            "Lifts a quarantine; the peer is declared again and re-handshaked on the next health round. Requires Staff privileges.",
            peer(),
        ),
        method(
            method_names::FEDERATION_SET_ENABLED,
            "Switch federation on or off",
            "The runtime switch. Off is the kill-switch: the self-description answers 404 and every peer request is refused. On is refused while [core.federation] is absent. Requires Staff privileges.",
            Some(schema::param_schema_value::<params::SetFederationEnabledParams>()),
        ),
        method(
            method_names::FEDERATION_LINKS_LIST,
            "List identity links",
            "Every identity link of every user, newest first, with its status and peer. Requires Staff privileges.",
            None,
        ),
        method(
            method_names::FEDERATION_LINKS_REVOKE,
            "Revoke an identity link",
            "Revokes the link here and queues a tombstone the peer receives within seconds, retried until delivered; the local revocation stands whatever the peer answers. Requires Staff privileges.",
            link(),
        ),
        method(
            method_names::FEDERATION_ASSERTIONS_LIST,
            "List stored assertions",
            "Every unexpired assertion received from a peer, with its link, peer, grants and signed JWS. Requires Staff privileges.",
            None,
        ),
        method(
            method_names::BEGINNERS_FEDERATION_LIST_PEERS,
            "List peers to link with",
            "The active peers the caller may link an account with, by display name only.",
            None,
        ),
        method(
            method_names::BEGINNERS_FEDERATION_START_LINK,
            "Start an identity link",
            "Returns a single-use, short-lived initiation URL (linkRequestTtlSecs). The browser opens it to sign in on the peer and confirm the link there. Refused when the caller is already linked to that peer.",
            Some(schema::param_schema_value::<params::StartLinkParams>()),
        ),
        method(
            method_names::BEGINNERS_FEDERATION_LIST_LINKS,
            "List my identity links",
            "The caller's identity links, newest first.",
            None,
        ),
        method(
            method_names::BEGINNERS_FEDERATION_REVOKE_LINK,
            "Revoke one of my identity links",
            "Revokes one of the caller's links here and queues a tombstone the peer receives within seconds, retried until delivered.",
            link(),
        ),
        method(
            method_names::BEGINNERS_FEDERATION_RESOLVE_GRANTS,
            "Resolve my grants on linked peers",
            "Asks the peers the caller has an active link with for the caller's grants there, verifies the signed answers and rejects any that name an id this instance or another peer owns. A stored assertion younger than assertionRefreshSecs is used without a call; an older one is refreshed, and served with stale=true when the peer does not answer before resolveDeadlineMs; an expired one contributes nothing. partial is true when some peer contributed nothing because of a failure. denied names the local reason (inactive owner, inactive, archived or deleted account) that leaves the caller without any federated grant; no peer is asked then. Each peer actually asked is recorded in the audit log.",
            None,
        ),
    ]
}
