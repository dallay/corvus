---
name: conventional-commits
description: >
  Conventional Commits specification and usage guidance.
  Trigger: Creating commits, git messages, or commit guidelines.
license: Apache-2.0
metadata:
  author: yuniel-acosta
  version: "1.0"
---

## When to Use

- When writing or reviewing git commit messages
- When tooling needs Conventional Commits compliance
- When explaining commit types, scopes, or breaking changes

## Critical Patterns

- Commit format: `<type>[optional scope][optional !]: <description>`
- `feat` for new features, `fix` for bug fixes; other types allowed (`docs`, `chore`, `ci`, `refactor`, etc.)
- Breaking changes MUST be indicated with `!` before `:` or with a `BREAKING CHANGE:` footer
- Description is required and follows `:` + space
- Body is optional and starts after a blank line; can be multi-paragraph
- Footers are optional, follow trailer format (`Token: value` or `Token #value`); tokens use `-` not spaces

## Code Examples

```text
feat: allow provided config object to extend other configs

BREAKING CHANGE: `extends` key in config file is now used for extending other config files
```

```text
feat(api)!: send an email when a product is shipped
```

```text
docs: correct spelling of CHANGELOG
```

```text
fix: prevent racing of requests

Introduce a request id and a reference to latest request. Dismiss
incoming responses other than from latest request.

Reviewed-by: Z
Refs: #123
```

## Commands

```bash
git commit -m "feat(auth): add token refresh"
git commit -m "fix(api)!: drop legacy endpoint" -m "BREAKING CHANGE: remove v1 endpoint"
```

## Resources

- External spec: https://www.conventionalcommits.org/en/v1.0.0/
