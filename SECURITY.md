# Security Policy

## Supported Versions
The following versions of this project are currently supported with security updates:

| Version | Supported          |
|---------|------------------|
| 1.x     | ✅ |
| 0.x     | ❌ |

## Reporting a Vulnerability
If you discover a security vulnerability in this project, please follow these steps:

1. **Do not** report vulnerabilities in public issues.
2. Send an email to [sgelias@outlook.com](mailto:sgelias@outlook.com) with details about the issue.
3. Provide a clear description, including steps to reproduce the vulnerability.
4. We will acknowledge receipt within 48 hours and respond with next steps.

We take security issues seriously and will work to resolve them as quickly as possible.

## Security Update Process
- Critical issues will be patched as soon as possible.
- Minor security issues will be addressed in the next scheduled release.
- Contributors reporting valid vulnerabilities may be publicly credited if desired.

## Automated Security Scanning

### Continuous Integration
This project uses automated security scanning to detect vulnerabilities in dependencies:

- **Tool**: [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- **Workflow**: Dedicated [Security workflow](.github/workflows/security.yml)
- **Frequency**:
  - Every push to `main` and `develop` branches
  - Every pull request to `main` and `develop`
  - Weekly scheduled scans (Sundays at midnight UTC)
  - Manual trigger available via GitHub Actions
- **CI Enforcement**: Builds fail if any vulnerabilities are detected (strict policy)
- **Vulnerability Database**: [RustSec Advisory Database](https://rustsec.org/)

### Local Security Checks

Before submitting pull requests, contributors are encouraged to run security audits locally:

```bash
# Install cargo-audit (first time only)
cargo install cargo-audit

# Run security scan
cargo audit
```

If vulnerabilities are found:
1. Review the advisory details provided by cargo-audit
2. Update the affected dependencies if patches are available
3. Check the project's `Cargo.toml` workspace dependencies for version constraints
4. Run `cargo update <crate-name>` to update specific dependencies
5. Re-run `cargo audit` to verify the fix

### Dependency Updates

Security-related dependency updates are prioritized:
- Critical vulnerabilities are addressed immediately
- Workspace-level dependencies are centrally managed in the root `Cargo.toml`
- Regular dependency audits ensure the project remains secure

For more information about contributing and dependency management, see [CONTRIBUTING.md](./CONTRIBUTING.md).
