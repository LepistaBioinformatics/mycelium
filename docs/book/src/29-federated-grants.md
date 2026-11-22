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

Stored grants reach downstream services in the `x-mycelium-profile` header,
under `federatedResources`, apart from the local `licensedResources` (see
[In the profile](#in-the-profile)).

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

## In the profile

On every request, after the profile cache, the gateway adds the grants
stored for the subject under `federatedResources`. Each entry is a licensed
resource with one more member, `issuer`: the instance id of the peer that
asserted it.

```json
{
  "licensedResources": { "records": [ { "tenantId": "…", "role": "reader" } ] },
  "federatedResources": [
    { "issuer": "6f9c…", "tenantId": "…", "accId": "…", "role": "reader", "perm": "read" }
  ]
}
```

- **Local by default.** Every SDK filter works on `licensedResources` only.
  `.with_federated()` adds every federated grant to the local ones;
  `.from_issuer(instance_id)` keeps one issuer's grants only (for chains
  without a tenant filter). Both must open the chain: called after another
  filter they leave nothing, rather than bring back grants that filter never
  saw. An SDK that predates this field ignores it, and still verifies the
  signature, which covers the exact header.
- **No confusion with local grants.** Once merged, a federated grant is a
  plain grant to the filters. That is safe because its tenant and account
  ids can never be this instance's own, nor another peer's: the consumer
  rejects any assertion naming one (see "ID collisions" below).
- **Never cached.** The federated part is read from the assertion store on
  every request, so a tombstone, a quarantine or an expiry acts at once.
- **No peer call on the request.** A subject without links costs one read.
  When an assertion is missing or due, a request routed to a downstream
  service starts a background refresh, at most once per subject per
  `assertionRefreshSecs`; the request itself serves what is stored. The
  gateway's own API never starts one (`resolveGrants` refreshes on
  demand).
- **Local denial wins.** A subject whose owner is inactive, or whose account
  is inactive, archived or deleted, gets no `federatedResources`.

## Translation

Raw federated grants carry the issuer's tenant and account ids, which mean
nothing to an application that only knows this instance. A **translation
rule** turns them into ordinary local grants: *"role `editor` on tenant
`T_A` at A becomes guest role `reviewer` on local account `Z`"*.

- **Local grants.** A translated grant is added to `licensedResources`, so
  the default filter chain sees it and an unchanged downstream works.
- **Read time only.** Rules are applied on every request to the grants
  stored for the subject, after the profile cache. Nothing is written to the
  guest tables: a rule enabled or disabled applies on the next request, and
  a tombstone removes the translated grant with the raw one.
- **Bounded.**
  - A rule targets a guest role and a tenant account that already exist on
    this instance, and never a system account. Deleting either deletes the
    rule.
  - System grants of the issuer are never translated.
  - The issuer's permission only restricts: a rule matches grants with at
    least its issuer permission, and the local permission is the lower of
    the rule's and the grant's. A read on A never becomes a write here.
- **Per peer.** A rule applies only to grants of the peer it names.

| Method | Who |
|---|---|
| `federation.translations.list` | Staff; optionally one peer (`peerInstanceId`) |
| `federation.translations.create` | Staff: `peerInstanceId`, `issuerTenantId`, `issuerRole`, `issuerMinPerm`, `localAccountId`, `localGuestRoleId`, `localPerm` |
| `federation.translations.setEnabled` / `federation.translations.delete` | Staff |

Every change is recorded in the resource audit log
(`federationTranslation`).

## Revocation

When the issuer removes access, it does not wait for the consumers'
next refresh: it pushes a signed **tombstone** to every peer holding an
active link for the subject. Its own link table is the list of peers to
tell.

| Change on the issuer | Tombstone |
|---|---|
| A guest revoked from an account (subscription or subscription manager) | `grantsChanged` for that email |
| A guest role deleted, or its permission changed | `grantsChanged` for every guest holding it |
| A guest's flags changed on an account | `grantsChanged` for the account's guests |
| An account deleted, deactivated, archived or disapproved | `grantsChanged` for its users and guests |
| A user deleting their own account | `grantsChanged` for that account's users |
| A link revoked | `linkRevoked` |

The consumer records the tombstone in the database and drops the stored
assertion at once, on every pod. An assertion issued before the tombstone
is never stored again, so a refresh already in flight cannot bring the old
grants back. `grantsChanged` keeps the link: the next resolution asks the
issuer, which answers with what is left. `linkRevoked` also revokes the
consumer's copy of the link.

Delivery is at least once. A tombstone is written to an outbox in the same
request, and the federation dispatcher pushes it within a second, retrying
a failed push after 5 s, doubling up to 15 min, and giving up after 20
attempts (about four hours). A consumer that was down when the push was
attempted drops every stored assertion when it starts, before it serves
anything, so it asks each issuer again before serving any grant, and
receives the tombstone with the next retry.

What the push does not reach is bounded by the assertion lifetime, which is
why it stays short. Changes outside the table above (a tenant deleted, a
role's children changed) reach the consumer with its next refresh, within
`assertionRefreshSecs`. A partition that heals while both instances stay up
lets the consumer serve a stored assertion for up to `assertionRefreshSecs`
before it asks again.

Quarantining a peer drops every assertion stored from it; removing a peer
drops them with it.

## What the issuer answers

The issuer answers "unknown" unless the reference names an **active link
with the caller**, the caller is an active peer and `profile.v1` was
negotiated. It also answers "unknown" for a subject it denies itself: an
inactive user, or an inactive, archived or deleted account. The answer
reveals nothing else.

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
