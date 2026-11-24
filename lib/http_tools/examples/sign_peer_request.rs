//! Sign one federation peer request, for end-to-end tests
//!
//! Derives the federation key of an HMAC secret exactly as the gateway does
//! and prints the compact JWS for the `x-mycelium-peer-signature` header.
//! The secret is read from the environment variable named by `--secret-env`,
//! never from the command line.
//!
//! ```bash
//! cargo run -p mycelium-http-tools --example sign_peer_request -- \
//!     --secret-env MYC_HMAC_SECRET_A --version 1 \
//!     --iss <caller instance id> --aud <callee instance id> \
//!     --method POST --path /federation/handshake [--body '{}'] [--jti <id>]
//! ```

use myc_core::domain::dtos::profile_signature::SigningKeyVersion;
use myc_http_tools::{
    federation::{sign_peer_request, PeerRequest},
    models::{KeyDerivationPurpose, VersionedEd25519Signer},
};
use std::{collections::HashMap, process::exit, time::SystemTime};
use uuid::Uuid;

fn arguments() -> HashMap<String, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    args.chunks(2)
        .filter_map(|pair| match pair {
            [flag, value] => {
                Some((flag.trim_start_matches("--").to_string(), value.clone()))
            }
            _ => None,
        })
        .collect()
}

fn required<'a>(args: &'a HashMap<String, String>, name: &str) -> &'a str {
    args.get(name).map(String::as_str).unwrap_or_else(|| {
        eprintln!("missing --{name}");
        exit(2)
    })
}

fn signer(args: &HashMap<String, String>) -> VersionedEd25519Signer {
    let secret =
        std::env::var(required(args, "secret-env")).unwrap_or_else(|_| {
            eprintln!("the --secret-env variable is not set");
            exit(2)
        });
    let version: u32 = required(args, "version").parse().unwrap_or(0);
    let version = SigningKeyVersion::try_from(version).unwrap_or_else(|_| {
        eprintln!("--version must be a non-zero key version");
        exit(2)
    });

    VersionedEd25519Signer::derive_from_secret(
        version,
        secret.as_bytes(),
        KeyDerivationPurpose::Federation,
    )
    .unwrap_or_else(|_| exit(1))
}

fn uuid(args: &HashMap<String, String>, name: &str) -> Uuid {
    Uuid::parse_str(required(args, name)).unwrap_or_else(|_| {
        eprintln!("--{name} must be a UUID");
        exit(2)
    })
}

fn main() {
    let args = arguments();
    let jti = args
        .get("jti")
        .cloned()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let body = args.get("body").cloned().unwrap_or_default();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();

    let jws = sign_peer_request(
        &signer(&args),
        &PeerRequest {
            iss: uuid(&args, "iss"),
            aud: uuid(&args, "aud"),
            method: required(&args, "method"),
            path: required(&args, "path"),
            body: body.as_bytes(),
            jti: &jti,
            issued_at: now,
            lifetime_secs: 60,
        },
    )
    .unwrap_or_else(|_| exit(1));

    println!("{jws}");
}
