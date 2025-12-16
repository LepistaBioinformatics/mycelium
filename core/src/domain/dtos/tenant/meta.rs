use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, ToSchema, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TenantMetaKey {
    /// Federal Revenue Register
    ///
    /// The Federal Revenue Register is the register of the federal revenue
    /// of the tenant.
    FederalRevenueRegister,

    /// The type for the Federal Revenue Register
    ///
    /// In Brazil, the FRR is CNPJ. In the US, the FRR is EIN.
    FederalRevenueRegisterType,

    /// The Country
    ///
    /// The country of the tenant.
    Country,

    /// The State
    ///
    /// The state of the tenant.
    State,

    /// The Province
    ///
    /// The province of the tenant.
    Province,

    /// The City
    ///
    /// The city of the tenant.
    City,

    /// The Address 1
    ///
    /// The first address of the tenant.
    Address1,

    /// The Address 2
    ///
    /// The second address of the tenant.
    Address2,

    /// The Zip Code
    ///
    /// The zip code of the tenant.
    ZipCode,

    /// The Tenant URL
    ///
    /// The URL of the tenant.
    DomainUrl,

    /// The Tenant Support Email
    ///
    /// The support email of the tenant.
    SupportEmail,

    /// The tenant preferred locale
    ///
    /// The preferred locale of the tenant. This value should be used during
    /// communication with the tenant users.
    Locale,

    /// To specify any other meta key
    ///
    /// Specify any other meta key that is not listed here.
    #[serde(untagged)]
    Custom(String),
}

impl Display for TenantMetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantMetaKey::FederalRevenueRegister => {
                write!(f, "federal_revenue_register")
            }
            TenantMetaKey::FederalRevenueRegisterType => {
                write!(f, "federal_revenue_register_type")
            }
            TenantMetaKey::Country => write!(f, "country"),
            TenantMetaKey::State => write!(f, "state"),
            TenantMetaKey::Province => write!(f, "province"),
            TenantMetaKey::City => write!(f, "city"),
            TenantMetaKey::Address1 => write!(f, "address1"),
            TenantMetaKey::Address2 => write!(f, "address2"),
            TenantMetaKey::ZipCode => write!(f, "zip_code"),
            TenantMetaKey::DomainUrl => write!(f, "domain_url"),
            TenantMetaKey::SupportEmail => write!(f, "support_email"),
            TenantMetaKey::Locale => write!(f, "locale"),
            TenantMetaKey::Custom(key) => write!(f, "{key}"),
        }
    }
}

impl FromStr for TenantMetaKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "federal_revenue_register" => {
                Ok(TenantMetaKey::FederalRevenueRegister)
            }
            "federal_revenue_register_type" => {
                Ok(TenantMetaKey::FederalRevenueRegisterType)
            }
            "country" => Ok(TenantMetaKey::Country),
            "state" => Ok(TenantMetaKey::State),
            "province" => Ok(TenantMetaKey::Province),
            "city" => Ok(TenantMetaKey::City),
            "address1" => Ok(TenantMetaKey::Address1),
            "address2" => Ok(TenantMetaKey::Address2),
            "zip_code" => Ok(TenantMetaKey::ZipCode),
            "domain_url" => Ok(TenantMetaKey::DomainUrl),
            "support_email" => Ok(TenantMetaKey::SupportEmail),
            "locale" => Ok(TenantMetaKey::Locale),
            _ => Ok(TenantMetaKey::Custom(s.to_owned())),
        }
    }
}

