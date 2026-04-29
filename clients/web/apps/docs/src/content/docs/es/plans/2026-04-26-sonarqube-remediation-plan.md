---
title: "Plan de remediación SonarQube"
date: 2026-04-26
last_updated: 2026-04-26
tags: [sonarqube, remediation, quality, plan]
status: draft
summary: "Plan por lotes para resolver todos los issues abiertos actuales de SonarQube en Corvus por prioridad y dominio."
description: "Plan de remediación de SonarQube a nivel de repositorio que cubre backend, frontend, accesibilidad, scripts y trabajo de seguimiento en Kotlin."
owner: team-platform
lastReviewed: 2026-04-26
appliesTo: corvus runtime, web, and tooling remediation
docType: architecture
---

# Plan de remediación SonarQube

_Esta página refleja el plan en inglés mientras se prepara una traducción completa._

## Objetivo

Resolver todos los issues abiertos actuales de SonarQube para `dallay_corvus` en una secuencia controlada que reduzca primero el riesgo mientras mantiene cada lote de implementación coherente y revisable.

## Alcance

Este plan cubre el conjunto actual de issues abiertos ya identificados en SonarQube, incluyendo:

- issues de complejidad en runtime/gateway/security de Rust
- issues frontend de dashboard y rook-dashboard
- issues de accesibilidad y CSS
- issues de mantenibilidad en scripts shell
- issue de duplicación en Kotlin

## Estrategia de ejecución

Usar **lotes por prioridad + dominio** en lugar de resolver todo en una sola pasada mixta.

## Plan por lotes

Seguir la misma secuencia descrita en la versión en inglés para Batch 1, Batch 2 y Batch 3.

## Restricciones

Mantener cada cambio enfocado, validable y alineado con el comportamiento existente.

## Criterios de éxito

El trabajo queda cerrado cuando los lotes planificados están implementados, validados y no quedan issues abiertos relevantes en SonarQube para este esfuerzo.

## Riesgos y mitigaciones

Los cambios de mayor riesgo son los de runtime, gateway y security; se deben validar con checks dirigidos y revertirse por lote si aparece una regresión.

## Siguiente paso

Usar este plan como guía de ejecución y como resumen del alcance para el seguimiento del remediation work.
