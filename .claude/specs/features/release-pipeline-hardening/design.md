# Design: Release Pipeline Hardening

**Feature:** release-pipeline-hardening
**Spec:** `./spec.md`
**Status:** Design
**Created:** 2026-07-11

Implements REL-01..15 from `spec.md`, plus two follow-ups raised directly by the user in this
round (tracked as REL-16/REL-17 below): a crates.io publish retry/backoff story, and the 9.0.0
major-bump-via-RCs process.

---

## 1. Current state (verified by reading the actual workflow files)

| File | Current behavior | Problem |
|---|---|---|
| `release-prerelease.yml` / `release-stable.yml` | Single `cargo release <level> --execute --no-confirm` step. `CARGO_REGISTRY_TOKEN` from a repo secret. | Publish, tag, and push all happen inside one opaque command. If publish fails partway (crates.io throttling), the version-bump commit was made locally but never pushed (job is ephemeral) — a retry re-runs the whole command from the original base version, re-bumping to a *different* version and stranding the partially-published crates at an orphaned, never-tagged version. |
| `docker-release.yml` | Triggers on tag push or `workflow_dispatch` with a `tag` input. Computes `steps.tag.outputs.version` from `inputs.tag \|\| github.ref_name`, but the `actions/checkout@v4` step has **no `ref:`** — it always checks out whatever ref triggered the workflow (for `workflow_dispatch`, that's the branch the dispatch was run from, not the tag). | REL-09/REL-10: a manual rebuild of an old tag builds from the current default branch, so the image's `VERSION` label and its actual content disagree. |
| *(no workflow)* | Nothing ever creates a GitHub Release. | REL-01..04: Releases (`v8.2.1` GA, a stale `8.3.1-rc.2` Draft) are stuck behind the images/crates that already moved to `8.3.1-rc.5`. |
| `docker-release.yml` | No provenance, no signing. | REL-12/13: consumers can't verify image origin. |
| `release.toml` | `tag-name = "{{version}}"` (no `v` prefix) — already the target convention. | REL-14/15: just needs documenting; no renaming of historical `v*` tags. |
| `release.toml` | `[rate-limit] new-packages = 5 / existing-packages = 30` already present. | cargo-release *does* have a built-in rate-limiter (queries crates.io's own published-count API and paces publishes to stay under crates.io's documented limits). This existing config is correct and is kept — the gap is what happens when crates.io still throttles anyway (global account-wide limits, clock/window edge cases), which the current single-shot workflow has no graceful recovery from. |

---

## 2. Ordering principle (unchanged from spec.md — and from cargo-release's own default)

```
version bump ──▶ commit (local, not pushed) ──▶ publish (rate-limited) ──▶ tag ──▶ push
                                                        │
                                               success  │  failure
                                                        ▼         ╲
                                              (continues above)    ╲──▶ job fails here.
                                                                        Nothing was ever
                                                                        pushed (tag or
                                                                        branch) -- remote
                                                                        state is untouched,
                                                                        exactly as if the
                                                                        run never happened.
```

**Revised from an earlier draft of this doc** (caught in review): the first version of this design
pushed the bump commit to `develop`/`main` immediately after bump, *before* publish, reasoning
that it would make a failed publish resumable without re-bumping. That trades away a property the
*current* single-command flow already has for free — cargo-release's own default order is
version → commit → publish → tag → **push last** — so today, if publish fails, the branch is
never touched at all. Pushing early would mean a failed publish leaves `develop` carrying a
"Release 9.0.0-rc.1" commit for a version that was never actually released (no tag, not fully on
crates.io) — anyone branching off `develop` in that window sees a lie. Keeping this repo's design.
below preserves push-last.

Splitting `cargo release` into separate step invocations (bump+commit → publish → tag+push) inside
the **same job** (same checkout, same working directory, no push in between) still lets us slot the
OIDC auth step in between bump and publish, without ever pushing before publish succeeds — the job
simply fails and ends before reaching the tag/push steps if publish fails, same as today.

**Recovery after a failed publish depends on the bump kind:**
- **Fixed-target bumps** (`major`, `minor`, `patch`, `release`): the target version is deterministic
  regardless of how many times you compute it. Simply **re-dispatch the same workflow** — it
  recomputes the identical target version, and `cargo release publish` already skips crates that
  got published in the previous (failed) attempt, so the retry only publishes what's left, then
  proceeds to tag+push normally. No manual intervention beyond re-running the workflow.
- **Relative bumps** (`rc`, `beta`): re-dispatching increments again (`rc.1` → `rc.2`), it does not
  retry `rc.1`. The simplest safe recovery is to **abandon the failed rc/beta attempt** — the
  handful of crates that did publish at that orphaned version stay on crates.io (harmless; version
  numbers aren't reused, nothing references that untagged version) — and just cut the next rc/beta
  number, which starts clean. This needs no special tooling, only documenting (§8).

---

## 3. `release-prerelease.yml` / `release-stable.yml` — staged steps

Both files get the same shape (only the `release_type` choices and the `if: github.ref` branch
differ, as today). Replace the single "Run release" step with:

```yaml
    permissions:
      contents: write
      id-token: write   # REL-05: OIDC token for crates-io-auth-action

    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          token: ${{ secrets.RELEASE_TOKEN }}   # unchanged — must cascade to tag-triggered workflows
          fetch-depth: 0
      # ... (Rust toolchain, rust-cache, cargo-release, git-cliff install — unchanged) ...

      - name: Configure git identity
        run: |
          git config user.name "Samuel Galvão Elias"
          git config user.email "sgelias@outlook.com"

      # release-prerelease.yml only: a `custom_version` input overrides
      # release_type, for the one bump cargo-release's LEVEL keywords can't
      # express (the first RC of a new major -- see §9). Both are exported
      # as env vars and resolved to $TARGET via a shell step, never
      # interpolated directly into a `run:` block (workflow-injection
      # hardening, flagged by this repo's own security hook).
      - name: Bump version, changelog, commit (dry run)
        if: inputs.dry_run == true
        run: cargo release "$TARGET" --no-publish --no-tag --no-push

      - name: Bump version, changelog, commit (no publish/tag/push yet)
        if: inputs.dry_run == false
        run: |
          cargo release "$TARGET" --execute --no-confirm --no-publish --no-tag --no-push

      # REL-05/06: short-lived OIDC token, not a stored secret
      - name: Authenticate to crates.io (Trusted Publishing)
        if: inputs.dry_run == false
        id: auth
        uses: rust-lang/crates-io-auth-action@v1

      - name: Publish crates (rate-limited by release.toml's [rate-limit])
        if: inputs.dry_run == false
        env:
          CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}
        run: cargo release publish --execute --no-confirm
        # If this step fails partway (crates.io still throttling despite
        # [rate-limit]), nothing has been pushed -- the bump commit only
        # exists in this ephemeral runner's working tree, exactly like
        # today's single-command flow. See docs/book/src/08-release-process.md
        # for the recovery procedure (differs for fixed vs. rc/beta bumps).

      - name: Tag and push
        if: inputs.dry_run == false
        run: cargo release tag --execute --no-confirm && cargo release push --execute --no-confirm
```

Notes:
- `--no-publish --no-tag --no-push` on the bump step, then a dedicated `publish` step, then a
  dedicated `tag`+`push` step, all in the *same job* with no push in between: these map 1:1 onto
  cargo-release's own documented sub-steps (`version`, `commit`, `publish`, `tag`, `push` —
  confirmed via `cargo release --help`) and preserve the exact same "nothing pushed if publish
  fails" property the current single-command flow already has — the only thing that changes is
  where the OIDC auth step slots in.
- `rust-lang/crates-io-auth-action@v1` (REL-05/06) exchanges the job's OIDC token for a
  short-lived (≤30 min) crates.io token, exported as `steps.auth.outputs.token`. This is fed to
  `CARGO_REGISTRY_TOKEN` for just the publish step — never a repo secret.
- **External prerequisite (cannot be done from this repo's code):** each of the 15 publishable
  crates must have **Trusted Publishing** configured on its own crates.io settings page (Owner →
  Publishing → GitHub Actions), pointing at this repo + the `release-prerelease.yml`/
  `release-stable.yml` workflow filenames. This is a **manual, one-time, per-crate,
  crates.io-account-owner action** — flagged clearly in tasks.md, not something I can automate.
  Until it's done for a crate, that crate's publish step will fail auth; keep
  `CARGO_REGISTRY_TOKEN` (the secret) as a fallback in `publish-crates.yml` (see below) until all
  15 crates have Trusted Publishing configured, then remove the secret (REL-07).

---

## 4. `publish-crates.yml` — kept as manual OIDC-first fallback

Per spec.md's Recommended Approach ("Keep `publish-crates.yml` as a manual OIDC-based fallback
only"): same OIDC auth step added, `CARGO_REGISTRY_TOKEN` env now sourced from
`steps.auth.outputs.token`. This is also the **documented manual-retry entry point** — an operator
can run this workflow with `workflow_dispatch` (dry_run=false) any time after a partial publish
failure; `cargo release publish` skips already-published crates automatically.

---

## 5. `docker-release.yml` — ref fix, provenance, signing, split into two jobs

```yaml
permissions:
  contents: read
  packages: write

jobs:
  image:
    permissions:
      contents: read
      packages: write
      id-token: write        # REL-13: cosign keyless
      attestations: write     # REL-12: build provenance
    outputs:
      version: ${{ steps.tag.outputs.version }}
      prerelease: ${{ steps.tag.outputs.prerelease }}
      prerelease_type: ${{ steps.tag.outputs.prerelease_type }}
      digest: ${{ steps.push.outputs.digest }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          ref: ${{ inputs.tag || github.ref_name }}   # REL-09/10: THE FIX -- was missing entirely

      - name: Classify tag
        id: tag
        run: |
          TAG="${{ inputs.tag || github.ref_name }}"
          # ... unchanged classification logic ...

      # `github.repository` is case-preserved ("LepistaBioinformatics/mycelium").
      # docker/metadata-action lowercases internally (why builds work today),
      # but cosign and attest-build-provenance below do NOT -- GHCR/OCI image
      # refs must be all-lowercase or both steps fail. Compute it once, reuse
      # everywhere.
      - name: Lowercase image name
        id: image
        run: echo "name=ghcr.io/$(echo '${{ github.repository }}' | tr '[:upper:]' '[:lower:]')" >> "$GITHUB_OUTPUT"

      - name: Docker metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ steps.image.outputs.name }}
          # ... tags: unchanged (same 3-line type=raw block as today) ...

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with: { registry: ghcr.io, username: ${{ github.actor }}, password: ${{ secrets.GITHUB_TOKEN }} }

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build and push
        id: push
        uses: docker/build-push-action@v6
        with:
          context: .
          file: Dockerfile
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          build-args: VERSION=${{ steps.tag.outputs.version }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      # REL-12
      - name: Attest build provenance
        uses: actions/attest-build-provenance@v1
        with:
          subject-name: ${{ steps.image.outputs.name }}
          subject-digest: ${{ steps.push.outputs.digest }}
          push-to-registry: true

      # REL-13
      - name: Install cosign
        uses: sigstore/cosign-installer@v3

      - name: Sign image (keyless)
        run: |
          cosign sign --yes \
            ${{ steps.image.outputs.name }}@${{ steps.push.outputs.digest }}

  github-release:
    needs: image
    permissions:
      contents: write
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          ref: ${{ inputs.tag || github.ref_name }}
          fetch-depth: 0   # git-cliff needs history

      - name: Install git-cliff
        run: cargo install git-cliff --locked

      - name: Generate release notes
        id: notes
        run: |
          git-cliff --latest --strip header -o /tmp/notes.md
          echo "path=/tmp/notes.md" >> "$GITHUB_OUTPUT"

      # REL-01..04: idempotent -- gh release create errors if the tag already
      # has a Release, so fall back to `edit` in that case instead of failing
      # the job (re-runs, e.g. workflow_dispatch replays, stay safe).
      - name: Create or update GitHub Release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          TAG="${{ needs.image.outputs.version }}"
          PRERELEASE_FLAG=""
          if [ "${{ needs.image.outputs.prerelease }}" = "true" ]; then
            PRERELEASE_FLAG="--prerelease"
          fi
          if gh release view "$TAG" >/dev/null 2>&1; then
            gh release edit "$TAG" --notes-file "${{ steps.notes.outputs.path }}" $PRERELEASE_FLAG
          else
            gh release create "$TAG" --title "$TAG" --notes-file "${{ steps.notes.outputs.path }}" $PRERELEASE_FLAG
          fi
```

Two jobs instead of one so a `github-release` failure (e.g. `git-cliff` hiccup) doesn't need a
full image rebuild+resign to retry — `needs: image` still lets `workflow_dispatch` re-run the
whole workflow for a tag when needed (idempotent either way, since publish/sign target the same
digest and Release creation checks for an existing Release first).

`gh release create/edit` uses `github.token` (the built-in `GITHUB_TOKEN`), not the same PAT used
for the release-bump workflows -- creating a Release doesn't need to *cascade* to another
workflow, unlike the tag push, so the default token's non-cascading behavior is fine here and
keeps permissions minimal (`contents: write` only, no PAT needed for this job).

---

## 6. Cross-cutting hardening

- **Least-privilege `permissions:`**: every job above now declares only what it uses (shown
  inline). Top-level workflow `permissions:` stays `contents: read` where a job doesn't need
  more; each job overrides as needed (GitHub Actions job-level `permissions:` takes precedence).
- **Pin third-party actions to full commit SHA**: `docker/metadata-action`, `docker/login-action`,
  `docker/setup-buildx-action`, `docker/build-push-action`, `actions/attest-build-provenance`,
  `sigstore/cosign-installer`, `rust-lang/crates-io-auth-action`, `dtolnay/rust-toolchain`,
  `Swatinem/rust-cache` — pin each to the SHA of the tag currently referenced (recorded with a
  `# vX.Y.Z` trailing comment per GitHub's own recommended convention). Exact SHAs resolved at
  implementation time via `gh api repos/<owner>/<repo>/git/refs/tags/<tag>`.
- **GitHub Environment `release`**: `release-stable.yml`, `release-prerelease.yml`, and
  `publish-crates.yml`'s jobs gain `environment: release`. **External prerequisite:** the
  `release` Environment plus its required-reviewers rule must be created in the repo's Settings →
  Environments UI — not something expressible in workflow YAML alone. Flagged in tasks.md.
- **Removing `CARGO_REGISTRY_TOKEN`** (REL-07): only after every crate has Trusted Publishing
  configured AND at least one full release cycle (an RC) has succeeded end-to-end using OIDC.
  Sequenced last in tasks.md, deliberately not bundled with the workflow-file edits.

---

## 7. Naming convention documentation (REL-14/15)

No code change (`tag-name = "{{version}}"` in `release.toml` is already the target, no `v`
prefix). Add a short section to `.claude/specs/project/STATE.md` (or a new
`docs/book/src/08-release-process.md`, see below) recording: tags are `X.Y.Z` /
`X.Y.Z-beta.N` / `X.Y.Z-rc.N`, no `v` prefix; historical `v*` tags (pre-8.3.0) are left as-is,
never renamed; image tags mirror this (`:X.Y.Z`+`:latest` stable, `:X.Y.Z-rc.N`+`:rc`, etc.,
already implemented in `docker-release.yml`'s tag classification step).

---

## 8. REL-16 (new) — crates.io publish retry runbook

Not a code change beyond the staged workflow steps in §3 — a **documented runbook** (new
`docs/book/src/08-release-process.md`) covering:

1. Normal flow: dispatch `release-prerelease.yml` (or `-stable.yml`) with `dry_run: true` first,
   review the bump diff, then re-dispatch with `dry_run: false`.
2. If the **publish** step fails (crates.io throttling despite `release.toml`'s `[rate-limit]`,
   network blip, or a specific crate's own issue): per §2, **nothing has been pushed** — the
   partial run's local commit died with the runner. Recovery depends on what kind of bump it was:
   - **`major`/`minor`/`patch`/`release`** (deterministic target version): just **re-dispatch the
     same workflow**. It recomputes the identical target version; `cargo release publish` skips
     the crates that already succeeded in the previous attempt and only publishes what's left.
   - **`rc`/`beta`** (relative bump): do **not** re-dispatch — it would produce `rc.N+1`, not retry
     `rc.N`. Instead, treat the partial `rc.N`/`beta.N` as abandoned (the handful of crates that
     did publish at that version are harmless orphans on crates.io — untagged, unreferenced,
     nothing depends on that exact version existing) and dispatch the workflow again for the
     **next** rc/beta number. This needs no special tooling.
   - `publish-crates.yml` (manual, OIDC-based) is also available any time as a standalone
     re-publish pass — useful if you want to retry *only* the publish step without touching
     bump/tag/push at all (e.g. crates.io was down when a `major`/`minor` bump's publish step
     failed, and you don't want to re-run the whole bump workflow for unrelated reasons).

---

## 9. REL-17 (new) — 9.0.0 major bump via RCs

Per the user: routine major bump, no known breaking API changes — the "manual adjustment" concern
was general caution, not a specific known blocker. Process (uses existing `cargo release` levels,
no new tooling):

1. From `develop`, dispatch `release-prerelease.yml` with `release_type: rc` — but `rc` **bumps
   the rc *pre-version***, it does not jump to a new major. To go from the current `8.3.1-rc.N`
   line to `9.0.0-rc.1`, cargo-release's `LEVEL` keywords can't compose "major" + "rc" in one call
   (and `--metadata`/`-m` is semver *build* metadata after a `+`, e.g. `9.0.0+rc.1` — **not** a
   pre-release identifier; verified against cargo-release's own docs, this was wrong in an earlier
   draft of this section). The correct approach is cargo-release's other accepted form of its
   `[LEVEL|VERSION]` argument: an **explicit target version string**, which must be valid semver
   and greater than the current version:
   ```
   cargo release 9.0.0-rc.1 --execute --no-confirm --no-publish --no-tag --no-push
   ```
   `release-prerelease.yml` now has a `custom_version` input specifically for this (overrides
   `release_type` when set) — dispatch it with `custom_version: 9.0.0-rc.1`, `release_type` left
   blank. Every subsequent RC in the 9.0.0 line uses the normal `release_type: rc` dropdown again
   (`9.0.0-rc.1` → `rc` level → `9.0.0-rc.2`, etc. — verify via a `dry_run: true` dispatch before
   executing for real, same as any other bump).
2. Each RC goes through the full staged pipeline (§2): bump (local) → publish → tag+push → image
   (signed+attested) → GitHub prerelease. Verify end-to-end (install the RC crate, boot the RC
   image) before cutting the next RC or promoting to stable.
3. Promote to stable with `cargo release release` (cargo-release's dedicated level that strips
   the pre-release suffix, `9.0.0-rc.N` → `9.0.0`) via `release-stable.yml`.
4. This is also the **first real end-to-end exercise of OIDC Trusted Publishing** (§3) — do the
   crates.io Trusted Publishing setup (external prerequisite) before cutting `9.0.0-rc.1`, and keep
   `CARGO_REGISTRY_TOKEN` as a fallback until this cycle proves it works, per §6.

---

## 10. Verification plan

No `actionlint`/`yamllint` available locally — verification instead:
- `python3 -c "import yaml; yaml.safe_load(open(f))"` against every edited workflow file (catches
  YAML syntax errors, not GitHub Actions semantic errors).
- Manual review of every `${{ }}` expression and `needs:`/`outputs:` wiring against GitHub Actions
  documented syntax.
- Confirm the lowercase-image-name step actually lowercases `LepistaBioinformatics/mycelium`
  (test the `tr` one-liner locally) and that both `attest-build-provenance` and `cosign sign`
  reference `steps.image.outputs.name`, never a raw `${{ github.repository }}` interpolation.
- `dry_run: true` dispatch of `release-prerelease.yml` (safe -- `--no-publish --no-tag`, nothing
  irreversible) as the actual functional test once pushed, run by the user (I cannot dispatch
  `workflow_dispatch` runs against the real repo from here in a way that's meaningfully safer than
  the user doing it themselves, and doing so would be taking a real CI action on their behalf
  without being asked).
- Full gate (`cargo fmt`, `cargo build --workspace`, `cargo test --workspace --all`) — unaffected
  by this feature (CI/docs only), run anyway to confirm no accidental source changes.