impl Serialize for TenantMetaKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            TenantMetaKey::Custom(key) => {
                serializer.serialize_str(key.as_str())
            }
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TenantMetaKey;
    use serde_json;
    use std::str::FromStr;

    // ? -----------------------------------------------------------------------
    // ? Test parsing of default keys
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_parse_federal_revenue_register() {
        let key = TenantMetaKey::from_str("federal_revenue_register").unwrap();
        assert!(matches!(key, TenantMetaKey::FederalRevenueRegister));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_federal_revenue_register_type() {
        let key =
            TenantMetaKey::from_str("federal_revenue_register_type").unwrap();
        assert!(matches!(key, TenantMetaKey::FederalRevenueRegisterType));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_country() {
        let key = TenantMetaKey::from_str("country").unwrap();
        assert!(matches!(key, TenantMetaKey::Country));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_state() {
        let key = TenantMetaKey::from_str("state").unwrap();
        assert!(matches!(key, TenantMetaKey::State));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_province() {
        let key = TenantMetaKey::from_str("province").unwrap();
        assert!(matches!(key, TenantMetaKey::Province));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_city() {
        let key = TenantMetaKey::from_str("city").unwrap();
        assert!(matches!(key, TenantMetaKey::City));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_address1() {
        let key = TenantMetaKey::from_str("address1").unwrap();
        assert!(matches!(key, TenantMetaKey::Address1));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_address2() {
        let key = TenantMetaKey::from_str("address2").unwrap();
        assert!(matches!(key, TenantMetaKey::Address2));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_zip_code() {
        let key = TenantMetaKey::from_str("zip_code").unwrap();
        assert!(matches!(key, TenantMetaKey::ZipCode));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_domain_url() {
        let key = TenantMetaKey::from_str("domain_url").unwrap();
        assert!(matches!(key, TenantMetaKey::DomainUrl));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_support_email() {
        let key = TenantMetaKey::from_str("support_email").unwrap();
        assert!(matches!(key, TenantMetaKey::SupportEmail));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    #[test]
    fn test_parse_locale() {
        let key = TenantMetaKey::from_str("locale").unwrap();
        assert!(matches!(key, TenantMetaKey::Locale));
        assert!(!matches!(key, TenantMetaKey::Custom(_)));
    }

    // ? -----------------------------------------------------------------------
    // ? Test that default keys are NOT parsed as Custom
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_default_keys_never_parse_as_custom() {
        let default_keys = vec![
            "federal_revenue_register",
            "federal_revenue_register_type",
            "country",
            "state",
            "province",
            "city",
            "address1",
            "address2",
            "zip_code",
            "domain_url",
            "support_email",
            "locale",
        ];

        for key_str in default_keys {
            let key = TenantMetaKey::from_str(key_str).unwrap();
            assert!(
                !matches!(key, TenantMetaKey::Custom(_)),
                "Default key '{}' should not be parsed as Custom",
                key_str
            );
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test parsing of custom keys
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_parse_custom_key() {
        let key = TenantMetaKey::from_str("custom_key").unwrap();
        assert!(matches!(key, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = key {
            assert_eq!(value, "custom_key");
        }
    }

    #[test]
    fn test_parse_custom_key_with_underscores() {
        let key = TenantMetaKey::from_str("my_custom_key_123").unwrap();
        assert!(matches!(key, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = key {
            assert_eq!(value, "my_custom_key_123");
        }
    }

    #[test]
    fn test_parse_custom_key_similar_to_default() {
        // Test keys that are similar but not identical to default keys
        let similar_keys = vec![
            "federal_revenue_register_extra",
            "country_code",
            "state_province",
            "address",
            "zip",
            "domain",
            "support",
            "locale_code",
        ];

        for key_str in similar_keys {
            let key = TenantMetaKey::from_str(key_str).unwrap();
            assert!(
                matches!(key, TenantMetaKey::Custom(_)),
                "Similar key '{}' should be parsed as Custom, not as default",
                key_str
            );
            if let TenantMetaKey::Custom(value) = key {
                assert_eq!(value, key_str);
            }
        }
    }

    #[test]
    fn test_parse_custom_key_case_sensitive() {
        // Default keys are lowercase, so uppercase should be custom
        let key = TenantMetaKey::from_str("COUNTRY").unwrap();
        assert!(matches!(key, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = key {
            assert_eq!(value, "COUNTRY");
        }
    }

    #[test]
    fn test_parse_custom_key_with_prefix() {
        let key =
            TenantMetaKey::from_str("custom_federal_revenue_register").unwrap();
        assert!(matches!(key, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = key {
            assert_eq!(value, "custom_federal_revenue_register");
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test Display implementation
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_display_default_keys() {
        let test_cases = vec![
            (
                TenantMetaKey::FederalRevenueRegister,
                "federal_revenue_register",
            ),
            (
                TenantMetaKey::FederalRevenueRegisterType,
                "federal_revenue_register_type",
            ),
            (TenantMetaKey::Country, "country"),
            (TenantMetaKey::State, "state"),
            (TenantMetaKey::Province, "province"),
            (TenantMetaKey::City, "city"),
            (TenantMetaKey::Address1, "address1"),
            (TenantMetaKey::Address2, "address2"),
            (TenantMetaKey::ZipCode, "zip_code"),
            (TenantMetaKey::DomainUrl, "domain_url"),
            (TenantMetaKey::SupportEmail, "support_email"),
            (TenantMetaKey::Locale, "locale"),
        ];

        for (key, expected) in test_cases {
            assert_eq!(key.to_string(), expected);
        }
    }

    #[test]
    fn test_display_custom_keys() {
        let key = TenantMetaKey::Custom("my_custom_key".to_string());
        assert_eq!(key.to_string(), "my_custom_key");
    }

    // ? -----------------------------------------------------------------------
    // ? Test serialization and deserialization
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_serialize_default_keys() {
        let key = TenantMetaKey::FederalRevenueRegister;
        let serialized = serde_json::to_string(&key).unwrap();
        assert_eq!(serialized, "\"federal_revenue_register\"");

        let key = TenantMetaKey::Locale;
        let serialized = serde_json::to_string(&key).unwrap();
        assert_eq!(serialized, "\"locale\"");
    }

    #[test]
    fn test_serialize_custom_keys() {
        let key = TenantMetaKey::Custom("my_custom_key".to_string());
        let serialized = serde_json::to_string(&key).unwrap();
        assert_eq!(serialized, "\"my_custom_key\"");
    }

    #[test]
    fn test_deserialize_default_keys() {
        let json = "\"federal_revenue_register\"";
        let key: TenantMetaKey = serde_json::from_str(json).unwrap();
        assert!(matches!(key, TenantMetaKey::FederalRevenueRegister));

        let json = "\"locale\"";
        let key: TenantMetaKey = serde_json::from_str(json).unwrap();
        assert!(matches!(key, TenantMetaKey::Locale));
    }

    #[test]
    fn test_deserialize_custom_keys() {
        let json = "\"my_custom_key\"";
        let key: TenantMetaKey = serde_json::from_str(json).unwrap();
        assert!(matches!(key, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = key {
            assert_eq!(value, "my_custom_key");
        }
    }

    #[test]
    fn test_deserialize_all_default_keys_via_json() {
        // Test that all default keys deserialize correctly from JSON
        // and are NOT interpreted as Custom
        let default_keys_json = vec![
            (
                "\"federal_revenue_register\"",
                TenantMetaKey::FederalRevenueRegister,
            ),
            (
                "\"federal_revenue_register_type\"",
                TenantMetaKey::FederalRevenueRegisterType,
            ),
            ("\"country\"", TenantMetaKey::Country),
            ("\"state\"", TenantMetaKey::State),
            ("\"province\"", TenantMetaKey::Province),
            ("\"city\"", TenantMetaKey::City),
            ("\"address1\"", TenantMetaKey::Address1),
            ("\"address2\"", TenantMetaKey::Address2),
            ("\"zip_code\"", TenantMetaKey::ZipCode),
            ("\"domain_url\"", TenantMetaKey::DomainUrl),
            ("\"support_email\"", TenantMetaKey::SupportEmail),
            ("\"locale\"", TenantMetaKey::Locale),
        ];

        for (json_str, expected_variant) in default_keys_json {
            let key: TenantMetaKey = serde_json::from_str(json_str).unwrap();
            assert_eq!(key, expected_variant);
            assert!(
                !matches!(key, TenantMetaKey::Custom(_)),
                "Default key from JSON '{}' should not be deserialized as Custom",
                json_str
            );
        }
    }

    #[test]
    fn test_round_trip_serialization_default_keys() {
        let default_keys = vec![
            TenantMetaKey::FederalRevenueRegister,
            TenantMetaKey::FederalRevenueRegisterType,
            TenantMetaKey::Country,
            TenantMetaKey::State,
            TenantMetaKey::Province,
            TenantMetaKey::City,
            TenantMetaKey::Address1,
            TenantMetaKey::Address2,
            TenantMetaKey::ZipCode,
            TenantMetaKey::DomainUrl,
            TenantMetaKey::SupportEmail,
            TenantMetaKey::Locale,
        ];

        for original_key in default_keys {
            let serialized = serde_json::to_string(&original_key).unwrap();
            let deserialized: TenantMetaKey =
                serde_json::from_str(&serialized).unwrap();

            assert_eq!(original_key, deserialized);
            assert!(
                !matches!(deserialized, TenantMetaKey::Custom(_)),
                "Default key should not deserialize as Custom"
            );
        }
    }

    #[test]
    fn test_round_trip_serialization_custom_keys() {
        let custom_keys = vec![
            "my_custom_key",
            "another_custom_key",
            "custom_federal_revenue_register",
            "COUNTRY",
        ];

        for key_str in custom_keys {
            let original_key = TenantMetaKey::Custom(key_str.to_string());
            let serialized = serde_json::to_string(&original_key).unwrap();
            let deserialized: TenantMetaKey =
                serde_json::from_str(&serialized).unwrap();

            assert_eq!(original_key, deserialized);
            assert!(
                matches!(deserialized, TenantMetaKey::Custom(_)),
                "Custom key '{}' should deserialize as Custom",
                key_str
            );
            if let TenantMetaKey::Custom(value) = deserialized {
                assert_eq!(value, key_str);
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test that default keys don't mix with custom keys
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_default_keys_remain_default_after_round_trip() {
        // Parse default key as string
        let key_str = "federal_revenue_register";
        let parsed = TenantMetaKey::from_str(key_str).unwrap();

        // Serialize and deserialize
        let serialized = serde_json::to_string(&parsed).unwrap();
        let deserialized: TenantMetaKey =
            serde_json::from_str(&serialized).unwrap();

        // Should still be default, not custom
        assert!(matches!(
            deserialized,
            TenantMetaKey::FederalRevenueRegister
        ));
        assert!(!matches!(deserialized, TenantMetaKey::Custom(_)));

        // Display should match original
        assert_eq!(deserialized.to_string(), key_str);
    }

    #[test]
    fn test_custom_keys_remain_custom_after_round_trip() {
        // Parse custom key as string
        let key_str = "my_custom_key";
        let parsed = TenantMetaKey::from_str(key_str).unwrap();

        // Serialize and deserialize
        let serialized = serde_json::to_string(&parsed).unwrap();
        let deserialized: TenantMetaKey =
            serde_json::from_str(&serialized).unwrap();

        // Should still be custom, not default
        assert!(matches!(deserialized, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = deserialized {
            assert_eq!(value, key_str);
        }
    }

    #[test]
    fn test_custom_key_similar_to_default_stays_custom() {
        // A custom key that contains a default key as substring
        let key_str = "federal_revenue_register_custom";
        let parsed = TenantMetaKey::from_str(key_str).unwrap();

        assert!(matches!(parsed, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(ref value) = parsed {
            assert_eq!(value, key_str);
        }

        // After round trip, should still be custom
        let serialized = serde_json::to_string(&parsed).unwrap();
        let deserialized: TenantMetaKey =
            serde_json::from_str(&serialized).unwrap();

        assert!(matches!(deserialized, TenantMetaKey::Custom(_)));
        if let TenantMetaKey::Custom(value) = deserialized {
            assert_eq!(value, key_str);
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test equality and hashing
    // ? -----------------------------------------------------------------------

    #[test]
    fn test_equality_default_keys() {
        let key1 = TenantMetaKey::from_str("country").unwrap();
        let key2 = TenantMetaKey::from_str("country").unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_equality_custom_keys() {
        let key1 = TenantMetaKey::from_str("custom_key").unwrap();
        let key2 = TenantMetaKey::from_str("custom_key").unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_inequality_default_vs_custom() {
        let default_key = TenantMetaKey::from_str("country").unwrap();
        // Test with a custom key that's different from default
        let custom_key = TenantMetaKey::from_str("custom_country").unwrap();
        assert_ne!(default_key, custom_key);
    }

    #[test]
    fn test_inequality_different_default_keys() {
        let key1 = TenantMetaKey::from_str("country").unwrap();
        let key2 = TenantMetaKey::from_str("state").unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_inequality_different_custom_keys() {
        let key1 = TenantMetaKey::from_str("custom_key_1").unwrap();
        let key2 = TenantMetaKey::from_str("custom_key_2").unwrap();
        assert_ne!(key1, key2);
    }
}
