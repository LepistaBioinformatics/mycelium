# Release Process

This guide explains how Mycelium releases are cut, using `cargo-release` and `git-cliff`,
orchestrated by GitHub Actions.

## Overview

Mycelium follows [Semantic Versioning](https://semver.org/). Releases are **not** run by hand
locally — they run through GitHub Actions `workflow_dispatch` jobs, because two pieces of the
pipeline only work inside CI:

- **`RELEASE_TOKEN`** (a PAT/App token) is what makes a pushed tag *cascade* to the
  tag-triggered `docker-release.yml` workflow — the built-in `GITHUB_TOKEN` would not.
- **crates.io Trusted Publishing (OIDC)** — the short-lived crates.io token is exchanged via
  GitHub's OIDC identity, which only exists inside an Actions run.

A local `cargo release <level>` (no `--execute`) dry-run is still the right way to preview a bump
before dispatching the real workflow — it needs no credentials and touches nothing.

Tools involved:
- **cargo-release**: version bumping, changelog hook, tagging, publishing.
- **git-cliff**: generates changelog notes from conventional commits — used both for
  `CHANGELOG.md` and as the body of the GitHub Release `docker-release.yml`'s `github-release`
  job creates for every tag.

## Version Semantics

| Version Type | Format | When to Use | Example |
|--------------|--------|-------------|---------|
| **MAJOR** | `X.0.0` | Breaking changes or incompatible API changes | `8.0.0` → `9.0.0` |
| **MINOR** | `x.Y.0` | New features (backward-compatible) | `8.3.0` → `8.4.0` |
| **PATCH** | `x.y.Z` | Bug fixes (backward-compatible) | `8.3.0` → `8.3.1` |

**Tag naming:** no `v` prefix — tags and image tags are `8.3.1`, `8.3.1-rc.1`, never `v8.3.1`.
Historical `v*` tags (pre-`8.3.0`) are left as-is, never renamed.

## The Automated Pipeline

Dispatching `release-prerelease.yml` (from `develop`) or `release-stable.yml` (from `main`) runs,
in one job: **bump → commit (local) → publish (OIDC, rate-limited) → tag → push.** Nothing is
pushed to the branch or to crates.io until the step that does it succeeds; if `publish` fails, the
job simply ends and the remote is untouched — see [Publish Failures & Retry](#publish-failures--retry)
below.

Once the tag is pushed, `docker-release.yml` fires automatically:
1. **`image` job** — builds from that exact tag (fixed ref bug, see below), pushes to GHCR, attests
   build provenance, and signs the image keylessly with cosign.
2. **`github-release` job** — creates (or updates, idempotently) a GitHub Release for the tag, with
   `git-cliff` notes and the correct `prerelease` flag.

You can also trigger `docker-release.yml` manually via `workflow_dispatch` with a `tag` input — for
example to rebuild an older tag. It checks out that exact tag's source (not whatever branch the
dispatch happened to run from), so the rebuilt image's content always matches its version label.

## Pre-release Workflow

Pre-releases progress through `beta` → `rc` stages (dispatch `release-prerelease.yml` from
`develop`, choosing `release_type`). `cargo-release` also supports an `alpha` level, but it isn't
wired into either workflow's dropdown today — use `custom_version` (see below) if you ever need it.

**Example progression**: `8.3.0-beta.1` → `8.3.0-beta.2` → `8.3.0-rc.1` → `8.3.0-rc.2`

**The stages are one-way.** Once a line is at `-rc.N` you cannot dispatch `beta` on it —
cargo-release refuses to walk a pre-release backwards, and reports it as:

```
error: unsupported release level beta, only major, minor, and patch are supported
```

That message is misleading: the `beta` level exists and works fine, it is the `rc` → `beta`
transition that is rejected (`src/ops/version.rs`, `increment_beta`). From `-rc.N` the only
relative moves are `rc` (next RC) or promotion to stable; starting a *new* line at beta means
going through stable first, or naming the version explicitly with `custom_version`.

`release-prerelease.yml`'s pre-flight step catches this before it spends six minutes building
cargo-release, and says which of the two it is.

### Promoting to stable

Dispatch `release-stable.yml` from `main` with `release_type: patch|minor|major` — cargo-release's
`release` level (`X.Y.Z-rc.N` → `X.Y.Z`, dropping the pre-release suffix) is available locally for
a dry-run preview, but isn't exposed as a `release-stable.yml` dropdown option; the stable workflow
always bumps from `main`'s current version, so merge the release branch into `main` first, then
dispatch with the level matching what the pre-release cycle was for (a `9.0.0-rc.N` line promotes
via `major` from the last stable tag once `main` is on the rc's commit — see the 9.0.0 walkthrough
below for the concrete case).

### Back-merge `main` into `develop` (mandatory)

`cargo release` bumps the version **on the branch it runs from**. A stable release therefore
leaves the `chore: Release version X.Y.Z` commit on `main` only, while `develop` still declares
the last pre-release version. `release-prerelease.yml` runs from `develop`, so until that commit
is brought back, every later bump is computed from a stale base:

| `main` | `develop` | dispatch | result |
|---|---|---|---|
| `9.0.0` | `9.0.0-rc.13` | `beta` | hard error (rc → beta, see above) |
| `9.0.0` | `9.0.0-rc.13` | `rc` | **silently cuts `9.0.0-rc.14`** — a version *behind* the published `9.0.0` |

The second row is the dangerous one: nothing fails, and a version lower than the current stable
reaches crates.io and GHCR.

**So, immediately after every stable release**, open a PR bringing `main` back into `develop`:

```bash
git checkout develop && git pull
git checkout -b chore/backmerge-X.Y.Z-into-develop
git merge origin/main
# resolve nothing in the normal case -- only main touched the version lines
git push -u origin chore/backmerge-X.Y.Z-into-develop
gh pr create --base develop
```

`release-prerelease.yml`'s pre-flight compares `develop`'s version against `main`'s and refuses to
run while `develop` is behind, so a forgotten back-merge now fails loudly at second zero instead of
producing a bad version. The comparison is by version, not by commit ancestry, so it is satisfied
whether the back-merge PR is merged or squashed.

## Publish Failures & Retry

If the **publish** step fails partway (crates.io throttling despite `release.toml`'s
`[rate-limit]`, a network blip, or a single crate's own issue), recovery depends on what kind of
bump it was — nothing was pushed, so the remote branch and tags are exactly as they were before
you dispatched:

- **`patch` / `minor` / `major` / `release`** (deterministic target version): just **re-dispatch
  the same workflow**. It recomputes the identical target version, and `cargo release publish`
  already skips crates that succeeded in the previous attempt — the retry only publishes what's
  left, then proceeds to tag+push normally.
- **`beta` / `rc`** (relative bump): re-dispatching increments again (`rc.1` → `rc.2`), it does
  **not** retry `rc.1`. Treat the partial `rc.N`/`beta.N` as abandoned — the handful of crates that
  did publish at that version are harmless orphans on crates.io (untagged, unreferenced, nothing
  depends on that exact version existing) — and dispatch the workflow again for the **next**
  rc/beta number.
- **`publish-crates.yml`** is also available any time as a standalone re-publish pass (same OIDC
  auth), with an `unpublished_only` input (`cargo release publish --unpublished`) to scope it to
  just the packages not yet published at their current `Cargo.toml` version — useful if you want to
  retry only the publish step without touching bump/tag/push.

## Major Version Bump via Release Candidates (9.0.0 walkthrough)

Going from the `8.3.x` line to `9.0.0` is a routine major bump with no breaking changes planned —
release it as a series of RCs first, same as any other release, with one wrinkle: cargo-release's
`major` LEVEL alone jumps straight to a **stable** `9.0.0`, and its `--metadata`/`-m` flag sets
semver *build* metadata after a `+` (e.g. `9.0.0+rc.1`), **not** a pre-release identifier — neither
composes "major" with "rc" the way you'd want. The fix is `release-prerelease.yml`'s
`custom_version` input, which accepts an explicit target version string (cargo-release's other
accepted form of its `[LEVEL|VERSION]` argument):

1. Dispatch `release-prerelease.yml` from `develop` with `custom_version: 9.0.0-rc.1` (leave
   `release_type` blank). Do a `dry_run: true` pass first and review the diff.
2. Every subsequent RC uses the normal `release_type: rc` dropdown again — `9.0.0-rc.1` → `rc` →
   `9.0.0-rc.2`, etc. Verify each RC end-to-end (install the RC crate, boot the RC image) before
   cutting the next one.
3. Before cutting `9.0.0-rc.1`, make sure crates.io Trusted Publishing is configured for every
   workspace crate (see below) — this is the first real end-to-end exercise of OIDC publishing;
   keep `CARGO_REGISTRY_TOKEN` as a fallback until this cycle proves it works.
4. Once satisfied, merge to `main` and dispatch `release-stable.yml` with `release_type: major` to
   drop the `-rc.N` suffix and cut `9.0.0` stable.

## crates.io Trusted Publishing setup (one-time, per crate)

OIDC-based publishing (no long-lived `CARGO_REGISTRY_TOKEN`) requires configuring **Trusted
Publishing** individually for each of this workspace's ~15 published crates, on crates.io itself —
this can't be done from workflow YAML:

1. For each crate: crates.io → the crate's page → Settings → Publishing → add a GitHub Actions
   trusted publisher pointing at `LepistaBioinformatics/mycelium` and the workflow filename
   (`release-prerelease.yml`, `release-stable.yml`, or `publish-crates.yml` — add all three, since
   any of them may run the publish step).
2. Until every crate has this configured, publish steps fall back to the `CARGO_REGISTRY_TOKEN`
   repository secret automatically.
3. Once every crate is confirmed working via OIDC (ideally proven by a full RC cycle), remove the
   `CARGO_REGISTRY_TOKEN` secret from the repository.

## Changelog Management

Mycelium uses `git-cliff` to automatically generate changelogs from conventional commits — both
`CHANGELOG.md` (via `release.toml`'s `pre-release-hook`) and each tag's GitHub Release body (via
`docker-release.yml`'s `github-release` job).

### Conventional Commit Format

All commits should follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Supported types**:

| Type | Description | Changelog Section |
|------|-------------|------------------|
| `feat` | New feature | 🚀 Features |
| `fix` | Bug fix | 🐛 Bug Fixes |
| `docs` | Documentation | 📚 Documentation |
| `perf` | Performance improvement | ⚡ Performance |
| `refactor` | Code refactoring | 🚜 Refactor |
| `style` | Code style changes | 🎨 Styling |
| `test` | Test changes | 🧪 Testing |
| `chore` | Maintenance tasks | ⚙️ Miscellaneous Tasks |

**Examples**:

```bash
# Feature commit
git commit -m "feat(auth): add passwordless authentication

Implements magic link authentication flow for users.
Users can now sign in by clicking a link sent to their email.

Fixes #110"

# Bug fix commit
git commit -m "fix(api): resolve null pointer in user endpoint

Fixes #123"

# Breaking change
git commit -m "feat(core): redesign authentication API

BREAKING CHANGE: The authentication API has been completely redesigned.
See migration guide for details.

Fixes #150"
```

### Previewing changelogs locally

```bash
# Preview unreleased changes
git-cliff --unreleased

# Notes for a specific already-tagged version (what the GitHub Release job runs)
git-cliff --latest --strip header
```

`CHANGELOG.md` itself is updated automatically by `release.toml`'s `pre-release-hook` as part of
the bump step — you don't need to run this by hand as part of a normal release.

### Changelog Configuration

The changelog format is configured in `cliff.toml` at the repository root. This file defines:
- Commit parsing rules
- Grouping and sorting
- Output format
- Template customization

## Dry Run (Recommended)

Always preview a release locally before dispatching the real workflow:

```bash
# Dry run (default -- no --execute flag). Needs no credentials, touches nothing.
cargo release rc

# Review the output carefully:
# - Version changes
# - Files that will be modified
# - Git commands that will run
# - Tags that will be created
```

Every release workflow also has its own `dry_run` input (`--no-publish --no-tag --no-push`) for
the same preview, run inside CI.

## Release Checklist

Use this checklist before dispatching a stable release:

- [ ] All tests pass: `cargo test --workspace --all`
- [ ] Code is properly formatted: `cargo fmt --all -- --check`
- [ ] No security vulnerabilities: `cargo audit`
- [ ] Documentation is up-to-date
- [ ] All commits follow conventional commit format
- [ ] All CI checks pass on the branch being released
- [ ] Team review is complete (for major/minor releases)
- [ ] Local dry run reviewed: `cargo release <level>`
- [ ] Workflow `dry_run: true` dispatch reviewed
- [ ] After a stable release: `main` back-merged into `develop` (see above)

## Release Configuration

The project's release behavior is configured in `release.toml` at the repository root.

Key configurations include:
- **Pre-release hooks**: regenerate `CHANGELOG.md` via `git-cliff` before tagging
- **`[rate-limit]`**: paces crates.io publishes (`new-packages`/`existing-packages`) to stay under
  crates.io's documented limits — see [Publish Failures & Retry](#publish-failures--retry) for what
  happens if it still isn't enough
- **Version bumping**: `shared-version = true` — all workspace crates move together
- **Git operations**: `tag-name = "{{version}}"` (no `v` prefix), commit/tag message templates
- **Publishing**: `publish = true`, consumed by the `publish` step of the staged pipeline

## Best Practices

1. **Test thoroughly**: run the full test suite before any release.
2. **Use dry runs**: both the local `cargo release <level>` preview and each workflow's `dry_run`
   input.
3. **Follow the progression**: don't skip stages (beta → rc → stable) for anything but a hotfix
   patch.
4. **Write good commits**: use conventional commits — they drive both `CHANGELOG.md` and every
   tag's GitHub Release notes.
5. **Coordinate releases**: communicate with the team for major/minor releases.
6. **Never publish crates.io by hand**: always go through a workflow (OIDC token only exists in
   CI); a stray local `cargo publish` bypasses the pipeline entirely and won't produce a tag, image,
   or Release.

## Troubleshooting

### Workflow fails: "Either release_type or custom_version must be set"

`release-prerelease.yml` requires one of the two — leave `release_type` on its default `beta`/`rc`
choice unless you specifically need `custom_version` (see the 9.0.0 walkthrough above).

### Changelog not generating correctly

```bash
# Verify conventional commit format
git log --oneline -n 10

# Test cliff configuration
git-cliff --unreleased

# Check cliff.toml configuration
cat cliff.toml
```

### A GitHub Release didn't get created for a tag

`docker-release.yml`'s `github-release` job depends on the `image` job succeeding first
(`needs: image`) — check the `image` job's logs (build, provenance attestation, cosign signing) for
the actual failure; the Release step itself is idempotent and safe to re-run via `workflow_dispatch`
with that tag once the underlying issue is fixed.

### Wrong version incremented

```bash
# Use a local dry run first to verify: cargo release <level>
# If a workflow already ran with the wrong level and nothing was pushed yet
# (publish failed before tag+push), just re-dispatch with the correct level --
# no cleanup needed, nothing reached the remote.
# If it DID reach the remote (tag pushed), coordinate a fix manually --
# do not delete/force-push a tag that crates.io/GHCR/a Release already reference
# without understanding the full blast radius first.
```

## Additional Resources

- [Semantic Versioning Specification](https://semver.org/)
- [Conventional Commits Specification](https://www.conventionalcommits.org/)
- [cargo-release Documentation](https://github.com/crate-ci/cargo-release)
- [git-cliff Documentation](https://git-cliff.org/)
- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [Contributing Guide](../../CONTRIBUTING.md#release-process)
