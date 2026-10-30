# Downstream Profile Signature

The gateway forwards the caller's identity to downstream services in the
`x-mycelium-profile` header. The format of that header is public, so a
downstream service that can be reached **without** passing through the
gateway (misconfiguration, SSRF, missing network policy) would accept a forged
profile, including `isStaff: true`.

Profile signing closes that gap. The gateway signs the exact profile header
with an Ed25519 private key; downstream services that opt in verify the
signature locally with the corresponding public key and reject anything the
gateway did not produce.

> **Opt-in on both sides.** Without `[core.profileSigning]` the gateway sends
> exactly what it sent before. Without `MYC_PROFILE_PUBLIC_KEY_*` a downstream
> service decodes the profile exactly as before.

---

## What is sent

For every request that already carries `x-mycelium-profile` (security groups
`protected` and `protectedByRoles`, and body-based identity routes), the
gateway adds:

| Header | Value |
|---|---|
| `x-mycelium-profile` | Unchanged: Base64 of ZSTD-compressed JSON |
| `x-mycelium-profile-signature` | Compact JWS (`header.claims.signature`) |

JWS protected header: `{"typ": "JWT", "alg": "EdDSA", "kid": "<key version>"}`.

Claims:

```json
{
  "iat": 1800000000,
  "ph": "<base64url(sha256(x-mycelium-profile value))>",
  "aud": "<downstream service name>"
}
```

- `ph` binds the signature to the **exact** header bytes; the profile itself
  is not duplicated. Token size depends on the service name and key version;
  SDKs reject a signature header larger than 4096 UTF-8 bytes.
- `iat` is the signing time in Unix seconds. Verifiers accept
  `|now - iat| <= tolerance` (default 60 seconds, both directions).
- `aud` is the name of the service the request is routed to. Verifiers that
  configure an expected audience reject pairs signed for another service.

In the traditional bearer/connection-string flow, `public` and
`authenticated` routes receive no profile and therefore no signature. A
body-based IdP route is resolved before that group match and signs the
profile it produces, including those groups. A profile restored from cache is signed again on every request.
Client-supplied `x-mycelium-*` headers, including a forged signature, are
stripped on the way in, and `x-mycelium-*` headers echoed by a downstream are
stripped from the response.

---

## Gateway configuration

Generate a key pair (keep the private key secret, distribute the public key):

```bash
openssl genpkey -algorithm ED25519 -out profile-signing-v1.pem
openssl pkey -in profile-signing-v1.pem -pubout -out profile-signing-v1.pub.pem
```

Configure the private key through any `SecretResolver` source. The PEM must
keep its real line breaks:

```toml
[core.profileSigning]
primaryKeyVersion = 1

[[core.profileSigning.keys]]
version = 1
privateKey = { env = "MYC_PROFILE_PRIVATE_KEY_1" }
# or: privateKey = { vault = { path = "myc/core/profileSigning", key = "v1-private-pem" } }
```

```bash
export MYC_PROFILE_PRIVATE_KEY_1="$(cat profile-signing-v1.pem)"
```

At boot, after Vault initialization and before accepting traffic, the gateway
resolves, imports and test-signs **every** listed key (primary or not). It
refuses to start (`MYC00033`) when:

- the section is present but `keys` is empty, a version is `0` or repeated, or
  `primaryKeyVersion` is not listed;
- a key cannot be resolved (missing or empty env var, Vault error or timeout
  after 10 seconds per key, with a 60-second budget for the complete key set);
- a `vault` entry is used while Vault is not configured (always the case in
  standalone mode);
- a value is not an Ed25519 PKCS#8 private key.

Errors name the key version and the failing step, never the key material.
The gateway signs only with `primaryKeyVersion`. If a signature cannot be
produced for a request, the request is **not** forwarded (500, logged as
`MYC00034`); an unsigned profile is never sent while signing is enabled.

Works identically in full, postgres-only and standalone modes. It adds no
database table, migration or network call per request.

---

## Downstream configuration

Every verifier (the Python, JavaScript and Go SDKs and the Rust
`GatewayProfileData` extractor) reads the same variables:

| Variable | Meaning |
|---|---|
| `MYC_PROFILE_PUBLIC_KEY_<version>` | Gateway public key (PEM SPKI) trusted for `kid = <version>`. Any such variable enables verification |
| `MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS` | Accepted clock difference, default `60` |
| `MYC_PROFILE_SIGNATURE_AUDIENCE` | Optional. When set, `aud` must equal it (use the service `name` registered in the gateway) |

Any present but invalid value is a configuration error. JavaScript checks it
when middleware/plugin is created. Python checks it in
`configure_profile_signature()` at startup; if that call is omitted, the first
request loads and caches the error and responds 500. Go checks it in
`mycelium.ConfigureProfileSignature()` at startup; otherwise the adapters load
it once and respond 500. Rust services register
`web::Data<ProfileSignatureVerification>` built with
`ProfileSignatureVerification::from_environment()` at startup; otherwise the
extractor loads it once and responds 500 (logged as `MYC00035`). An invalid
configuration never means "verification disabled". Tolerance is a positive
integer no larger than 9007199254740991 in every verifier.

With verification enabled, every SDK entry point verifies **before** decoding
the profile and answers `403 Unable to verify Mycelium profile` on failure,
in every environment (the development leniency does not apply). Python
`get_profile_from_header_required` preserves FastAPI's 422 when the profile
header itself is absent. Configuration/internal failures return generic 500
and never execute the handler. See the
[SDK Integration Guide](./19-sdk.md) and each SDK README for code samples.

