# Release Pipeline Hardening Specification

## Problem Statement

The release cycle produces git tags (via `cargo release`) that build & push Docker images to
GHCR automatically, but **no workflow ever creates a GitHub Release**. As a result the GHCR
images are out of sync with GitHub Releases: images exist up to `8.3.1-rc.5` while the newest
*published* Release is still `v8.2.1` (plus a stale `8.3.1-rc.2` Draft). On top of the missing
release step, the pipeline relies on a long-lived `CARGO_REGISTRY_TOKEN` secret, produces no
supply-chain provenance/signatures for images, and has a `workflow_dispatch` bug that builds an
image from the wrong source ref. This feature makes tag → crates.io publish → GitHub Release →
signed GHCR image a single, atomic, secure, and reproducible flow.

## Goals

- [ ] Every published tag results in a **GitHub Release** (pre-release flag for `beta`/`rc`),
      with notes generated from `git-cliff`/`CHANGELOG.md` — zero manual release steps.
- [ ] Every published tag results in **exactly one GHCR image built from that tag's source**,
      tagged consistently, with no `workflow_dispatch` ref drift.
- [ ] crates.io publishing uses **Trusted Publishing (OIDC)** — no long-lived
      `CARGO_REGISTRY_TOKEN` secret stored in the repo.
- [ ] GHCR images carry **build provenance attestation** and are **signed (cosign keyless)**,
      so downstream consumers can verify origin (SLSA-aligned).
- [ ] Tag/version/image naming is **consistent and documented** (resolve the `v`-prefix drift).
- [ ] A failure at any stage leaves a **recoverable, non-contradictory state** (re-runnable on
      the tag), and the irreversible step (crates.io publish) runs **before** the tag is pushed.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Webapp (`mycelium-webapp`) release/deploy pipeline | Separate module; this spec is gateway-only |
| SDK-py (`mycelium-sdk-py`) PyPI publishing | Separate module & registry; different flow |
| Dokploy / production deploy automation | Deployment consumes images; out of the release-artifact scope |
| Retroactively creating Releases for pre-`8.3.1` tags | One-off cleanup, tracked as a manual todo — not pipeline logic |
| Yanking / republishing already-published crates | crates.io publish is irreversible; recovery is operational, not automated |
| Multi-arch (arm64) image builds | Current images are single-arch; can be a later P3 enhancement |

---

## User Stories

### P1: Automated GitHub Release on every tag ⭐ MVP

**User Story**: As a maintainer, I want a GitHub Release created automatically for every release
tag so that Releases stay in sync with tags and GHCR images.

**Why P1**: This is the direct cause of the observed desync — the missing piece.

**Acceptance Criteria**:

1. WHEN a version tag (`X.Y.Z` or `X.Y.Z-beta.N` / `X.Y.Z-rc.N`) is pushed THEN the pipeline
   SHALL create a GitHub Release for that tag.
2. WHEN the tag is a pre-release (`-beta.` / `-rc.`) THEN the Release SHALL be flagged
   `prerelease = true`; WHEN it is a stable `X.Y.Z` tag THEN it SHALL be flagged as the latest
   stable Release.
3. WHEN the Release is created THEN its body SHALL be populated from `git-cliff` /
   `CHANGELOG.md` notes for that version.
4. WHEN a Release for that tag already exists THEN the step SHALL be idempotent (update or no-op,
   never fail the pipeline nor duplicate).

**Independent Test**: Push a throwaway `0.0.0-rc.1` tag on a test branch → a pre-release GitHub
Release appears with changelog notes; re-running the workflow does not duplicate it.

---

### P2: Trusted Publishing for crates.io (OIDC, no stored token)

**User Story**: As a maintainer, I want crates.io publishing to authenticate via GitHub OIDC so
that no long-lived registry token is stored in repository secrets.

**Why P2**: Removes the highest-value standing secret; short-lived (≤30 min) scoped tokens.

**Acceptance Criteria**:

1. WHEN the publish job runs THEN it SHALL obtain a short-lived crates.io token via
   `rust-lang/crates-io-auth-action@v1` using `permissions: id-token: write`.
