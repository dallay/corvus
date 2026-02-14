# Security Policy

## Supported Versions

We provide security updates only for actively maintained versions.
Please ensure you are using one of the versions listed below:

| Version        | Supported |
| -------------- | --------- |
| main / latest  | ✔️        |
| Older releases | ❌        |

If you are using an unsupported version, please upgrade to the latest release before reporting a vulnerability.

---

## Reporting a Vulnerability

If you discover a security vulnerability, please follow the guidelines below:

1. **Do not open a public issue.**
   Security issues must not be disclosed publicly until a fix is released.

2. **To report a security issue:**
   Please use the GitHub Security Advisory **“Report a Vulnerability”** tab
   (located under the repository’s **Security** section).

3. Your report should ideally include:
   - A clear description of the vulnerability
   - Steps to reproduce
   - Possible impact
   - Suggested fix (optional)

We will respond within **72 hours** and work with you to validate and resolve the issue.

---

## Disclosure Policy

- We follow a **coordinated disclosure** model.
- Once the vulnerability is confirmed and fixed, we will:
  - Publish a security advisory
  - Acknowledge reporters (if they wish)
  - Release patched versions

We kindly ask security researchers to:

- Avoid exploiting or abusing vulnerabilities
- Avoid running destructive tests on production services
- Give us reasonable time to fix issues before public disclosure

---

## Security Best Practices

If you are building on top of this project, we recommend:

- Always use the latest stable version
- Review release notes and security advisories
- Never expose internal endpoints to the public
- Rotate secrets regularly
- Follow the principle of least privilege (PoLP)

---

Thank you for helping keep this project and its users safe.
