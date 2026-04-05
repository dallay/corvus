# Delta for Safety Model

## ADDED Requirements

### Requirement: REQ-SAFE-010 Shared Workspace Corpus Discovery

The system MUST provide a shared workspace corpus discovery rule set for persistent indexing that is
scoped to the active workspace only.

Corpus discovery MUST start from the canonical workspace root and MUST apply the same raw-path and
resolved-path safety checks already required for workspace search. Discovery MUST respect existing
ignore rules and resource limits for skipped files. Files or directories outside the canonical
workspace boundary MUST NOT be admitted to the corpus, even when reached through symlinks.

Accepted corpus entries MUST be converted to workspace-relative identities before they are handed to
the persistence layer. Absolute paths MAY be used transiently during validation, but they MUST NOT
be treated as persisted corpus identities.

#### Scenario: Discovery indexes only files inside the active workspace

- GIVEN a workspace root `/workspace`
- AND a sibling directory `/other-project` containing text files
- WHEN the workspace trigram corpus is discovered
- THEN only files whose resolved paths are inside `/workspace` MUST be considered for indexing
- AND files from `/other-project` MUST NOT be admitted to the corpus

#### Scenario: Symlink escape is excluded from corpus discovery

- GIVEN a workspace containing a symlink `src/external.rs` pointing to `/outside/secret.rs`
- WHEN the workspace trigram corpus is discovered
- THEN `src/external.rs` MUST be excluded from the corpus
- AND discovery MUST complete without widening workspace scope

### Requirement: REQ-SAFE-011 Deterministic Non-Text and Self-Index Exclusion

Persistent corpus discovery MUST exclude files that cannot be deterministically treated as safe text
inputs.

At minimum, discovery MUST exclude:

- binary files detected by the repository's existing null-byte sampling rule,
- files whose contents are not valid UTF-8,
- files determined to be non-text after decoding validation,
- unreadable files,
- index storage files created by the workspace trigram index itself, including database sidecars.

These exclusions MUST be deterministic: the same workspace contents and index location MUST produce
the same admitted corpus membership on repeated builds.

#### Scenario: Invalid UTF-8 file is excluded from the corpus

- GIVEN a workspace file `fixtures/bad.txt` whose bytes are not valid UTF-8
- WHEN the workspace trigram corpus is discovered
- THEN `fixtures/bad.txt` MUST be excluded from the corpus
- AND no replacement-character decoding MUST be used to admit it as text

#### Scenario: Binary file is excluded from the corpus

- GIVEN a workspace containing `src/text.rs` and `assets/logo.bin`
- AND `assets/logo.bin` contains a null byte within the sampled binary-detection window
- WHEN the workspace trigram corpus is discovered
- THEN `src/text.rs` MAY be admitted if it otherwise passes discovery rules
- AND `assets/logo.bin` MUST be excluded from the corpus

#### Scenario: Index database files are excluded from the corpus

- GIVEN the workspace trigram index stores its SQLite database and sidecar files under the workspace
- AND those files are otherwise reachable by the workspace walk
- WHEN the workspace trigram corpus is discovered
- THEN the index database file and its sidecars MUST be excluded from the corpus
- AND discovery MUST NOT index its own generated artifacts
