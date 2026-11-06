# Profile signature contract vectors

`vectors.json` is the cross-language contract for `x-mycelium-profile-signature`. It is produced
by the gateway itself — the real `compress_and_encode_profile_to_base64` and
`sign_profile_payload` — in `lib/http_tools/tests/profile_signature_contract.rs`, with a fixed
clock and the RFC 8032 §7.1 test keys 1 and 2 (**TEST ONLY**, public material).

Each vector carries the exact profile header, the signature header, the key versions the
consumer trusts, the verifier clock, the tolerance, an optional expected audience and the expected
outcome (`valid` or a rejection reason). The file also lists the reason precedence every verifier
must follow.

Ed25519 signatures are deterministic, so regenerating produces identical bytes. The Rust test
fails if the committed file drifts from what the code produces.

## Regenerate and distribute

From `modules/mycelium-api-gateway`:

```bash
MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES=1 \
    cargo test -p mycelium-http-tools --test profile_signature_contract --locked
```

Copy the file byte for byte to the SDKs (their test suites compare the SHA-256 with this file
when the monorepo layout is available):

```bash
cp lib/http_tools/test/mock/profile-signature/vectors.json \
    ../mycelium-sdk-py/src/tests/mock/profile-signature/vectors.json
cp lib/http_tools/test/mock/profile-signature/vectors.json \
    ../mycelium-sdk-js/src/testdata/profile-signature/vectors.json
cp lib/http_tools/test/mock/profile-signature/vectors.json \
    ../mycelium-sdk-go/testdata/profile-signature/vectors.json
```

The Rust downstream verifier (`GatewayProfileData`) runs the same corpus in
`lib/http_tools/tests/profile_signature_verifier_contract.rs`.

Never hand-edit the file, and never generate signatures with Python/JS code and present them as
gateway output.

The current corpus has 46 cases: the original 45 plus
`valid_rust_permissions_and_null_status`, covering real Rust `read`/`write`
permission strings, a nil tenant and `verboseStatus: null`.

## JWKS vectors

`jwks-vectors.json` is the cross-language contract for the profile JWKS that the gateway publishes
at `/.well-known/mycelium-profile-jwks.json` and that verifiers read from `MYCELIUM_JWKS_URL`. It
is produced by `lib/http_tools/tests/profile_jwks_contract.rs` from the real publisher
(`PublishedProfileKeys` built from `VersionedEd25519Signer`), with the same TEST ONLY keys as
`vectors.json`.

It carries:

- `published`: the exact body the gateway serves for keys 1 and 2;
- `vectors`: documents a verifier must accept (`valid`, with the trusted `expectedKids`) or refuse
  as a whole (`invalid_jwks`): private member `d`, other `kty`/`crv`, `x` not 32 bytes or not
  Base64URL, non-canonical or duplicated `kid`, `use` other than `sig`, `alg` other than `EdDSA`;
- `cacheControl`: the cache lifetime derived from a `Cache-Control` value (`max-age` clamped to
  60..86400 seconds, 43200 when absent);
- `contract`: path, variables, fetch timeout, body limit and refetch cooldown.

Regenerate with the same variable, then copy it next to `vectors.json` in each SDK:

```bash
MYC_REGENERATE_PROFILE_SIGNATURE_FIXTURES=1 \
    cargo test -p mycelium-http-tools --test profile_jwks_contract --locked
```
