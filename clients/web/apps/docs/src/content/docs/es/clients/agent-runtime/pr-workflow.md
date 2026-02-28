---
title: Corvus PR Workflow (Colaboración de Alto Volumen)
---

Este documento define cómo Corvus maneja un alto volumen de PRs manteniendo:

- Alto rendimiento
- Alta eficiencia
- Alta estabilidad
- Alta extensibilidad
- Alta sostenibilidad
- Alta seguridad

Referencia relacionada: [`ci-map.md`](ci-map.md) para propiedad por workflow, disparadores y flujo de triaje.

## 1) Objetivos de gobernanza

1. Mantener el throughput de merge predecible bajo carga alta de PRs.
2. Mantener la calidad de señal de CI alta (feedback rápido, pocos falsos positivos).
3. Mantener la revisión de seguridad explícita para superficies de riesgo.
4. Mantener los cambios fáciles de razonar y fáciles de revertir.

## 2) Configuración requerida del repositorio

Mantén estas reglas de protección de rama en `main`:

- Requiere status checks antes del merge.
- Requiere el check `CI Required Gate`.
- Requiere revisiones de PR antes del merge.
- Requiere revisión de CODEOWNERS para paths protegidos.
- Descarta aprobaciones stale cuando se hacen nuevos commits.
- Restringe force-push en ramas protegidas.

## 3) Ciclo de vida del PR

### Paso A: Intake

- Contributor abre PR con `.github/pull_request_template.md` completo, incluyendo: problema, cambio, no-objetivos, riesgos y plan de rollback.
- `PR Labeler` aplica labels de path + size.
- `Auto Response` publica guía para contribuidores primerizos.

### Paso B: Validación

- `CI Required Gate` es la merge gate.
- PRs de docs usan fast-path y saltan jobs pesados de Rust.
- PRs no-docs deben pasar lint, tests y smoke check de release build.

### Paso C: Revisión

- Revisores priorizan por riesgo y labels de tamaño.
- Paths sensibles a seguridad (`src/security`, runtime, CI) requieren atención de maintainer.
- PRs grandes (`size: L`/`size: XL`) deben dividirse salvo justificación fuerte.

### Paso D: Merge

- Preferir **squash merge** para mantener historial compacto.
- Título del PR debe seguir estilo Conventional Commit.
- Merge solo cuando el path de rollback está documentado.

## 4) Política de tamaño de PR

- `size: XS` <= 80 líneas cambiadas
- `size: S` <= 250 líneas cambiadas
- `size: M` <= 500 líneas cambiadas
- `size: L` <= 1000 líneas cambiadas
- `size: XL` > 1000 líneas cambiadas

Política:

- Apuntar a `XS/S/M` por defecto.
- `L/XL` PRs necesitan justificación explícita y evidencia de test más fuerte.
- Si una feature grande es inevitable, dividir en PRs apilados.

## 5) Política de contribución IA/agente

PRs asistidos por AI son bienvenidos, y la revisión también puede ser asistida por agent.

Requerido:

1. Resumen claro del PR con límite de scope.
2. Evidencia explícita de test/validación.
3. Impacto de seguridad y notas de rollback para cambios riesgosos.

Recomendado:

1. Breves notas de herramienta/workflow cuando la automatización influyó materialmente el cambio.
2. Snippets opcionales de prompt/plan para reproducibilidad.

**No** requerimos que los contribuidores cuantifiquen propiedad de líneas AI-vs-humano.

Énfasis de revisión para PRs heavy-AI:

- Compatibilidad de contrato
- Límites de seguridad
- Manejo de errores y comportamiento de fallback
- Regresiones de rendimiento y memoria

## 6) SLA de revisión y disciplina de cola

- Primer triaje por maintainer: objetivo en 48 horas.
- Si PR está bloqueado, maintainer deja una lista de verificación accionable.
- Automatización `stale` se usa para mantener cola saludable; maintainers pueden aplicar `no-stale` cuando sea necesario.
- Automatización `pr-hygiene` revisa PRs abiertos cada 12 horas. Se activa cuando un PR no tiene nuevos commits por 48+ horas. El PR se considera bloqueado si está detrás de `main` o si el head commit tiene `CI Required Gate` fallando o missing.

## 7) Reglas de seguridad y estabilidad

Cambios en estas áreas requieren revisión más estricta y evidencia de test más fuerte:

- `src/security/**`
- Manejo de proceso de runtime
- Límites de acceso a filesystem
- Comportamiento de red/autenticación
- GitHub workflows y pipeline de release

Mínimo para PRs riesgosos:

- Declaración de threat/riesgo
- Notas de mitigación
- Pasos de rollback

## 8) Recuperación de fallas

Si un PR mergeado causa regresiones:

1. Revertir PR inmediatamente en `main`.
2. Abrir issue de seguimiento con análisis de root cause.
3. Reintroducir fix solo con tests de regresión.

Preferir restauración rápida de calidad de servicio sobre fixes perfectos tardíos.

## 9) Checklist de maintainer (merge-ready)

- Scope es enfocado y entendible.
- CI gate está verde.
- Campos de impacto de seguridad están completos.
- Notas de workflow de agent son suficientes para reproducibilidad (si automatización fue usada).
- El plan de rollback es explícito.
- Título del commit sigue Conventional Commits.

## 10) Modelo operativo de revisión por agent

Para mantener calidad de revisión estable bajo alto volumen de PRs, usamos un modelo de dos carriles:

### Carril A: Fast triage (agent-friendly)

- Confirmar completitud del template del PR.
- Confirmar señal de CI gate (`CI Required Gate`).
- Confirmar clase de riesgo vía labels y paths tocados.
- Confirmar que existe declaración de rollback.

### Carril B: Deep review (risk-based)

Requerido para cambios de alto riesgo (security/runtime/gateway/CI):

- Validar supuestos del threat model.
- Validar modo de falla y comportamiento de degradación.
- Validar compatibilidad hacia atrás e impacto de migración.
- Validar impacto de observabilidad/logging.

## 11) Prioridad de cola y disciplina de labels

Orden de triaje recomendado:

1. `size: XS`/`size: S` + bug/security fixes
2. `size: M` cambios enfocados
3. `size: L`/`size: XL` requests de split o revisión staged

Disciplina de labels:

- Path labels identifican ownership de subsistema rápidamente.
- Size labels guían estrategia de batching.
- `no-stale` reservado para trabajo aceptado pero bloqueado.

## 12) Contrato de handoff de agent

Cuando un agent hace handoff a otro (o a un maintainer), incluir:

1. Límite de scope (qué cambió / qué no cambió).
2. Evidencia de validación.
3. Riesgos y unknowns abiertos.
4. Siguiente acción sugerida.

Esto mantiene baja la pérdida de contexto y evita deep dives repetidos.
