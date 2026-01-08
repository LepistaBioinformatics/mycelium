# Contributing to Mycelium

Thank you for your interest in contributing to Mycelium! This document provides
guidelines and instructions for contributing to the project.

Mycelium is an open-source API Gateway designed for modern, multi-tenant, API-oriented environments. We welcome contributions of all kinds: code, documentation, bug reports, feature requests, and more.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Environment Setup](#development-environment-setup)
4. [Building and Testing](#building-and-testing)
5. [Code Standards](#code-standards)
6. [Git Workflow](#git-workflow)
7. [Pull Request Process](#pull-request-process)
8. [Reporting Bugs](#reporting-bugs)
9. [Suggesting Features](#suggesting-features)
10. [Code Review Guidelines](#code-review-guidelines)
11. [Resources](#resources)

## Code of Conduct

This project adheres to a [Code of Conduct](./CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please read it before contributing.

## Getting Started

Contributions can take many forms:

- **Code contributions**: Bug fixes, new features, performance improvements
- **Documentation**: Improving docs, adding examples, fixing typos
- **Bug reports**: Identifying and reporting issues
- **Feature requests**: Proposing new functionality
- **Task requests**: Requesting specific tasks or improvements
- **Code reviews**: Reviewing pull requests from other contributors

### Creating Issues

When opening a new issue, **always use the appropriate issue template**:

1. Go to [New Issue](https://github.com/sgelias/mycelium/issues/new/choose)
2. Select the appropriate template:
   - **Bug report** - For reporting bugs
   - **Feature request** - For suggesting new features
   - **Task request** - For requesting specific tasks or improvements
3. Fill in all required sections in the template

Using templates ensures your issue includes all necessary information and helps maintainers respond more quickly.

Before starting work on a significant change, please open an issue to discuss your proposed changes with the maintainers.

## Development Environment Setup

### Prerequisites

- **Rust toolchain**: Install via [rustup](https://rustup.rs/)
- **Docker & Docker Compose**: For running development services
- **Git**: For version control

### Option 1: DevContainer (Recommended)

The easiest way to get started is using the provided DevContainer configuration:

1. Install [Visual Studio Code](https://code.visualstudio.com/)
2. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
3. Clone the repository:
   ```bash
   git clone https://github.com/sgelias/mycelium.git
   cd mycelium
   ```
4. Open the project in VS Code
5. When prompted, click "Reopen in Container" (or use Command Palette: "Dev Containers: Reopen in Container")

The DevContainer automatically sets up:
- Rust toolchain and development tools
- PostgreSQL database
- Redis cache
- Jaeger (distributed tracing)
- Prometheus (metrics)
- Grafana (visualization)
- OpenTelemetry Collector
- Pre-configured VS Code extensions:
  - rust-analyzer
  - code-spell-checker

### Option 2: Local Setup

If you prefer to develop locally without DevContainer:

1. Install Rust toolchain:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/sgelias/mycelium.git
   cd mycelium
   ```

3. Start development services:
   ```bash
   docker-compose up -d
   ```

4. Set environment variables as needed (refer to `.devcontainer/.env` for examples)

## Building and Testing

### Build the Project

To build the entire workspace:

```bash
cargo build
```

To build a specific workspace member:

```bash
cargo build -p myc-core
```

### Testing Requirements

- All major new features must include automated tests.
- Bug fixes should include tests that prevent regression.
- Tests should follow Rust's testing conventions as described in [The Rust
  Book](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).
- Unit tests should be placed in the same file as the code being tested, within
  a `#[cfg(test)]` module.
- Integration tests should be placed in the `tests/` directory.
- Run tests locally with `cargo test` before submitting pull requests.
- Aim for maintaining or improving current code coverage.

#### How to Run Tests

Run all tests:

```bash
cargo test
```

Run tests for a specific package:

```bash
cargo test -p mycelium-base
```

Run tests with logging enabled:

```bash
RUST_LOG=debug cargo test
```

### Code Formatting

Format your code before committing:

```bash
cargo fmt
```

The project uses custom rustfmt configuration (`.rustfmt.toml`):
- Maximum line width: 80 characters
- Binary operator separator: back
- Separate definition blocks: always

### Security Audit

Check for security vulnerabilities using `cargo-audit`.

First, install cargo-audit (if not already installed):

```bash
cargo install cargo-audit
```

Then run the security audit:

```bash
cargo audit
```

**Note**: Automated security scanning is being configured (see issue #121).

## Code Standards

### Rust Conventions

- Follow standard Rust naming conventions and idioms
- Use the Rust 2021 edition features appropriately
- Prefer explicit types when it improves readability
- Use meaningful variable and function names

### Code Quality

- **Format**: All code must be formatted with `cargo fmt`
- **Tests**: New features and bug fixes must include tests
- **Documentation**: Public APIs must be documented with doc comments

### Documentation

- Use doc comments (`///`) for public APIs
- Include examples in doc comments when helpful
- Keep documentation up-to-date with code changes
- Use clear, concise language

### Testing

- Write unit tests for individual components
- Write integration tests for complex interactions
- Aim for good test coverage, especially for critical paths
- Tests should be deterministic and not rely on external state

## Git Workflow

### Forking and Branching

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/mycelium.git
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/sgelias/mycelium.git
   ```
4. Create a feature branch from `develop` using the issue type and number:
   ```bash
   git checkout develop
   git pull upstream develop
   git checkout -b task/117
   ```

### Branch Naming

Branch names should follow the format `type/issue-number`, where type matches the issue type:
- `task/117` - for a task issue #117
- `feat/110` - for a feature issue #110
- `fix/123` - for a bug fix issue #123
- `docs/456` - for a documentation issue #456
- `refactor/789` - for a refactoring issue #789

This convention ensures clear traceability between branches and their corresponding GitHub issues.

### Commit Messages

Write clear, descriptive commit messages:

- Use the imperative mood ("Add feature" not "Added feature")
- First line: brief summary (50 chars or less)
- Blank line, then detailed explanation if needed
- Reference issue numbers: "Fixes #123" or "Relates to #456"

Example:
```
Add passwordless authentication flow

Implements the passwordless authentication flow using magic links.
Users can now sign in by clicking a link sent to their email.

Fixes #110
```

### Keeping Your Fork Updated

Regularly sync your fork with upstream:

```bash
git checkout develop
git pull upstream develop
git push origin develop
```

## Pull Request Process

### Before Submitting

1. Ensure your code builds: `cargo build`
2. Run tests: `cargo test`
3. Format code: `cargo fmt`
4. Update documentation if needed
5. Rebase on latest `develop` if needed

### Submitting a Pull Request

1. Push your branch to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

2. Open a pull request on GitHub targeting the `develop` branch (not `main`)
   - The PR will automatically load our [Pull Request Template](.github/PULL_REQUEST_TEMPLATE.md)
   - Fill in all sections of the template

3. Use a descriptive PR title following the format:
   - `[FEAT] Add new feature description`
   - `[FIX] Fix bug description`
   - `[TASK] Task description`
   - `[DOCS] Documentation update`
   - `[REFACTOR] Refactoring description`

4. In the PR description, include:
   - **Related Issue**: Link to the issue(s) this PR addresses
   - **Summary**: Brief description of changes
   - **Testing**: How you tested the changes
   - **Breaking Changes**: Any breaking changes and migration notes

### Pull Request Template

When you create a new PR, GitHub will automatically load our [Pull Request
Template](.github/PULL_REQUEST_TEMPLATE.md). The template is located at
`.github/PULL_REQUEST_TEMPLATE.md` and becomes available to contributors once
it's merged into the repository's default branch.

The template includes the following sections to ensure comprehensive PR documentation:

- **Summary**: Brief description of your changes and their purpose
- **Type of Change**: Classification of the PR (bug fix, feature, breaking change, etc.)
- **Changes Made**: Detailed bullet-point list of modifications
- **Related Issues**: Reference to related issue(s) using `#issue_number` format
- **Checklist**: Quality gates including:
  - Tests added for new functionality
  - Existing tests pass
  - Code follows style guidelines
  - Documentation updated
  - Conventional commit format followed
- **Testing**: Detailed testing approach including:
  - Test environment (OS, Rust version)
  - Step-by-step test instructions
- **Screenshots/Logs**: Visual evidence when applicable
- **Additional Notes**: Any extra context for reviewers

**Important**: Make sure to fill out all relevant sections thoroughly. Complete
information helps reviewers understand your changes and speeds up the review
process.

### Review Process

- A maintainer will review your PR
- Address any feedback or requested changes
- Keep the discussion focused and professional
- Once approved, a maintainer will merge your PR

**Note**: Automated CI checks are being configured (see issue #118).

## Reporting Bugs

### Before Reporting

- Check existing [issues](https://github.com/sgelias/mycelium/issues) to avoid duplicates
- Verify the bug exists in the latest version
- Gather relevant information about your environment

### Creating a Bug Report

When creating a new bug report, **use the Bug Report template** available in the issue creation form:

1. Go to [New Issue](https://github.com/sgelias/mycelium/issues/new/choose)
2. Select **"Bug report"** template
3. Fill in all required sections:
   - **Describe the bug**: Clear description of what the bug is
   - **To Reproduce**: Steps to reproduce the behavior
   - **Expected behavior**: What you expected to happen
   - **Screenshots**: If applicable
   - **Additional context**: Any other relevant information

The template ensures you provide all necessary information for efficient bug resolution.

### Security Vulnerabilities

**Do not report security vulnerabilities through public GitHub issues.**

Please refer to our [Security Policy](./SECURITY.md) for reporting security issues.

## Suggesting Features

We welcome feature suggestions! Here's how to propose new functionality:

### Creating a Feature Request

When suggesting a new feature, **use the Feature Request template** available in the issue creation form:

1. Go to [New Issue](https://github.com/sgelias/mycelium/issues/new/choose)
2. Select **"Feature request"** template
3. Fill in all required sections:
   - **Is your feature request related to a problem?**: Describe the problem
   - **Describe the solution you'd like**: Your proposed solution
   - **Describe alternatives you've considered**: Alternative approaches
   - **Additional context**: Any other relevant information

The template ensures your feature request includes all necessary context for evaluation.

### Before Submitting

- **Check existing issues**: See if someone already suggested it
- **Review related EPICs**: Check if it aligns with ongoing initiatives (see below)

### Related EPICs

When suggesting features, check if they relate to existing EPICs:
- [#90 - Downstream Services Discoverability](https://github.com/sgelias/mycelium/issues/90)
- [#79 - Max Coverage of Tests](https://github.com/sgelias/mycelium/issues/79)
- [#62 - Open-source MAG in Compliance with OSI Guidelines](https://github.com/sgelias/mycelium/issues/62)
- [#61 - MAG in Compliance with OpenSSF Specifications](https://github.com/sgelias/mycelium/issues/61)

### Discussion First

For significant features, please discuss the approach with maintainers before implementing. This helps ensure:
- The feature aligns with project goals
- The approach is sound
- Effort isn't wasted on features that won't be merged

## Code Review Guidelines

### For Contributors

- Be responsive to feedback
- Ask questions if feedback is unclear
- Don't take criticism personally - focus on technical merit
- Update your PR based on review comments
- Mark resolved conversations as resolved

### For Reviewers

- Be respectful and constructive
- Focus on the code, not the person
- Explain the reasoning behind suggestions
- Acknowledge good work
- Follow the principles in our [Code of Conduct](./CODE_OF_CONDUCT.md)

### Review Checklist

- Does the code follow project standards?
- Are tests included and passing?
- Is documentation updated?
- Are there any security concerns?
- Is the code maintainable and clear?
- Does it solve the stated problem?

## Release Process

Mycelium uses [`cargo-release`](https://github.com/crate-ci/cargo-release) to manage version releases and publishing. This section describes the release workflow and version management strategy.

### Installing Required Tools

First, install the required tools if you haven't already:

```bash
# Install cargo-release for version management
cargo install cargo-release

# Install git-cliff for changelog generation
cargo install git-cliff
```

### Version Semantics

Mycelium follows [Semantic Versioning](https://semver.org/) (SemVer):

- **MAJOR** (`X.0.0`): Incompatible API changes or breaking changes
- **MINOR** (`x.Y.0`): New functionality in a backward-compatible manner
- **PATCH** (`x.y.Z`): Backward-compatible bug fixes

### Pre-release Tags

Pre-release versions follow a specific progression for testing and validation:

1. **alpha** (`x.y.z-alpha.N`): Early development, unstable, frequent changes
   - Used for initial testing of new features
   - Not recommended for production use
   - Example: `8.3.0-alpha.1`, `8.3.0-alpha.2`

2. **beta** (`x.y.z-beta.N`): Feature complete, but may have bugs
   - Used for wider testing and feedback
   - API should be relatively stable
   - Example: `8.3.0-beta.1`, `8.3.0-beta.2`

3. **rc** (Release Candidate) (`x.y.z-rc.N`): Production-ready candidate
   - Final testing before release
   - Only critical bug fixes allowed
   - Example: `8.3.0-rc.1`, `8.3.0-rc.2`

4. **Stable Release** (`x.y.z`): Production-ready version
   - Example: `8.3.0`

### Release Workflow

#### 1. Starting a New Development Cycle

Create the first alpha release for testing:

```bash
# Create alpha.1 release
cargo release alpha --execute

# Subsequent alphas
cargo release alpha --execute  # Creates alpha.2, alpha.3, etc.
```

#### 2. Moving to Beta

When features are complete and ready for broader testing:

```bash
# Create beta.1 release
cargo release beta --execute

# Subsequent betas
cargo release beta --execute  # Creates beta.2, beta.3, etc.
```

#### 3. Creating Release Candidates

When the code is stable and ready for final validation:

```bash
# Create rc.1 release
cargo release rc --execute

# Subsequent release candidates (if needed)
cargo release rc --execute  # Creates rc.2, rc.3, etc.
```

#### 4. Final Stable Release

When all testing is complete and the release candidate is approved:

```bash
# Release the stable version
cargo release release --execute
```

This removes the pre-release suffix and creates a stable version (e.g., `8.3.0-rc.2` → `8.3.0`).

#### 5. Patch Releases

For bug fixes on existing stable releases:

```bash
# Increment patch version (8.3.0 → 8.3.1)
cargo release patch --execute
```

#### 6. Minor and Major Releases

For new features or breaking changes:

```bash
# Increment minor version (8.3.1 → 8.4.0)
cargo release minor --execute

# Increment major version (8.4.0 → 9.0.0)
cargo release major --execute
```

### Complete Release Cycle Example

Here's a complete example of releasing version 8.3.0:

```bash
# Start with alpha releases
cargo release alpha --execute        # 8.3.0-alpha.1
cargo release alpha --execute        # 8.3.0-alpha.2

# Move to beta after features are complete
cargo release beta --execute         # 8.3.0-beta.1
cargo release beta --execute         # 8.3.0-beta.2

# Create release candidates
cargo release rc --execute           # 8.3.0-rc.1
cargo release rc --execute           # 8.3.0-rc.2

# Final stable release
cargo release release --execute      # 8.3.0

# Later, for bug fixes
cargo release patch --execute        # 8.3.1

# For the next feature release
cargo release minor --execute        # 8.4.0
```

### Dry Run (Recommended)

Before executing any release, perform a dry run to preview changes:

```bash
# Dry run (default behavior)
cargo release alpha

# Review the output, then execute if everything looks correct
cargo release alpha --execute
```

### Changelog Generation

Mycelium uses [`git-cliff`](https://git-cliff.org/) to automatically generate changelogs from conventional commits. The configuration is in `cliff.toml` at the repository root.

#### Generating Changelogs

To update the CHANGELOG.md before a release:

```bash
# Generate changelog for unreleased changes
git-cliff --unreleased --prepend CHANGELOG.md

# Generate changelog for a specific version
git-cliff --tag v8.3.0 --prepend CHANGELOG.md

# Preview changelog without writing to file
git-cliff --unreleased
```

#### Conventional Commit Format

Git-cliff parses commits based on [Conventional Commits](https://www.conventionalcommits.org/). Use these prefixes:

- `feat:` - New features (🚀 Features)
- `fix:` - Bug fixes (🐛 Bug Fixes)
- `docs:` - Documentation changes (📚 Documentation)
- `perf:` - Performance improvements (⚡ Performance)
- `refactor:` - Code refactoring (🚜 Refactor)
- `style:` - Code style changes (🎨 Styling)
- `test:` - Test additions or changes (🧪 Testing)
- `chore:` - Maintenance tasks (⚙️ Miscellaneous Tasks)

Example commit message:
```
feat(auth): add passwordless authentication

Implements magic link authentication flow for users.

Fixes #110
```

### Release Configuration

The project's release configuration is defined in `release.toml` at the repository root. This file configures:
- Pre-release hooks (tests, build verification)
- Version bumping behavior
- Git tag format
- Changelog generation via git-cliff
- Publishing settings

### Best Practices

1. **Always test before releasing**: Run `cargo test` and verify builds
2. **Use dry runs**: Preview changes with `cargo release <level>` before using `--execute`
3. **Follow the progression**: alpha → beta → rc → release
4. **Use conventional commits**: Follow the commit format for automatic changelog generation
5. **Update CHANGELOG.md**: Run `git-cliff` to generate changelog before each release
6. **Tag releases**: `cargo-release` automatically creates Git tags
7. **Coordinate with team**: For major releases, ensure all stakeholders are informed

### Release Checklist

Before creating a stable release:

- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No security vulnerabilities (`cargo audit`)
- [ ] Documentation is updated
- [ ] All commits follow conventional commit format
- [ ] CHANGELOG.md is updated with `git-cliff --unreleased --prepend CHANGELOG.md`
- [ ] Release notes are prepared
- [ ] All CI checks pass
- [ ] Team review is complete (for major/minor releases)

## Resources

### Documentation

- [README](./README.md) - Project overview and features
- [Authorization Model](./docs/book/src/01-authorization.md) - Core authorization concepts
- [Code of Conduct](./CODE_OF_CONDUCT.md) - Community guidelines
- [Security Policy](./SECURITY.md) - Security reporting process
- [License](./LICENSE) - Apache 2.0 License

### Development

- [Project Repository](https://github.com/sgelias/mycelium)
- [Issue Tracker](https://github.com/sgelias/mycelium/issues)
- [Pull Requests](https://github.com/sgelias/mycelium/pulls)

### Community

- For questions, open a [GitHub Discussion](https://github.com/sgelias/mycelium/discussions) or issue
- Follow our [Code of Conduct](./CODE_OF_CONDUCT.md) in all interactions

---

Thank you for contributing to Mycelium! Your efforts help make this project better for everyone.
