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
Template](.github/PULL_REQUEST_TEMPLATE.md). The template includes:

- **Summary**: Description of your changes
- **Type of Change**: Classification of the PR (bug fix, feature, etc.)
- **Changes Made**: Detailed list of modifications
- **Related Issues**: Link to related issue(s)
- **Checklist**: Including test requirements and code standards
- **Testing**: How you verified the changes
- **Screenshots/Logs**: Visual evidence if applicable

Make sure to fill out all relevant sections to help reviewers understand your changes.

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
