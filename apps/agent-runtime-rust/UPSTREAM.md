# Upstream Source

This module was bootstrapped from:

- Repository: https://github.com/dallay/corvus
- Imported snapshot directory: local developer snapshot

License and original project files are preserved inside this module.

## Gradle Bridge

Cargo bridge tasks are defined in
`/Users/acosta/Dev/corvus/apps/agent-runtime-rust/build.gradle.kts` and are disabled by default.

Enable them explicitly when needed:

```bash
./gradlew :agent-runtime-rust:check -PenableRustTasks=true
```