2. WHEN publishing THEN `cargo release publish` (or `cargo publish`) SHALL consume the token via
   `CARGO_REGISTRY_TOKEN` from the action output, not from a repository secret.
3. WHEN crates.io Trusted Publishing is configured THEN the stored `CARGO_REGISTRY_TOKEN` secret
   SHALL be removed from the repository.
4. WHEN the crates.io publish fails THEN the tag SHALL NOT be pushed (publish runs before tag),
   leaving no orphan tag/image/release.

**Independent Test**: Run the release in dry-run → auth step exchanges OIDC successfully; confirm
no `CARGO_REGISTRY_TOKEN` remains under repo secrets after cutover.

---

### P2: Reproducible, correctly-sourced GHCR image

**User Story**: As a maintainer, I want the image to always be built from the exact tagged
commit so that the image content matches its version label.

**Why P2**: The current `workflow_dispatch` path checks out the default branch, not `inputs.tag`.

**Acceptance Criteria**:

1. WHEN the image workflow is triggered by tag push THEN it SHALL check out the tag ref.
2. WHEN the image workflow is triggered via `workflow_dispatch` with a `tag` input THEN it SHALL
   check out that tag ref (`ref: ${{ inputs.tag || github.ref_name }}`), never the default branch.
3. WHEN building THEN it SHALL pass `build-args: VERSION=<tag>` and produce image tags per the
   documented tagging strategy (`:X.Y.Z` + `:latest` for stable; `:X.Y.Z-rc.N` + `:rc`; etc.).
4. WHEN the same tag is built twice THEN the resulting image SHALL be reproducible from identical
   source (no drift).

**Independent Test**: `workflow_dispatch` for an existing older tag → the pushed image's embedded
`VERSION` and source match that tag, not `main`.

---

### P2: Supply-chain provenance & signing for images

**User Story**: As a downstream operator, I want to verify that a GHCR image was built by this
repo's release workflow so that I can trust its origin.

**Why P2**: "Most secure" requirement; enables SLSA-style verification.

**Acceptance Criteria**:

1. WHEN an image is pushed THEN a build provenance attestation SHALL be generated via
   `actions/attest-build-provenance` for the image digest.
2. WHEN an image is pushed THEN it SHALL be signed keyless via cosign (Sigstore/Fulcio) using
   `permissions: id-token: write`, bound to the workflow's OIDC identity.
3. WHEN a consumer runs `cosign verify` with the expected identity/issuer THEN verification SHALL
   succeed for released images.

**Independent Test**: `cosign verify --certificate-identity-regexp ... --certificate-oidc-issuer https://token.actions.githubusercontent.com ghcr.io/.../mycelium:<tag>` succeeds.

---

### P3: Consistent, documented version/tag/image naming

**User Story**: As a maintainer, I want one documented naming convention so that tags, images,
and Releases line up predictably.

**Why P3**: Cleanup of historical `v`-prefix drift (`v8.2.1` vs `8.3.0`); important but not blocking.

**Acceptance Criteria**:

1. WHEN a decision is recorded THEN either `tag-name = "{{version}}"` (no `v`) or
   `tag-name = "v{{version}}"` SHALL be applied consistently in `release.toml` **and** Cargo.toml
   `[workspace.metadata.release]`.
2. WHEN the convention changes THEN `docker-release.yml` tag classification regex and any
   downstream references SHALL be updated to match.
3. WHEN documented THEN the convention SHALL be recorded in STATE.md / codebase docs.

---

## Edge Cases

- WHEN `git-cliff` produces empty notes for a version THEN the Release SHALL still be created
  with a minimal auto-generated body (never fail).
- WHEN the tag push webhook is missed (as happened for `8.3.1-rc.2`) THEN `workflow_dispatch`
  with the tag SHALL fully reproduce crates publish (if needed) + image + Release.
- WHEN a stable tag is pushed THEN `:latest` SHALL move to it; WHEN a pre-release tag is pushed
  THEN `:latest` SHALL NOT move.
- WHEN crates.io reports "already published" for a crate (retry after partial publish) THEN the
  publish step SHALL treat it as success/skip, not hard-fail the pipeline.
