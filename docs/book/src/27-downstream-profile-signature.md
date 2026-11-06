# Downstream Profile Signature

The gateway forwards the caller's identity to downstream services in the
`x-mycelium-profile` header. The format of that header is public, so a
downstream service that can be reached **without** passing through the
gateway (misconfiguration, SSRF, missing network policy) would accept a forged
profile, including `isStaff: true`.

Profile signing closes that gap. The gateway signs the exact profile header
with an Ed25519 private key and publishes the matching public keys as a JSON
Web Key Set (JWKS). Downstream services that opt in download those keys from
the gateway, verify the signature locally and reject anything the gateway did
not produce.

> **Opt-in on both sides.** Without `[core.profileSigning]` the gateway sends
> exactly what it sent before. Without `MYCELIUM_JWKS_URL` a downstream
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
- `aud` is the name of the service the request is routed to. Verifiers
  always compare it with their configured audience and reject pairs signed
  for another service.

In the traditional bearer/connection-string flow, `public` and
`authenticated` routes receive no profile and therefore no signature. A
body-based IdP route is resolved before that group match and signs the
profile it produces, including those groups. A profile restored from cache is signed again on every request.
Client-supplied `x-mycelium-*` headers, including a forged signature, are
stripped on the way in, and `x-mycelium-*` headers echoed by a downstream are
stripped from the response.

---

## Gateway configuration

Generate a private key and keep it secret. Only the private key is
configured: the gateway derives the public key and publishes it, so there is
no public key file to distribute.

```bash
openssl genpkey -algorithm ED25519 -out profile-signing-v1.pem
```

Configure the private key through any `SecretResolver` source. The PEM must
keep its real line breaks:

```toml
[core.profileSigning]
primaryKeyVersion = 1
# Optional: how long downstreams may cache the published keys
# (60 to 86400 seconds, default 43200 = 12 hours)
jwksMaxAgeSeconds = 43200

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

- the section is present but `keys` is empty, a version is `0` or repeated,
  `primaryKeyVersion` is not listed, or `jwksMaxAgeSeconds` is outside
  60..86400;
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

## Published keys (JWKS)

When `[core.profileSigning]` is configured, the gateway serves its public
keys at a public, unauthenticated route:

```
GET /.well-known/mycelium-profile-jwks.json
```

```json
{
  "keys": [
    {"kty": "OKP", "crv": "Ed25519", "x": "<32-byte public key, base64url>",
     "kid": "1", "use": "sig", "alg": "EdDSA"}
  ]
}
```

- The set contains **every** configured key version, not only the primary,
  so a downstream already trusts the next key before the gateway signs with
  it. It never contains private material (`d`) or certificates.
- The body is built once at boot. The response carries
  `Content-Type: application/jwk-set+json` and
  `Cache-Control: public, max-age=<jwksMaxAgeSeconds>`.
- Without `[core.profileSigning]` the route answers `404`.

The keys only verify, so publishing them is safe. Profile-signing keys must
**never be reused for any other purpose** (for example instance federation):
the signature uses `typ: "JWT"`, which is only unambiguous while nothing else
is signed with these keys.

---

## Downstream configuration

Every verifier (the Python, JavaScript and Go SDKs and the Rust
`GatewayProfileData` extractor) reads the same variables:

| Variable | Meaning |
|---|---|
| `MYCELIUM_JWKS_URL` | URL of the gateway JWKS, for example `https://gateway.internal/.well-known/mycelium-profile-jwks.json`. Setting it enables verification |
| `MYC_PROFILE_SIGNATURE_AUDIENCE` | **Required** when `MYCELIUM_JWKS_URL` is set. The `aud` claim must equal it (use the service `name` registered in the gateway) |
| `MYC_PROFILE_SIGNATURE_TOLERANCE_SECONDS` | Optional. Accepted clock difference, default `60` |
| `MYCELIUM_JWKS_ALLOW_INSECURE_HTTP` | Development only. `true` accepts an `http://` JWKS URL (a warning is logged); any other value than `true`/`false` is an error |

