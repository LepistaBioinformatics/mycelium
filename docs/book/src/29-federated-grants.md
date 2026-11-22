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
For each peer the user has an active link with:

1. A peer that is quarantined, unreachable, did not negotiate `profile.v1`
   or is not allowed to assert is **skipped**.
2. A stored assertion that has not expired is used as is (`stored`); the
   peer is not called.
3. Otherwise the consumer sends a signed `POST /federation/resolve` with the
   link's opaque reference. Every peer is asked in parallel, each call is
   bounded by `peerTimeoutMs`, and the whole resolution by
   `resolveDeadlineMs`.

The answer lists one outcome per peer, and `partial` is true when some peer
could not contribute:

| Outcome | Meaning |
|---|---|
| `answered` | The peer answered, the assertion passed every check and was stored |
| `stored` | Served from the store |
| `unknown` | The peer has no active link for the reference |
| `skipped` | The peer was not asked (the reason says why) |
| `unreachable` | The call failed or missed the deadline (partial) |
| `rejected` | The assertion failed a check (partial; the reason says which) |

A partial result is not an error. An unreachable or rejected answer counts
against the peer's circuit breaker, as a failed health check does.

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
| `assertionTtlSecs` | `[core.federation]` | Lifetime of the assertions this instance issues |
| `peerTimeoutMs` | `[core.federation]` | Bound of one call to a peer |
| `resolveDeadlineMs` | `[core.federation]` | Bound of a whole resolution |
| `mayAssert` | Peer policy | Whether the peer's assertions are accepted (default: no) |