- WHEN the image build succeeds but signing/attestation fails THEN the pipeline SHALL fail
  visibly (no silently-unsigned released image) and be safely re-runnable on the tag.
- WHEN two release workflows race THEN concurrency groups SHALL serialize them.

---

## Recommended Approach (secure target architecture)

Recorded here per request; concrete YAML belongs in `design.md`.

**Ordering principle:** the irreversible step (crates.io publish) runs **inside the bump job,
before the tag is pushed** — `cargo release`'s native order is bump → commit → publish → tag →
push. If publish fails, no tag exists and nothing downstream fires. Recoverable steps (image,
Release) run **after**, triggered by the tag, and are re-runnable.

1. **Release bump job** (`release-stable.yml` / `release-prerelease.yml`, `workflow_dispatch`):
   - `permissions: { contents: write, id-token: write }`.
   - `rust-lang/crates-io-auth-action@v1` → export `CARGO_REGISTRY_TOKEN` (OIDC, short-lived).
   - `cargo release <type> --execute --no-confirm` (bump + changelog + publish + tag + push).
   - Push uses `RELEASE_TOKEN` (PAT/App) so the tag push **cascades** to tag-triggered workflows
     (the built-in `GITHUB_TOKEN` would NOT cascade).

2. **Tag-triggered artifact workflow** (`docker-release.yml`, extended — on `push: tags`):
   - Job `image`: `permissions: { contents: read, packages: write, id-token: write,
     attestations: write }`; checkout the tag ref; build from `Dockerfile`; push to GHCR;
     `actions/attest-build-provenance`; cosign keyless sign.
   - Job `github-release` (`needs: image`): `permissions: { contents: write }`; create/update
     the GitHub Release from `git-cliff` notes; `--prerelease` for `beta`/`rc`.

3. **Cross-cutting hardening:**
   - Least-privilege `permissions:` per job (default read).
   - Pin third-party actions to full commit SHA.
   - Optional GitHub `Environment: release` with required reviewers gating publish.
   - Remove `CARGO_REGISTRY_TOKEN` repo secret after Trusted Publishing cutover.
   - Keep `publish-crates.yml` as a manual OIDC-based fallback only.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| --- | --- | --- | --- |
| REL-01 | P1: Auto GitHub Release | Design | Pending |
| REL-02 | P1: pre-release vs stable flagging | Design | Pending |
| REL-03 | P1: changelog notes in Release | Design | Pending |
| REL-04 | P1: idempotent Release creation | Design | Pending |
| REL-05 | P2: OIDC Trusted Publishing auth | Design | Pending |
| REL-06 | P2: token from action, not secret | Design | Pending |
| REL-07 | P2: remove stored CARGO_REGISTRY_TOKEN | Design | Pending |
| REL-08 | P2: publish-before-tag ordering | Design | Pending |
| REL-09 | P2: image built from correct tag ref | Design | Pending |
| REL-10 | P2: workflow_dispatch ref fix | Design | Pending |
| REL-11 | P2: documented image tag strategy | Design | Pending |
| REL-12 | P2: build provenance attestation | Design | Pending |
| REL-13 | P2: cosign keyless signing | Design | Pending |
| REL-14 | P3: consistent tag/version naming | Design | Pending |
| REL-15 | P3: naming documented in STATE/docs | Design | Pending |

**ID format:** `REL-[NUMBER]`
**Status values:** Pending → In Design → In Tasks → Implementing → Verified
**Coverage:** 15 total, 0 mapped to tasks, 15 unmapped ⚠️ (Tasks phase pending)

---

## Success Criteria

- [ ] For a full `beta → rc → stable` cycle, each pushed tag has: a matching GitHub Release, a
      matching signed GHCR image built from that tag, and (for the bump) published crates.
- [ ] No `CARGO_REGISTRY_TOKEN` secret exists in the repository; publishing still works via OIDC.
- [ ] `cosign verify` succeeds against a released image with the expected workflow identity.
- [ ] A deliberately failed publish leaves no orphan tag/image/Release.
- [ ] `workflow_dispatch` rebuild of an old tag produces an image whose source matches that tag.
- [ ] Tag/image/Release naming convention is single and documented.
