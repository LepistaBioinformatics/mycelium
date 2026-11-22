# Federated Grants

Once an account on this instance (the *consumer*) is linked to the same
person's account on a peer (the *issuer*), the consumer can ask the issuer
which roles that person holds there. The issuer answers with a signed
**assertion**, and the consumer verifies, checks and stores it.

> Requires everything [identity linking](./28-identity-linking.md) requires,
> plus `profile.v1` negotiated with the issuer (both sides advertise it in
> their self-description from this release) and `mayAssert` set in the
> issuer's peer policy on the consumer (`myc-cli federation peer add
> --may-assert`). A peer asserts nothing until the operator allows it.

The grants are not yet part of the `x-mycelium-profile` header that
downstream services receive: that change breaks the SDK contract and ships
with the SDKs. Until then, resolution runs on demand.

---

## Asking for grants

The linked user calls `beginners.federation.resolveGrants` on the consumer.
If the user's owner is inactive, or the account is inactive, archived or
deleted on the consumer, the answer is `denied` with the reason, and no peer
is asked: federation adds grants, it never overrides a local denial. Like
local authorization, it applies once the cached profile expires.
Otherwise, for each peer the user has an active link with:

1. A peer that is quarantined or untrusted, did not negotiate `profile.v1`
   or is not allowed to assert is **skipped**.
2. A stored assertion younger than `assertionRefreshSecs` is used as is
   (`stored`); the peer is not called.
3. Otherwise the consumer sends a signed `POST /federation/resolve` with the
   link's opaque reference. Every peer is asked in parallel, each call is
   bounded by `peerTimeoutMs`, and the whole resolution by
   `resolveDeadlineMs`.
4. A peer whose circuit is open is not called: its stored assertion, if
   still unexpired, is served as `stale`.

The answer lists one outcome per peer:

| Outcome | Meaning |
|---|---|
| `answered` | The peer answered, the assertion passed every check and was stored |
| `stored` | Served from the store, younger than `assertionRefreshSecs` |
| `stale` | The peer could not refresh its assertion; the stored grants, still unexpired, are served |
| `unknown` | The peer has no active link for the reference; any stored assertion is dropped |
| `skipped` | The peer was not asked (the reason says why) |
| `unreachable` | The call failed or missed the deadline, and nothing unexpired was stored |
| `rejected` | The assertion failed a check (the reason says which); any stored assertion is dropped |

`stale` is true when some grants come from an assertion its peer could not
refresh. `partial` is true when some peer contributed nothing because of a
failure (`unreachable`, `rejected`). Neither is an error. An unreachable,
stale or rejected answer counts against the peer's circuit breaker, as a
failed health check does.

## Degradation

A silent peer does not take its users' access down at once, and does not
keep it alive for long either:

- **Within the refresh age** the stored grants are served without a call.
- **Past the refresh age, before expiry**, the peer is asked; if it does not
  answer, the stored grants are served as `stale`. While the peer is up, a
  role it changes reaches the consumer within `assertionRefreshSecs`.
- **Past expiry** a silent peer contributes nothing. Absence is never read as
  "last known good".

## What the issuer answers

The issuer answers "unknown" unless the reference names an **active link
with the caller**, the caller is an active peer and `profile.v1` was
negotiated. The answer reveals nothing else.

Otherwise it signs an assertion (`typ: mycelium-assertion+jwt`, EdDSA, its
federation key) bound to the caller (`aud`) and the reference (`sub`), valid
for `assertionTtlSecs`. It carries the user's grants on the issuer, with the
issuer's own tenant and account ids, and the list of those ids. It never
carries instance-wide flags (staff, manager, account status) nor grants on
system accounts: a peer cannot make anyone staff or manager elsewhere.

## What the consumer checks

- **Signature, audience, subject and lifetime.** An unknown key id makes the
  consumer fetch the issuer's keys once before refusing.
- **Lifetime.** An assertion is kept no longer than the consumer's own
  `assertionTtlSecs`, whatever the issuer signed: the short lifetime is what
  bounds a role removed on the issuer until revocations are pushed.
- **ID collisions.** Every tenant and account id in the assertion must not
  exist on the consumer and must not have been claimed by another peer. The
  first peer to assert an id owns it. A collision rejects the whole
  assertion: a peer cannot borrow one of this instance's tenants, or another
  peer's.

Each call to a peer is recorded in the resource audit log
(`federationResolution`), with its outcome and reason; reads from the store
are not.

## Inspecting

| Method | Who |
|---|---|
| `beginners.federation.resolveGrants` | The linked user, own grants only |
| `federation.assertions.list` | Staff: every stored, unexpired assertion |

Expired assertions are purged by the federation dispatcher.

## Settings

| Setting | Where | Meaning |
|---|---|---|
| `assertionTtlSecs` | `[core.federation]` | Lifetime of the assertions this instance issues, and the longest it keeps one it receives |
| `assertionRefreshSecs` | `[core.federation]` | Age after which a stored assertion is refreshed (below `assertionTtlSecs`) |
| `peerTimeoutMs` | `[core.federation]` | Bound of one call to a peer |
| `resolveDeadlineMs` | `[core.federation]` | Bound of a whole resolution |
| `mayAssert` | Peer policy | Whether the peer's assertions are accepted (default: no) |
