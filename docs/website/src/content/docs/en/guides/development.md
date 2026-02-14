---
title: Development Procedures
---

This guide outlines the standard procedures for developing and maintaining this project.

## Using the Makefile

The `Makefile` provides a standardized way to interact with the project across different operating systems.

### Essential Commands

| Command | Description |
|---------|-------------|
| `make setup` | Initial setup (permissions, tool checks) |
| `make run` | Runs the main application |
| `make build` | Full project build including tests |
| `make test` | Runs all unit tests |
| `make check` | Runs all quality checks (format + lint + tests) |
| `make format` | Automatically fixes code formatting |
| `make clean` | Removes build artifacts |

### Documentation Commands

- `make docs`: Generates Dokka API documentation.
- `make docs-web-build`: Builds the Starlight website.
- `make docs-web-dev`: Starts the development server for the website.

## Workflow

1. **Setup**: Run `make setup` once after cloning.
2. **Development**: Write code in `app/` or `examples/`.
3. **Formatting**: Run `make format` frequently to keep code clean.
4. **Verification**: Run `make check` before committing to ensure everything passes.
5. **Testing**: Add tests in `src/test` and run them with `make test`.

## Adding Dependencies

To add a new dependency:
1. Define it in `gradle/libs.versions.toml`.
2. Reference it in the desired module's `build.gradle.kts` using `libs.<name>`.
3. Run `make build` to verify.
