# Security Policy

## Supported Versions

Security updates and patches are provided for the latest minor release line:

| Version | Supported |
| :--- | :--- |
| `0.2.x` | ✅ Yes |
| `< 0.2.0` | ❌ No |

---

## Supply Chain Security & Build Provenance

All official `htmx-analyzer` release binaries published on GitHub Releases are attested using GitHub Actions build provenance (`actions/attest-build-provenance`).

Before executing downloaded binaries, verify their integrity and build provenance using the GitHub CLI:

```bash
gh attestation verify <asset> \
  -R adrijshikhar/htmx-analyzer \
  --signer-workflow adrijshikhar/htmx-analyzer/.github/workflows/release.yml \
  --source-ref refs/tags/<tag>
```

In addition, our CI enforces `#![forbid(unsafe_code)]` across all workspace crates and audits dependencies via `cargo-deny`.

---

## Reporting a Vulnerability

We take the security of `htmx-analyzer` seriously. If you discover a security issue or vulnerability, please follow coordinated disclosure practices and do **not** report it via public GitHub issues.

### Reporting Process

1. **GitHub Private Vulnerability Reporting (Preferred):**
   Navigate to the [Security Advisories](https://github.com/adrijshikhar/htmx-analyzer/security/advisories) tab and click **"Report a vulnerability"**.

2. **Email Disclosure:**
   If GitHub Security Advisories is unavailable, please email the maintainer directly at:
   **adrijshikhar@gmail.com** with the subject `[SECURITY] htmx-analyzer vulnerability report`.

### What to Include

Please provide:
- A description of the issue and potential security impact.
- Steps to reproduce or a minimal proof of concept (PoC).
- Any proposed remediation or patches.

### Timeline & Acknowledgments

- **Initial Response:** You will receive an acknowledgment within 48 hours.
- **Resolution:** A fix will be developed, tested, and released promptly.
- **Credit:** Reporters will be acknowledged in the release notes and advisory (unless requested otherwise).
