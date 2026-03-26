---
title: "Mapa de Workflows de CI"
description: Mapa de referencia de los workflows de GitHub, sus responsabilidades, condiciones de disparo y si deben bloquear merges.
owner: team-runtime
status: canonical
lastReviewed: 2026-03-26
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
- `.github/workflows/release.yml` (`Release`)
  - Propósito: Build de artifacts etiquetados y publicación de GitHub releases

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
- `Release`: tag push (`v*`)
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
3. Failures de Release en tags: inspeccionar `.github/workflows/release.yml`.
4. Failures de seguridad: inspeccionar `.github/workflows/security.yml` y `deny.toml`.
5. Failures de sintaxis/lint de workflow: inspeccionar `.github/workflows/workflow-sanity.yml`.

## Reglas de Mantenimiento

- Mantener merge-blocking checks deterministas y reproducibles (`--locked` donde aplica).
- Preferir permisos de workflow explícitos (least privilege).
- Usar path filters para workflows costosos cuando sea práctico.
- Evitar mezclar automatización de onboarding/community con lógica de merge-gating.
