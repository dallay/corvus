# Skill: tdd

Description
-----------
Skill for Test-Driven Development (TDD): write failing tests first, implement the minimal code to
pass, and then refactor safely while preserving behavior.

Purpose
-------
Enforce a Red-Green-Refactor workflow to improve design quality, regression safety, and long-term
maintainability across Kotlin, Rust, and Gradle modules in this repository.

Triggers
--------
- User asks to implement a feature "with TDD" or "test first"
- User requests adding behavior with strong regression guarantees
- Changes in `**/*Test.kt`, `**/*Spec.kt`, `**/tests/**/*.rs`, or new test files
- Refactors in critical paths where behavior must remain unchanged
- Bugs that need reproducible failing tests before fixes

Capabilities
------------
- Convert requirements into executable test cases (happy path, edge cases, failure modes)
- Write minimal failing test first, then minimal implementation to pass
- Refactor after green tests to improve readability and structure
- Add regression tests for bug fixes before touching production logic
- Keep tests deterministic, isolated, and fast

Limitations
-----------
- Do not skip the failing-test step unless explicitly requested by the user
- Do not perform destructive refactors without passing tests
- Do not introduce flaky tests dependent on network, time, or shared global state

Preconditions
------------
- Relevant test tooling must be available (`./gradlew test`, module-specific test tasks, or
  `cargo test`)
- The target module should compile before starting significant refactoring

Expected Output
---------------
- A small sequence of commits or patches following Red -> Green -> Refactor
- Commands used for verification (for example: `./gradlew :composeApp:jvmTest`,
  `./gradlew test`, `cargo test`)
- Brief rationale explaining design choices discovered through tests

Standard Workflow
-----------------
1. Red
   - Write one focused failing test that describes the next behavior.
   - Confirm it fails for the expected reason.
2. Green
   - Implement the smallest possible code change.
   - Run the smallest relevant test scope first, then broader suite if needed.
3. Refactor
   - Improve naming, duplication, and structure without changing behavior.
   - Re-run tests and keep them green.

Quality Guardrails
------------------
- One behavior per test
- Clear test names describing intent
- Avoid over-mocking; prefer real domain behavior where practical
- Cover negative paths and boundary conditions
- Keep production code simple; let tests drive abstractions

Related Skills
--------------
- Use [kotlin](.agents/skills/kotlin/SKILL.md) for Kotlin-specific implementation details
- Use [rust](.agents/skills/rust/SKILL.md) for Rust module work and Cargo-level changes
- Use [kotlin-coroutines](.agents/skills/kotlin-coroutines/SKILL.md) for async flow testing patterns

When to Invoke This Skill
-------------------------
- Default for new behavior in core logic, bug fixes, and risky refactors
- Especially useful in security-sensitive and business-critical modules
