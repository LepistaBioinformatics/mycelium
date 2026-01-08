# Release Process

This guide explains how to create and manage releases for Mycelium using `cargo-release` and `git-cliff`.

## Overview

Mycelium follows [Semantic Versioning](https://semver.org/) and uses automated tools to manage releases:

- **cargo-release**: Manages version bumping, tagging, and publishing
- **git-cliff**: Generates changelogs from conventional commits

## Prerequisites

Install the required tools:

```bash
cargo install cargo-release
cargo install git-cliff
```

## Version Semantics

Mycelium follows Semantic Versioning (SemVer):

| Version Type | Format | When to Use | Example |
|--------------|--------|-------------|---------|
| **MAJOR** | `X.0.0` | Breaking changes or incompatible API changes | `8.0.0` → `9.0.0` |
| **MINOR** | `x.Y.0` | New features (backward-compatible) | `8.3.0` → `8.4.0` |
| **PATCH** | `x.y.Z` | Bug fixes (backward-compatible) | `8.3.0` → `8.3.1` |

## Pre-release Workflow

Pre-releases follow a specific progression through stages:

### 1. Alpha Stage

**Purpose**: Early development and testing

**Characteristics**:
- Unstable, frequent changes
- Used for initial feature testing
- Not recommended for production

**Creating an alpha release**:

```bash
# First alpha
cargo release alpha --execute  # Creates x.y.z-alpha.1

# Subsequent alphas
cargo release alpha --execute  # Creates x.y.z-alpha.2, etc.
```

**Example progression**: `8.3.0-alpha.1` → `8.3.0-alpha.2` → `8.3.0-alpha.3`

### 2. Beta Stage

**Purpose**: Feature-complete version ready for broader testing

**Characteristics**:
- Features are complete
- API should be relatively stable
- May still have bugs
- Used for wider testing and feedback

**Moving to beta**:

```bash
# First beta
cargo release beta --execute   # Creates x.y.z-beta.1

# Subsequent betas
cargo release beta --execute   # Creates x.y.z-beta.2, etc.
```

**Example progression**: `8.3.0-beta.1` → `8.3.0-beta.2` → `8.3.0-beta.3`

### 3. Release Candidate (RC) Stage

**Purpose**: Production-ready candidate for final validation

**Characteristics**:
- Final testing before stable release
- Only critical bug fixes allowed
- Ready for production testing
- Last chance to catch issues

**Creating release candidates**:

```bash
# First RC
cargo release rc --execute     # Creates x.y.z-rc.1

# Subsequent RCs (if needed)
cargo release rc --execute     # Creates x.y.z-rc.2, etc.
```

**Example progression**: `8.3.0-rc.1` → `8.3.0-rc.2`

### 4. Stable Release

**Purpose**: Production-ready version

**Creating the stable release**:

```bash
cargo release release --execute  # Creates x.y.z
```

**Example**: `8.3.0-rc.2` → `8.3.0`

## Version Increment Commands

### Patch Release

For bug fixes on existing stable releases:

```bash
cargo release patch --execute
```

**Example**: `8.3.0` → `8.3.1`

### Minor Release

For new features (backward-compatible):

```bash
cargo release minor --execute
```

**Example**: `8.3.1` → `8.4.0`

### Major Release

For breaking changes:

```bash
cargo release major --execute
```

**Example**: `8.4.0` → `9.0.0`

## Complete Release Cycle Example

Here's a complete example of releasing version 8.3.0:

```bash
# Alpha stage - initial testing
cargo release alpha --execute        # 8.3.0-alpha.1
# ... make changes, test ...
cargo release alpha --execute        # 8.3.0-alpha.2
# ... more changes, testing ...
cargo release alpha --execute        # 8.3.0-alpha.3

# Beta stage - features complete
cargo release beta --execute         # 8.3.0-beta.1
# ... wider testing, bug fixes ...
cargo release beta --execute         # 8.3.0-beta.2

# Release candidate - final validation
cargo release rc --execute           # 8.3.0-rc.1
# ... production testing ...
cargo release rc --execute           # 8.3.0-rc.2

# Stable release
cargo release release --execute      # 8.3.0

# Later patch releases
cargo release patch --execute        # 8.3.1
cargo release patch --execute        # 8.3.2

# Next minor release
cargo release minor --execute        # 8.4.0
```

## Changelog Management

Mycelium uses `git-cliff` to automatically generate changelogs from conventional commits.

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

### Generating Changelogs

Before creating a release, update the changelog:

```bash
# Preview unreleased changes
git-cliff --unreleased

# Update CHANGELOG.md with unreleased changes
git-cliff --unreleased --prepend CHANGELOG.md

# Generate changelog for a specific version
git-cliff --tag v8.3.0 --prepend CHANGELOG.md
```

### Changelog Configuration

The changelog format is configured in `cliff.toml` at the repository root. This file defines:
- Commit parsing rules
- Grouping and sorting
- Output format
- Template customization

## Dry Run (Recommended)

Always preview release changes before executing:

```bash
# Dry run (default - no --execute flag)
cargo release alpha

# Review the output carefully:
# - Version changes
# - Files that will be modified
# - Git commands that will run
# - Tags that will be created

# If everything looks correct, execute
cargo release alpha --execute
```

## Release Checklist

Use this checklist before creating a stable release:

- [ ] All tests pass: `cargo test`
- [ ] Code is properly formatted: `cargo fmt`
- [ ] No security vulnerabilities: `cargo audit`
- [ ] Documentation is up-to-date
- [ ] All commits follow conventional commit format
- [ ] Changelog is updated: `git-cliff --unreleased --prepend CHANGELOG.md`
- [ ] All CI checks pass
- [ ] Team review is complete (for major/minor releases)
- [ ] Release notes are prepared
- [ ] Dry run reviewed: `cargo release <level>`

## Release Configuration

The project's release behavior is configured in `release.toml` at the repository root.

Key configurations include:
- **Pre-release hooks**: Run tests and builds before releasing
- **Version bumping**: Control how versions are incremented
- **Git operations**: Tag format, commit messages
- **Changelog integration**: Automatic changelog generation with git-cliff
- **Publishing**: Control what gets published and where

## Best Practices

1. **Test thoroughly**: Run full test suite before any release
2. **Use dry runs**: Always preview changes before executing
3. **Follow the progression**: Don't skip stages (alpha → beta → rc → release)
4. **Write good commits**: Use conventional commits for automatic changelog generation
5. **Update changelog**: Generate changelog before each release
6. **Coordinate releases**: Communicate with team for major/minor releases
7. **Tag properly**: Let cargo-release handle tagging automatically
8. **Document changes**: Include migration guides for breaking changes

## Common Workflows

### Hotfix Release

For urgent bug fixes on a stable release:

```bash
# On main branch with stable release 8.3.0
git checkout -b hotfix/critical-bug
# ... fix the bug ...
git commit -m "fix: resolve critical security issue"

# Merge to main
git checkout main
git merge hotfix/critical-bug

# Create patch release
git-cliff --unreleased --prepend CHANGELOG.md
git add CHANGELOG.md
git commit -m "docs: update changelog for 8.3.1"
cargo release patch --execute  # 8.3.0 → 8.3.1
```

### Feature Release

For a new feature release:

```bash
# On develop branch
git checkout -b feature/new-capability
# ... implement feature ...
git commit -m "feat: add new capability"

# Merge to develop
git checkout develop
git merge feature/new-capability

# Start pre-release cycle
cargo release alpha --execute    # 8.4.0-alpha.1
# ... test, fix, repeat ...
cargo release beta --execute     # 8.4.0-beta.1
# ... wider testing ...
cargo release rc --execute       # 8.4.0-rc.1
# ... final validation ...

# Merge to main and release
git checkout main
git merge develop
git-cliff --unreleased --prepend CHANGELOG.md
git add CHANGELOG.md
git commit -m "docs: update changelog for 8.4.0"
cargo release release --execute  # 8.4.0
```

## Troubleshooting

### Release fails due to uncommitted changes

```bash
# Ensure working directory is clean
git status

# Commit or stash changes
git add .
git commit -m "chore: prepare for release"
```

### Changelog not generating correctly

```bash
# Verify conventional commit format
git log --oneline -n 10

# Test cliff configuration
git-cliff --unreleased

# Check cliff.toml configuration
cat cliff.toml
```

### Wrong version incremented

```bash
# Use dry run first to verify
cargo release <level>

# If wrong level used, manually fix:
# Edit Cargo.toml files
# Delete incorrect git tag: git tag -d vX.Y.Z
# Try again with correct level
```

## Additional Resources

- [Semantic Versioning Specification](https://semver.org/)
- [Conventional Commits Specification](https://www.conventionalcommits.org/)
- [cargo-release Documentation](https://github.com/crate-ci/cargo-release)
- [git-cliff Documentation](https://git-cliff.org/)
- [Contributing Guide](../../CONTRIBUTING.md#release-process)
