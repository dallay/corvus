---
title: "Mapa de Workflows de CI"
description: Mapa de referencia de los workflows de GitHub, sus responsabilidades, condiciones de disparo y si deben bloquear merges.
owner: team-runtime
status: canonical
lastReviewed: 2026-04-12
appliesTo: main
docType: reference
---

# Mapa de Workflows de CI

Este documento explica qué hace cada workflow de GitHub, cuándo se ejecuta, y si debe bloquear
merges.

## Merge-Blocking vs Opcional

Los checks merge-blocking deben mantenerse pequeños y deterministas. Los checks opcionales son
útiles para automatización y mantenimiento, pero no deben bloquear desarrollo normal.

### Merge-Blocking

- `.github/workflows/ci.yml` (`CI`)
  - Propósito: Validación de Rust (`fmt`, `clippy`, `test`, smoke check de release build)
  - Merge gate: `CI Required Gate`
- `.github/workflows/workflow-sanity.yml` (`Workflow Sanity`)
  - Propósito: Lint de archivos de workflow (`actionlint`, checks de tabs)
  - Recomendado para PRs que cambian workflows

### No-Blocking pero Importantes

- `.github/workflows/docker.yml` (`Docker`)
  - Propósito: PR docker smoke check y publicación de imágenes en pushes a `main`/tag
- `.github/workflows/security.yml` (`Security Audit`)
  - Propósito: Avisos de dependencias (`cargo audit`) y checks de política/licencia (`cargo deny`)
- `.github/workflows/publish-release.yml` (`Publish Release`)
  - Propósito: publicar artefactos estables después de que se publique el GitHub Release canónico
- `.github/workflows/release-please-beta.yml` (`Release Please Beta`)
  - Propósito: crear PRs beta, tags, GitHub Releases y publicación beta desde la rama `beta`

### Automatización Opcional del Repositorio

- `.github/workflows/labeler.yml` (`PR Labeler`)
  - Propósito: Labels de path + size
- `.github/workflows/auto-response.yml` (`Auto Response`)
  - Propósito: Mensajes de onboarding para contribuidores primerizos
- `.github/workflows/stale.yml` (`Stale`)
  - Propósito: Automatización de ciclo de vida de issues/PRs stale
- `.github/workflows/pr-hygiene.yml` (`PR Hygiene`)
  - Propósito: Nudge a PRs stale-but-active para rebase/re-run de required checks antes de
    starvation de cola

## Mapa de Triggers

- `CI`: push a `main`/`develop`, PRs a `main`
- `Docker`: push a `main`, tag push (`v*`), PRs que tocan archivos docker/workflow, dispatch manual
- `Publish Release`: `release.published` después de que `release-please` cree el GitHub Release canónico
- `Release Please Beta`: push a `beta`, dispatch manual

## Nota de Gobernanza de Release Estable

- `release-please` es dueño del PR estable de release, del tag canónico `vX.Y.Z`, del GitHub Release canónico y de las notas de release.
- `publish-release.yml` y `_publish.yml` arrancan desde `release.published` y adjuntan artefactos al GitHub Release existente.
- Esto mantiene a `release-please` como la única autoridad canónica de notas de release mientras la publicación de assets ocurre después de que el release ya existe.
- `release-please-beta.yml` es dueño del PR beta, del tag canónico `vX.Y.Z-beta.N`, del GitHub Release beta y de las notas del canal beta.
- `_publish.yml` solo publica artefactos beta cuando `release-please-beta.yml` lo llama con `prerelease: true`.
- `Security Audit`: push a `main`, PRs a `main`, schedule semanal
- `Workflow Sanity`: PR/push cuando cambian `.github/workflows/**`, `.github/*.yml`, o
  `.github/*.yaml`
- `PR Labeler`: eventos lifecycle de `pull_request_target`
- `Auto Response`: issue abierto, `pull_request_target` abierto
- `Stale`: schedule diario, dispatch manual
- `PR Hygiene`: schedule cada 12 horas, dispatch manual

## Guía de Triaje Rápido

1. `CI Required Gate` fallando: empezar con `.github/workflows/ci.yml`.
2. Failures de Docker en PRs: inspeccionar job `pr-smoke` en `.github/workflows/docker.yml`.
3. Failures de release estable: inspeccionar `.github/workflows/release-please.yml` y `.github/workflows/publish-release.yml`.
4. Failures de release beta: inspeccionar `.github/workflows/release-please-beta.yml`.
5. Failures de seguridad: inspeccionar `.github/workflows/security.yml` y `deny.toml`.
6. Failures de sintaxis/lint de workflow: inspeccionar `.github/workflows/workflow-sanity.yml`.

## Reglas de Mantenimiento

- Mantener merge-blocking checks deterministas y reproducibles (`--locked` donde aplica).
- Preferir permisos de workflow explícitos (least privilege).
- Usar path filters para workflows costosos cuando sea práctico.
- Evitar mezclar automatización de onboarding/community con lógica de merge-gating.