| SDK | Install | Entry points covered |
|---|---|---|
| Python `mycelium-http-tools` | `pip install 'mycelium-http-tools[fastapi,signature]'` | `get_profile_from_request`, `profile_middleware`, `get_profile_from_header`, `get_profile_from_header_required` |
| JavaScript `@lepistabioinformatics/mycelium-sdk` | `jose` is a dependency | Express `profileMiddleware`, `getProfileFromHeaderRequired`; `fastifyProfilePlugin` |
| Go `mycelium-sdk-go` | Standard library only | `adapters/nethttp`, `adapters/gin`, `adapters/echo` `Middleware` |
| Rust `mycelium-http-tools` | Part of the crate | `GatewayProfileData` extractor (actix-web) |

The Rust extractor keeps its bearer-token fallback (the profile is fetched from
the gateway) only for requests that carry neither profile header; as soon as
either header is present, the signed profile must verify.

Rejections are logged with a stable reason:

| Reason | Typical cause |
|---|---|
| `missing_signature` | Gateway not signing yet (rollout order), or header stripped by a proxy |
| `unknown_key` | Gateway signs with a version this service does not trust yet (rotation lag), or a forged `kid` |
| `invalid_signature` | Forged or corrupted token |
| `profile_hash_mismatch` | Profile altered after signing |
| `timestamp_out_of_tolerance` | Clock skew between hosts, or an old captured pair |
| `audience_mismatch` | Pair signed for another service |
| `unsupported_algorithm`, `malformed_signature`, `invalid_claims`, `duplicate_header` | Hostile or broken input |
| `invalid_profile` | Signature valid but the profile does not decode (SDK/gateway version mismatch) |

---

## Rollout and rollback

> **Order is mandatory.** A downstream that verifies behind a gateway that is
> not signing rejects **every** authenticated request.

Rollout:

1. Deploy the gateway with `[core.profileSigning]`. Downstreams keep working
   (they ignore the new header).
2. Confirm signature presence and successful verification using a controlled
   synthetic request. Log only status/reason/key version, never the header value.
3. Upgrade the downstream SDK and set `MYC_PROFILE_PUBLIC_KEY_<version>`;
   restart. Repeat per service, at its own pace.

Rollback, in reverse order: remove the public keys from the downstreams first
(this removes their protection, a conscious operator decision), then remove
`[core.profileSigning]` from the gateway.

---

## Key rotation

1. Generate key pair v2. Add `MYC_PROFILE_PUBLIC_KEY_2` to every downstream
   **while keeping v1**; restart them.
2. Add the v2 private key to `[[core.profileSigning.keys]]`, set
   `primaryKeyVersion = 2`, rolling-restart the gateway. During the restart
   old replicas still emit `kid: "1"`; downstreams accept both.
3. Once no replica uses v1, wait for in-flight requests plus the tolerance
   window, then remove the v1 private key from the gateway.
4. Remove `MYC_PROFILE_PUBLIC_KEY_1` from the downstreams when convenient.

**Compromised private key:** remove the corresponding public key from every
downstream immediately. Requests signed with it start failing, which is the
intended effect: the timestamp window does not protect against someone who
holds the private key and can sign fresh tokens.

Keys are loaded once per process; rotation always requires a restart (no hot
reload).

---

## Security limits

- **Replay inside the window.** A captured `(profile, signature)` pair is
  accepted again until `iat + tolerance`. There is no nonce or replay cache.
  Configure `MYC_PROFILE_SIGNATURE_AUDIENCE` so a pair cannot be replayed to a
  different service.
- **Only the profile is signed.** `x-mycelium-email`, `x-mycelium-role`,
  `x-mycelium-scope` and other headers are not covered; derive identity and
  authorization from the verified profile.
- **No method, path or body binding.** The signature proves who the caller
  is, not which request they sent.
- **Not encryption.** The profile remains readable by anyone who sees the
  request; use TLS between gateway and services.


## Reproducing the integration checks

From a prepared monorepo checkout, use the local SDKs and a binary built with
exactly one mode. The driver installs nothing and uses loopback-only synthetic
services. Postgres modes start isolated Compose infrastructure with pinned
Postgres 18, Redis 8 (full only), and Mailpit with verified STARTTLS.

```bash
bash scripts/validate-profile-signature.sh --stage sdk
bash scripts/validate-profile-signature.sh --stage rust --mode standalone
bash scripts/validate-profile-signature.sh --stage e2e --mode standalone
# Repeat the rust/e2e commands with postgres-only and full.
```

Set `PYTHON`, `NODE`, `CARGO_TARGET_DIR`, `PSIG_API_BIN` and
`PSIG_EVIDENCE_DIR` for non-default tool/binary paths. Run the driver from the
monorepo root, not the gateway-only repository. The e2e stage also builds the Go
and Rust consumers (`test/profile-signature-consumers/go_app` and `rust_app`),
so Go 1.23 or newer is required. The gateway runner also accepts explicit
`--api-bin`, `--python`, `--node`, `--sdk-js-dir`, `--go-bin`, `--rust-bin`,
`--mode` and `--evidence-dir` arguments. The optional WSL root Docker adapter is test-only:
`PSIG_DOCKER_WSL_ROOT=1`; ordinary Linux/CI uses the current Docker account.

The public artifacts contain individual assertions and the actual exit status.
Captured profiles, JWS values, magic links, credentials and raw service logs
remain in a private temporary directory and are deleted at cleanup. A failed
run stays failed even when earlier assertions passed. These results do not
imply that the remote GitHub workflow has run.

Use `--benchmark` only after correctness checks and release builds. It runs
A (signing/verification off), B (signing on, verification off), and C (both on)
with real small/200-license profiles, keep-alive HTTP, concurrency 1/8/32 and
three repetitions. Compile tools before collecting samples. Benchmark results
are measurements of that environment, not an SLA.
