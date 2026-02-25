# Scribe Journal ✍️

Documentation agent journal for maintaining accuracy, synchronization, and clarity across Corvus.

## Guidelines for Journaling

- Track discrepancies found between code and docs.
- Maintain a bilingual glossary for technical terms.
- Record validation results from `make docs-web-build`.
- Log major structural changes or documentation debt.

---

## 2026-02-25 - Agent Scribe Initialization

**Status:** Initialized

**Context:**
- Documentation location: `clients/web/apps/docs/src/content/docs/`
- Root symlink: `docs/` -> `clients/web/apps/docs/src/content/docs/`
- Build command: `make docs-web-build`
- Check command: `make docs-web-check`

**Glossary (English / Spanish):**
- Agent Runtime / Runtime del Agente
- Reactive Orchestrator / Orquestador Reactivo
- Graph Memory / Memoria de Grafo
- Sidecar / Sidecar (mantener)
- Host-Mediated / Mediado por Host
- Edge-Native / Nativo en el Edge
- Task Management / Gestión de Tareas
- Peripheral / Periférico

**Current Documentation Debt:**
- Need to verify all CLI commands in `en/guides/cli-reference.md` against `clients/agent-runtime/` code.
- Ensure all KMP modules listed in `en/guides/features.md` exist and match the code.
- Sync any new architecture diagrams from `en/` to `es/`.

---
