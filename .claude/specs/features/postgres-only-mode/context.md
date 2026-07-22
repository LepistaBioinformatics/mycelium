# Context: Postgres-Only Mode (resolved gray areas)

**Feature:** postgres-only-mode
**Created:** 2026-07-22

Captured from the discuss phase (user decisions on gray areas that the brief left open).

---

## Q1 — Deployment topology of the new mode

**Answer: Multi-pod (production-grade).**

The Postgres-only mode must run in Kubernetes with multiple replicas, exactly like full mode.
Consequence: the cross-pod coordination that Redis provides today (the notifier's `LPUSH` email
queue) must be re-implemented on Postgres in a multi-consumer-safe way (`SELECT … FOR UPDATE SKIP
LOCKED` or advisory locks) so concurrent pods never double-send an email. This is the primary scope
risk — it is **not** the single-instance in-process poller that standalone mode uses.

## Q2 — What does "default" mean for the Cargo build / published image

**Answer: Keep default image = full.**

`cargo build` with no flags, and the released Docker image, **stay** Postgres+Redis (full mode).
The new Postgres-only mode is an **explicit opt-in** build (its own feature flag / image tag), not
the Cargo `default`. This avoids any breaking change to existing deployments that pull the default
image. The word "default" in the original request was overloaded; the new mode is a third opt-in
mode, not the default build.

## Q3 — KV/cache implementation in the new mode

**Answer: Postgres table.**

The artifact cache (JWKS, profile caches — TTL'd in Redis today) is backed by a Postgres table with
an `expires_at` column: lazy expiry on read + a periodic sweeper. Cross-pod shared and persistent.
Accepted trade-off: cache reads/writes now hit Postgres (Redis existed precisely to keep them off
the hot DB path). This matches "o KV será no Postgres no lugar do Redis" literally. Rejected: the
per-pod in-process moka option and the two-tier moka+Postgres option.
