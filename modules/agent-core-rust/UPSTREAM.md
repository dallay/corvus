# Upstream Source

This module was bootstrapped from:

- Repository: https://github.com/zeroclaw-labs/zeroclaw
- Imported snapshot directory: `/Users/acosta/Downloads/zeroclaw-main`

License and original project files are preserved inside this module.

## Gradle Bridge

Cargo bridge tasks are defined in
`/Users/acosta/Dev/corvus/modules/agent-core-rust/build.gradle.kts` and are disabled by default.

Enable them explicitly when needed:

```bash
./gradlew :agent-core-rust:check -PenableRustTasks=true
```
