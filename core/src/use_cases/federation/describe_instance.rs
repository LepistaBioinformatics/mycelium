use crate::{
    domain::dtos::federation::{FederationPublicJwk, InstanceSelfDescription},
    models::FederationConfig,
};

use uuid::Uuid;

/// Build the self-description this instance publishes (FED-03)
///
/// `keys` are the public halves of every valid federation key (FED-04).
/// No assertion kind is issued yet. `role_catalog_url` is set while the
/// role catalog is published (D-10).
///
pub fn describe_instance(
    instance_id: Uuid,
    config: &FederationConfig,
    version: &str,
    keys: Vec<FederationPublicJwk>,
    role_catalog_url: Option<String>,
) -> InstanceSelfDescription {
    InstanceSelfDescription {
        instance_id,
        display_name: config.display_name.clone(),
        base_url: config.base_url.clone(),
        version: version.to_string(),
        assertion_kinds: vec![],
        keys,
        role_catalog_url,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(display_name: Option<&str>) -> FederationConfig {
        serde_json::from_value(serde_json::json!({
            "displayName": display_name,
            "baseUrl": "https://myc-01.example.com",
        }))
        .unwrap()
    }

    fn key(kid: &str) -> FederationPublicJwk {
        FederationPublicJwk {
            kty: "OKP".to_string(),
            crv: "Ed25519".to_string(),
            x: "11qYAYKxCrfVS_7TyWQHOg7hcvPapiMlrwIaaPcHURo".to_string(),
            kid: kid.to_string(),
            key_use: "sig".to_string(),
            alg: "EdDSA".to_string(),
        }
    }

    #[test]
    fn serializes_the_instance_in_camel_case() {
        let instance_id = Uuid::new_v4();
        let document = serde_json::to_value(describe_instance(
            instance_id,
            &config(Some("Instance 01")),
            "9.0.1",
            vec![key("1"), key("2")],
            None,
        ))
        .unwrap();

        assert_eq!(document["instanceId"], instance_id.to_string());
        assert_eq!(document["displayName"], "Instance 01");
        assert_eq!(document["baseUrl"], "https://myc-01.example.com");
        assert_eq!(document["version"], "9.0.1");
        assert_eq!(document["assertionKinds"], serde_json::json!([]));
        assert_eq!(document["keys"][1]["kid"], "2");
        assert_eq!(document["keys"][0]["use"], "sig");
        assert!(document.get("roleCatalogUrl").is_none());
    }

    #[test]
    fn omits_an_absent_display_name() {
        let document = serde_json::to_value(describe_instance(
            Uuid::new_v4(),
            &config(None),
            "9.0.1",
            vec![key("1")],
            None,
        ))
        .unwrap();

        assert!(document.get("displayName").is_none());
    }

    #[test]
    fn carries_no_private_material() {
        let document = serde_json::to_value(describe_instance(
            Uuid::new_v4(),
            &config(Some("Instance 01")),
            "9.0.1",
            vec![key("1")],
            None,
        ))
        .unwrap();

        for key in document["keys"].as_array().unwrap() {
            assert!(key.get("d").is_none(), "{key}");
        }
        assert!(!document.to_string().contains("\"d\""));
    }

    #[test]
    fn advertises_the_role_catalog_while_it_is_published() {
        let document = serde_json::to_value(describe_instance(
            Uuid::new_v4(),
            &config(None),
            "9.0.1",
            vec![],
            Some("https://myc-01.example.com/catalog".to_string()),
        ))
        .unwrap();

        assert_eq!(
            document["roleCatalogUrl"],
            "https://myc-01.example.com/catalog"
        );
    }
}
