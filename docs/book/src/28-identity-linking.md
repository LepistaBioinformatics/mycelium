# Identity Linking Across Instances

Instance federation lets one Mycelium instance accept grants that another
instance holds for the same person. The anchor of that trust is an
**identity link**: an explicit, user-made link between an account on this
instance (the *consumer*) and the person's account on a declared peer (the
*issuer*). No grant flows between instances without one.

> Requires a gateway built with the `federation` feature, a
> `[core.federation]` section, federation switched on at runtime
> (`federation.setEnabled`), and the issuer declared and `active` as a peer
> on the consumer (and the consumer on the issuer). Each gateway's own
> `baseUrl` must be listed in `api.allowedOrigins`: the linking pages post
> their forms to the gateway that served them, as the hosted login pages do.

---

## How a link is made

```text
App on B        Gateway B (consumer)            Gateway A (issuer)
   | beginners.federation.startLink
   |-------------->|
   |<-- initiation URL (single use, linkRequestTtlSecs)
   | browser opens it
   |               | page: "link your account to A"
   |               |-- 303 /federation/link/authorize?state=<signed JWS> -->|
   |               |                    hosted login on A (email code)      |
   |               |                    page: "link a@x here to b@x on B?"  |
   |               |<-- POST /federation/link/confirm (signed) -------------|
   |               |    both sides record the link                          |
   |<------------- 303 back to B's /federation/link/done --------------------|
```

1. The user, signed in on B, calls `beginners.federation.listPeers` to see
   the active peers by name, then `beginners.federation.startLink` with the
   peer's `instanceId`. The answer carries a single-use **initiation URL**.
   Only the hash of its secret is stored.
2. Opening the URL shows a page naming the peer. Opening it spends nothing,
   so a link scanner that fetches it does no harm. Continuing spends it and
   sends the browser to A with a **state** signed by B's federation key. The
   state names B, A, the link request, the user's email on B and the page to
   return to.
3. A verifies the state against B's published keys before offering any
   login. It then signs the user in through its **hosted login** (emailed
   code, attempt cap, no framing) as an internal client whose only redirect
   URI is A's own callback.
4. A compares the emails. **They must be equal**, case aside, unless the
   operator set `allowDifferentEmails` in the peer's policy. A then shows
   which account on A will be linked to which account on which instance, and
   waits for an explicit confirmation.
5. On confirmation A records the link, tells B with a signed request, and
   sends the browser back to B. Each side mints an opaque reference for the
   user; neither is an email or a user id. If B refuses or cannot be reached,
   A revokes its own record. If B recorded the link but its answer was lost
   (a timeout), B keeps an active copy that A does not know; it carries no
   grants, since A answers "unknown" for it.

## Removing a link

Either side may revoke, through the API only:

| Method | Who |
|---|---|
| `beginners.federation.listLinks` / `beginners.federation.revokeLink` | The linked user, own links only |
| `federation.links.list` / `federation.links.revoke` | Staff |

Revoking marks the link revoked here and queues a `linkRevoked` tombstone for
the peer, which revokes its copy and drops its stored grants (see
[Revocation](./29-federated-grants.md#revocation)). The local revocation
stands whatever the peer answers; a peer that cannot be reached receives the
tombstone when it is back.

## Limits

- **TOTP is not asked during linking.** The issuer signs the user in with
  the emailed-code hosted login, which has no TOTP step yet. A user who
  protects password logins with TOTP gets a weaker check when linking.
- **External identity providers.** A user who only signs in to the issuer
  through an external IdP cannot link from the issuer's pages, since they
  only offer the hosted login.
- While federation is on, the issuer's hosted login endpoints accept the
  internal client `mycelium-federation-link`, even if `auth.hostedLogin` is
  not configured. Its only redirect URI is the issuer's own callback.
- Signing in on the issuer with an email that has no account there creates
  one, as the hosted login always does. A link to such an account carries no
  grants.

## Settings

| Setting | Where | Meaning |
|---|---|---|
| `linkRequestTtlSecs` | `[core.federation]` | Lifetime of an initiation URL, and of the issuer's pending consent |
| `allowDifferentEmails` | Peer policy | Allows linking accounts whose emails differ (default: no) |
