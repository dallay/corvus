# No Fully Qualified References

Summary: Avoid using fully-qualified references inline in Kotlin source files (for example
`@io.swagger.v3.oas.annotations.parameters.RequestBody` or `java.util.UUID.randomUUID()`). Add
top-level imports and use the short names instead (e.g.
`import io.swagger.v3.oas.annotations.parameters.RequestBody` then `@RequestBody`, or
`import java.util.UUID` then `UUID.randomUUID()`).

## Why

- Improves readability and consistency.
- Enables IDE autocomplete and easier refactoring.
- Keeps code concise and easier to scan.

## Scope

- Applies to `.kt` files. Covers:
  - Inline annotations using fully-qualified names.
  - Fully-qualified class references in code (e.g. `com.example.Foo.Bar`).
  - Fully-qualified calls to static/Java class methods or properties (e.g.
    `java.util.UUID.randomUUID()`).

## Exceptions

- KDoc: Fully-qualified class names are allowed in KDoc when referencing external types in
  documentation blocks (e.g., `See com.example.lib.SomeType`) — this helps link resolvers and
  maintain clear docs.
- Intentional disambiguation: If two different packages export the same simple name and the
  developer intentionally wants to use the FQCN at the use-site to disambiguate, this is acceptable
  but should be documented with a comment.

## Bad examples

```kotlin
// Fully-qualified annotation inline
fun update(@io.swagger.v3.oas.annotations.parameters.RequestBody request: UpdateRequest) { ... }

// Fully-qualified static call
val id = java.util.UUID.randomUUID()

// Fully-qualified nested type usage
val x: com.some.lib.Outer.Inner = //...
```

## Good examples

```kotlin
import io.swagger.v3.oas.annotations.parameters.RequestBody
import java.util.UUID
import com.some.lib.Outer.Inner

fun update(@RequestBody request: UpdateRequest) { ... }

val id = UUID.randomUUID()

val x: Inner = //...
```

## Auto-fix guidance for agents (safe, conservative)

1. Detect candidate fully-qualified usages with regex patterns such as:
  - annotations: `@([A-Za-z_][\w]*(?:\.[A-Za-z_][\w]*)+)`
  - class/method references: `\b([A-Za-z_][\w]*(?:\.[A-Za-z_][\w]*)+)\.[A-Za-z_][\w]*\b`
2. For each match, compute the simple name (substring after the last `.`).
3. Collision check: if the simple name is already imported but from a different package, skip
   auto-fix and add a comment `// TODO: resolve name collision for SimpleName` near the occurrence.
4. Insert `import FULL.PACKAGE.SimpleName` after the `package` declaration (or at top if no package)
   if that import is not present.
5. Replace usages of the fully-qualified name with the simple name (for annotations, replace the
   annotation token; for calls, replace the qualifier portion so `java.util.UUID.randomUUID()`
   becomes `UUID.randomUUID()`).
6. Do not change occurrences inside string literals, comments, or KDoc blocks.
7. Avoid changing code where the fully-qualified qualifier is used as part of a nested package chain
   intentionally (e.g., when referring to Java package constants). When unsure, leave a TODO for
   human review.
8. Group repo-wide refactors into a single PR with tests and a changelog note.

## Edge cases

- Name collisions across packages: skip and report for manual resolution.
- References inside annotation arguments that require compile-time constants: be conservative — if
   replacement would alter constant expressions, skip and add TODO.
- Generated code or files explicitly marked as generated: skip.

## Agent guidance (short)

- Prefer safe, non-destructive changes. When in doubt, add a `// TODO` and do not auto-fix.
- Keep imports grouped and sorted, follow project style (no wildcard imports).

## Commit message suggestion

- `chore(kotlin): prohibit fully-qualified inline references; prefer imports (agent guidance)`