The JWKS URL must use `https://` and must not carry credentials. It comes
**only** from the service configuration: a verifier never follows a key or a
key location found in the request it is verifying (which is also why tokens
carrying `jku`, `jwk`, `x5u` or `x5c` are rejected). Static public keys in
environment variables are not supported (see
[issue #225](https://github.com/LepistaBioinformatics/mycelium/issues/225)).

How the keys are kept:

- **First use:** the set is downloaded on the first signed request, not at
  startup. Requests only reach the service through the gateway, so the gateway
  is up by then, and the service never depends on it to start (both can boot
  in any order). Until a set is downloaded, requests fail closed (`403`) and
  the download is retried at most every 5 seconds.
- **Cache:** the set is reused for the `max-age` announced by the gateway,
  bounded to 1 minute..1 day (12 hours when absent), then refreshed.
- **Unknown `kid`:** a signature naming a key version the cached set does not
  have triggers one refetch, at most once every 30 seconds, shared by
  concurrent requests. A new key is therefore picked up without restarting
  the service.
- **Gateway unreachable:** a failed refresh keeps the last valid set and is
  logged.
- **Strict set:** a set with private material, another key type or curve, a
  key that is not 32 bytes, or a non-canonical or repeated `kid` is refused
  as a whole.
- **Bounded fetch:** 5-second timeout, 64 KiB body, redirects never followed.

If a signed profile arrives while `MYCELIUM_JWKS_URL` is unset, the verifier
logs one warning per process: the profile is still decoded as before, but
nothing is verified.

Any present but invalid value, including a missing audience, is a
configuration error. Call the startup function of your SDK so a bad setting
stops the service before it accepts traffic. It only validates the settings
and never needs the gateway. Downloading the keys at startup is opt-in: it
makes an unreachable gateway stop the service, so use it only when the
service does not need to start before the gateway:

| SDK | Startup call (validates the settings) | Opt-in download at startup |
|---|---|---|
| Python | `configure_profile_signature()` | `configure_profile_signature(prefetch=True)` (raises `ProfileJwksFetchError`) |
| JavaScript | Fastify: registering `fastifyProfilePlugin`. Express: the settings are read when the middleware is created | Pass `signatureVerifier: await prepareProfileSignatureVerifier()` to the Express middleware or the Fastify plugin |
| Go | `mycelium.ConfigureProfileSignature(ctx)` returns an error to abort on | `verifier.Prefetch(ctx)` on the returned verifier |
| Rust | Register `web::Data<ProfileSignatureVerification>` built with `ProfileSignatureVerification::from_environment()` | `verification.prefetch().await` |

When the settings are invalid and no startup call was made, requests fail with
500 (Rust logs `MYC00035`). An invalid configuration never means
"verification disabled". Tolerance is a positive integer no larger than
9007199254740991 in every verifier. Warnings go to the standard logging of each
platform (`logging` in Python, `console.warn` in JavaScript, `log/slog` in Go,
`tracing` in Rust).

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
| `unknown_key` | Version not published by the gateway even after a refetch (the refetch cooldown was active, or the gateway is unreachable), or a forged `kid` |
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
3. Upgrade the downstream SDK, set `MYCELIUM_JWKS_URL` and
   `MYC_PROFILE_SIGNATURE_AUDIENCE`, and restart. Repeat per service, at its
   own pace.

Rollback, in reverse order: unset `MYCELIUM_JWKS_URL` in the downstreams first
(this removes their protection, a conscious operator decision), then remove
`[core.profileSigning]` from the gateway.

---

## Key rotation

No downstream service changes at any step:

1. Generate private key v2 and add it to `[[core.profileSigning.keys]]`
   **keeping `primaryKeyVersion = 1`**; rolling-restart the gateway. The JWKS
   now publishes v1 and v2.
2. Set `primaryKeyVersion = 2` and rolling-restart the gateway. During the
   restart old replicas still emit `kid: "1"` and new ones `kid: "2"`;
   downstreams accept both (the first `kid: "2"` they see triggers a refetch
   if their cached set predates step 1).
3. Once no replica uses v1, wait for in-flight requests plus the tolerance
   window, then remove the v1 private key from the gateway and restart. The
   JWKS stops publishing v1; downstreams drop it at their next refresh.

**Compromised private key:** sign with a new key immediately (add it as the
primary in one step) and remove the compromised key from the gateway.
Downstreams pick up the new `kid` within the 30-second refetch cooldown.
Until their cached set expires (`jwksMaxAgeSeconds`) they may still trust the
removed key; lower `jwksMaxAgeSeconds` beforehand, or restart the downstreams,
when that window matters. The timestamp window does not protect against
someone who holds the private key and can sign fresh tokens.

The gateway loads its keys once per process; changing them requires a
gateway restart (no hot reload).

---

## Security limits

- **Replay inside the window.** A captured `(profile, signature)` pair is
  accepted again until `iat + tolerance`. There is no nonce or replay cache.
  The mandatory `MYC_PROFILE_SIGNATURE_AUDIENCE` prevents replaying a pair to
  a different service.
- **The JWKS is the trust anchor.** Serve it over TLS and point
  `MYCELIUM_JWKS_URL` at the gateway itself, never at a location an
  untrusted party controls.
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
