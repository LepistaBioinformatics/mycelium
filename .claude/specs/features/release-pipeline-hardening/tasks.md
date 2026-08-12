# Tasks: Release Pipeline Hardening

**Feature:** release-pipeline-hardening
**Design:** `./design.md`
**Status:** Code done (REL-T1..T10) — pending manual prerequisites (REL-T11..T14) and final review

Legend: 🤖 = I implement this. 🧑 = requires you to act outside this repo (crates.io UI, GitHub
Settings UI) — I cannot do these; flagged clearly so nothing silently blocks.

- [x] REL-T1 🤖 — Pin every third-party action referenced by the 4 release/publish workflows to a
      full commit SHA (resolve via `gh api repos/<owner>/<repo>/git/refs/tags/<tag>`).
- [x] REL-T2 🤖 — `release-prerelease.yml` + `release-stable.yml`: split the single `cargo release`
      step into bump (`--no-publish --no-tag --no-push`) → OIDC auth → publish → tag+push, all in
      the same job with no push in between (REL-05, REL-06, REL-08, REL-16). Also added a
      `custom_version` input to `release-prerelease.yml` for the 9.0.0 edge case (REL-17).
- [x] REL-T3 🤖 — `publish-crates.yml`: add the same OIDC auth step; keep as the documented manual
      retry entry point (REL-16).
- [x] REL-T4 🤖 — `docker-release.yml`: fix the missing `ref:` on checkout (REL-09, REL-10).
- [x] REL-T5 🤖 — `docker-release.yml`: add build provenance attestation + cosign keyless signing,
      split into `image` + `github-release` jobs (REL-12, REL-13).
- [x] REL-T6 🤖 — `docker-release.yml`'s new `github-release` job: create/update a GitHub Release
      per tag from `git-cliff` notes, idempotent, `prerelease` flag from the existing tag
      classification (REL-01..04).
- [x] REL-T7 🤖 — Document the naming convention (no `v` prefix, historical tags untouched) in
      STATE.md (REL-14, REL-15).
- [x] REL-T8 🤖 — Write `docs/book/src/08-release-process.md`: normal flow, publish-failure retry
      procedure (`--unpublished`), and the 9.0.0 RC→stable process (REL-16, REL-17).
- [x] REL-T9 🤖 — Least-privilege `permissions:` review across all 4 workflow files; confirm no job
      has broader scope than it uses.
- [x] REL-T10 🤖 — YAML-syntax-validate every edited workflow file (`python3 -c "import yaml; ..."`);
      full gate (`cargo fmt`, `cargo build --workspace`, `cargo test --workspace --all`).
- [ ] REL-T11 🧑 — Configure crates.io **Trusted Publishing** for all 15 publishable crates
      (crates.io → each crate → Settings → Publishing → add GitHub Actions config pointing at this
      repo + `release-prerelease.yml`/`release-stable.yml`/`publish-crates.yml`). Blocks REL-05/06
      actually working; `CARGO_REGISTRY_TOKEN` secret stays as fallback until this is done.
- [ ] REL-T12 🧑 — Create the GitHub **Environment `release`** (Settings → Environments) with
      required reviewers, and reference it (`environment: release`) in the 3 publish/bump jobs —
      the environment itself can't be created via workflow YAML.
- [ ] REL-T13 🧑 — After REL-T11 is confirmed working end-to-end on a real RC (see REL-T14),
      remove the `CARGO_REGISTRY_TOKEN` repository secret (REL-07).
- [ ] REL-T14 🧑 — Cut `9.0.0-rc.1` by dispatching `release-prerelease.yml` with
      `custom_version: 9.0.0-rc.1` (REL-17) — this is a real release action against the live
      repo/registry; I've documented the exact process in design.md §9 and
      `docs/book/src/08-release-process.md`, but will not dispatch it myself without your explicit
      go-ahead on the day you want to cut it.
