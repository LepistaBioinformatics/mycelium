use super::{
    ProfileHeaderHash, ProfileSignatureAudience, UnixTimestampSeconds,
};

use serde::{Deserialize, Serialize};

/// Claims of the downstream profile signature JWS
///
/// Wire contract shared by the gateway and every SDK verifier:
///
/// ```json
/// { "iat": 1800000000, "ph": "<base64url(sha256(profile header))>",
///   "aud": "<downstream service name>" }
/// ```
///
/// The profile itself is never embedded; `ph` binds the signature to the
/// exact header value instead.
///
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileSignatureClaims {
    pub iat: UnixTimestampSeconds,
    pub ph: ProfileHeaderHash,
    pub aud: ProfileSignatureAudience,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> ProfileSignatureClaims {
        ProfileSignatureClaims {
            iat: UnixTimestampSeconds::try_from(1_800_000_000).unwrap(),
            ph: ProfileHeaderHash::from_profile_header("abc"),
            aud: ProfileSignatureAudience::try_from("svc".to_string()).unwrap(),
        }
    }

    #[test]
    fn serializes_wire_shape() {
        let json = serde_json::to_value(claims()).unwrap();

        assert_eq!(
            json,
            serde_json::json!({
                "iat": 1_800_000_000u64,
                "ph": "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0",
                "aud": "svc",
            })
        );
    }

    #[test]
    fn roundtrips_through_json() {
        let json = serde_json::to_string(&claims()).unwrap();
        let parsed: ProfileSignatureClaims =
            serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, claims());
    }

    #[test]
    fn rejects_missing_or_invalid_members() {
        let invalid = [
            r#"{"ph":"ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0","aud":"s"}"#,
            r#"{"iat":1,"aud":"s"}"#,
            r#"{"iat":1,"ph":"ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"}"#,
            r#"{"iat":-1,"ph":"ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0","aud":"s"}"#,
            r#"{"iat":1.5,"ph":"ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0","aud":"s"}"#,
            r#"{"iat":"1","ph":"ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0","aud":"s"}"#,
            r#"{"iat":1,"ph":"short","aud":"s"}"#,
            r#"{"iat":1,"ph":"ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0","aud":""}"#,
        ];

        for raw in invalid {
            assert!(
                serde_json::from_str::<ProfileSignatureClaims>(raw).is_err(),
                "{raw}"
            );
        }
    }
}
