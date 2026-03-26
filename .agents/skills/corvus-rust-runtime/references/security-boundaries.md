# Security Boundaries

## High-risk areas

Treat these modules as high-risk by default:

- `src/security/`
- `src/gateway/`
- `src/auth/`
- `src/tools/`

## Non-negotiables

- Fail closed on missing or uncertain policy/auth state
- Validate all external input early
- Do not log secrets, tokens, pairing codes, or raw sensitive payloads
- Do not silently widen filesystem, network, or process access
- Add regression tests for security-sensitive behavior changes

## Boundary checklist

- Is the input trusted? If not, validate and sanitize.
- Is the path normalized and still inside the allowed root?
- Is the tool/process invocation constrained by policy?
- Does any error/log accidentally expose sensitive data?
- If verification fails, does the code reject instead of continue?
