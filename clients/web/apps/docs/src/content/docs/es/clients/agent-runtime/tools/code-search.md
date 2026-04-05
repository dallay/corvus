---
title: code_search
summary: Guía de rollout, evidencia de benchmarks y comportamiento actual de la herramienta nativa code_search.
description: Guía respaldada por benchmarks para el rollout de code_search nativo, incluyendo fallback, limitaciones y cuándo preferirlo sobre búsqueda por shell.
owner: team-runtime
status: canonical
lastReviewed: 2026-04-05
appliesTo: main
docType: guide
---

# `code_search`

`code_search` es la herramienta nativa de búsqueda en el workspace dentro del runtime Rust. Soporta corrección para búsquedas literales y regex, devuelve coincidencias estructuradas y siempre trata la verificación en vivo sobre el contenido actual de los archivos como la fuente de verdad.

## Comportamiento verificado actual

- El planning indexado solo se intenta para consultas elegibles para reducción por índice trigram: consultas literales compatibles cuando existe un índice trigram compatible y fresco.
- Las consultas regex están **soportadas para corrección y seguridad**, pero la reducción de candidatos trigram por índice **no** soporta regex en v1.
- El planning regex devuelve `query_regex_not_supported` antes de cargar índice, y la ejecución se etiqueta como `fallback_discovery_live_verification` mientras continúa por discovery más live verification.
- `index_unavailable` aplica solo cuando una consulta literal que sí sería elegible para índice no encuentra un índice trigram compatible; esas corridas se etiquetan como `index_unavailable` y continúan por discovery más live verification.
- Las coincidencias finales siempre salen de la verificación en vivo del contenido actual. Los candidatos indexados por sí solos nunca son resultados autoritativos.

## Para qué sirve esta página

Esta página es la fuente de evidencia de rollout para el Issue #360. Está separada intencionalmente de los microbenchmarks de Criterion en `clients/agent-runtime/benches/agent_benchmarks.rs`.

- **Benches de Criterion**: solo microbenchmarks de bajo nivel.
- **Runner de rollout**: baseline real por `ShellTool`, nativo sin índice, nativo con cold-build, nativo con warm-index y comparación de paridad por línea canónica.

Runner:

```bash
cargo run --manifest-path clients/agent-runtime/Cargo.toml \
  --example code_search_rollout_benchmark -- \
  --workspace both \
  --repo-path /path/to/repo \
  --samples 5 \
  --cold-build-samples 2
```

## Metodología del benchmark

### Baseline de shell

El baseline de shell usa la ruta real de la herramienta `shell` con `NativeRuntime`, que ejecuta `grep` a través de `sh -c`. Eso preserva el mismo wrapping y los mismos chequeos de política que usan hoy los flujos del agente.

### Modos nativos

- `native_no_index`: elimina `state/code-search/index.db` antes de cada corrida medida.
- `native_cold_build`: elimina el índice, mide `refresh_or_rebuild()` y luego mide la primera búsqueda de `code_search`.
- `native_warm_index`: construye o refresca una vez y luego mide búsquedas repetidas con el índice reusable presente.

### Reglas de paridad

La paridad compara resultados de shell y nativos como coincidencias canónicas por línea:

```text
archivo + línea + contenido completo de la línea
```

Este harness de rollout solo hace recomendaciones sobre filas donde la paridad pasa.

### Entorno registrado

#### Workspace fixture determinístico

- tipo de workspace: `fixture`
- generado por: `clients/agent-runtime/examples/code_search_rollout_benchmark.rs`
- cantidad de archivos: `4`
- fecha del benchmark: `2026-04-05T19:34:18.060248+00:00`
- host: macOS / aarch64 / Apple M2 Max
- perfil de Rust: `debug`

#### Snapshot actual del repo

- tipo de workspace: `repo_snapshot`
- raíz del workspace: `<redacted>`
- commit SHA: `82fa4896`
- cantidad de archivos: `234763`
- fecha del benchmark: `2026-04-05T19:47:11.665525+00:00`
- host: macOS / aarch64 / Apple M2 Max
- perfil de Rust: `debug`

## Matriz de benchmark

El runner de rollout registra estos seis casos representativos en ambos workspaces:

| Caso | Tipo de consulta | Forma del resultado | Nota |
| --- | --- | --- | --- |
| `literal_small_hit` | literal | small-hit | una o pocas líneas coincidentes |
| `literal_large_hit` | literal | large-hit | muchas líneas coincidentes |
| `literal_no_hit` | literal | no-hit | literal sin coincidencias |
| `regex_small_hit` | regex | small-hit | modo regex, etiquetado como fallback |
| `regex_large_hit` | regex | large-hit | modo regex, etiquetado como fallback |
| `regex_no_hit` | regex | no-hit | regex sin coincidencias, etiquetado como fallback |

## Resultados registrados

### Workspace fixture determinístico

| Caso | Modo | Modo de plan | Razón | Samples | Median ms | P95 ms | Build median ms | Search median ms | Total median ms | Paridad |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| literal_small_hit | shell_baseline | — | shell_grep_baseline | 5 | 243 | 244 | — | — | — | baseline |
| literal_small_hit | native_no_index | index_unavailable | index_unavailable | 5 | 14 | 15 | — | 14 | 14 | pass |
| literal_small_hit | native_cold_build | indexed_narrowing | indexed_candidates_complete | 2 | 37 | 37 | 4 | 16 | 37 | pass |
| literal_small_hit | native_warm_index | indexed_narrowing | indexed_candidates_complete | 5 | 33 | 33 | — | 16 | 33 | pass |
| literal_large_hit | shell_baseline | — | shell_grep_baseline | 5 | 244 | 245 | — | — | — | baseline |
| literal_large_hit | native_no_index | index_unavailable | index_unavailable | 5 | 15 | 17 | — | 15 | 15 | pass |
| literal_large_hit | native_cold_build | indexed_narrowing | indexed_candidates_complete | 2 | 36 | 36 | 4 | 16 | 36 | pass |
| literal_large_hit | native_warm_index | indexed_narrowing | indexed_candidates_complete | 5 | 32 | 33 | — | 16 | 32 | pass |
| literal_no_hit | shell_baseline | — | shell_grep_baseline | 5 | 249 | 250 | — | — | — | baseline |
| literal_no_hit | native_no_index | index_unavailable | index_unavailable | 5 | 14 | 15 | — | 14 | 14 | pass |
| literal_no_hit | native_cold_build | indexed_narrowing | indexed_candidates_complete | 2 | 37 | 37 | 4 | 16 | 37 | pass |
| literal_no_hit | native_warm_index | indexed_narrowing | indexed_candidates_complete | 5 | 32 | 33 | — | 16 | 32 | pass |
| regex_small_hit | shell_baseline | — | shell_grep_baseline | 5 | 251 | 251 | — | — | — | baseline |
| regex_small_hit | native_no_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 14 | 15 | — | 14 | 14 | pass |
| regex_small_hit | native_cold_build | fallback_discovery_live_verification | query_regex_not_supported | 2 | 18 | 18 | 4 | 13 | 18 | pass |
| regex_small_hit | native_warm_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 14 | 14 | — | 14 | 14 | pass |
| regex_large_hit | shell_baseline | — | shell_grep_baseline | 5 | 245 | 246 | — | — | — | baseline |
| regex_large_hit | native_no_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 14 | 14 | — | 14 | 14 | pass |
| regex_large_hit | native_cold_build | fallback_discovery_live_verification | query_regex_not_supported | 2 | 18 | 18 | 4 | 14 | 18 | pass |
| regex_large_hit | native_warm_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 14 | 14 | — | 14 | 14 | pass |
| regex_no_hit | shell_baseline | — | shell_grep_baseline | 5 | 244 | 244 | — | — | — | baseline |
| regex_no_hit | native_no_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 15 | 16 | — | 15 | 15 | pass |
| regex_no_hit | native_cold_build | fallback_discovery_live_verification | query_regex_not_supported | 2 | 18 | 18 | 4 | 13 | 18 | pass |
| regex_no_hit | native_warm_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 13 | 14 | — | 13 | 13 | pass |

### Snapshot actual del repo

| Caso | Modo | Modo de plan | Razón | Samples | Median ms | P95 ms | Build median ms | Search median ms | Total median ms | Paridad |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| literal_small_hit | shell_baseline | — | shell_grep_baseline | 5 | 237 | 239 | — | — | — | baseline |
| literal_small_hit | native_no_index | index_unavailable | index_unavailable | 5 | 29 | 29 | — | 29 | 29 | pass |
| literal_small_hit | native_cold_build | indexed_narrowing | indexed_candidates_complete | 2 | 27897 | 27897 | 27239 | 326 | 27897 | pass |
| literal_small_hit | native_warm_index | indexed_narrowing | indexed_candidates_complete | 5 | 654 | 656 | — | 326 | 654 | pass |
| literal_large_hit | shell_baseline | — | shell_grep_baseline | 5 | 246 | 257 | — | — | — | baseline |
| literal_large_hit | native_no_index | index_unavailable | index_unavailable | 5 | 33 | 33 | — | 33 | 33 | pass |
| literal_large_hit | native_cold_build | indexed_narrowing | indexed_candidates_complete | 2 | 27706 | 27706 | 27039 | 338 | 27706 | pass |
| literal_large_hit | native_warm_index | indexed_narrowing | indexed_candidates_complete | 5 | 662 | 667 | — | 337 | 662 | pass |
| literal_no_hit | shell_baseline | — | shell_grep_baseline | 5 | 249 | 251 | — | — | — | baseline |
| literal_no_hit | native_no_index | index_unavailable | index_unavailable | 5 | 28 | 28 | — | 28 | 28 | pass |
| literal_no_hit | native_cold_build | indexed_narrowing | indexed_candidates_complete | 2 | 28071 | 28071 | 27385 | 342 | 28071 | pass |
| literal_no_hit | native_warm_index | indexed_narrowing | indexed_candidates_complete | 5 | 655 | 656 | — | 327 | 655 | pass |
| regex_small_hit | shell_baseline | — | shell_grep_baseline | 5 | 244 | 246 | — | — | — | baseline |
| regex_small_hit | native_no_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 35 | 36 | — | 35 | 35 | pass |
| regex_small_hit | native_cold_build | fallback_discovery_live_verification | query_regex_not_supported | 2 | 27751 | 27751 | 27708 | 42 | 27751 | pass |
| regex_small_hit | native_warm_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 36 | 36 | — | 36 | 36 | pass |
| regex_large_hit | shell_baseline | — | shell_grep_baseline | 5 | 248 | 250 | — | — | — | baseline |
| regex_large_hit | native_no_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 32 | 32 | — | 32 | 32 | pass |
| regex_large_hit | native_cold_build | fallback_discovery_live_verification | query_regex_not_supported | 2 | 27997 | 27997 | 27957 | 39 | 27997 | pass |
| regex_large_hit | native_warm_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 32 | 33 | — | 32 | 32 | pass |
| regex_no_hit | shell_baseline | — | shell_grep_baseline | 5 | 248 | 323 | — | — | — | baseline |
| regex_no_hit | native_no_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 28 | 28 | — | 28 | 28 | pass |
| regex_no_hit | native_cold_build | fallback_discovery_live_verification | query_regex_not_supported | 2 | 26974 | 26974 | 26939 | 35 | 26974 | pass |
| regex_no_hit | native_warm_index | fallback_discovery_live_verification | query_regex_not_supported | 5 | 29 | 29 | — | 29 | 29 | pass |

## Guía de rollout

### SHOULD prefer native `code_search`

Prefiere `code_search` nativo para corridas regex medidas cuando la paridad pasa y la ejecución se queda en fallback `native_no_index` o `native_warm_index`. No extiendas esta recomendación a `native_cold_build`, donde las filas medidas del snapshot del repo todavía favorecen a shell porque la construcción del índice domina la corrida.

Por qué:

- cada fila usada para recomendar pasó la paridad canónica,
- las filas regex soportadas están etiquetadas explícitamente como fallback (`query_regex_not_supported` → discovery más live verification), no como reducción trigram regex-aware por índice,
- en esta corrida local en `debug`, las filas regex de `native_no_index` y `native_warm_index` fueron materialmente más rápidas que el baseline de shell, mientras `native_cold_build` quedó como la excepción.

### MAY prefer native `code_search`

Usa `code_search` nativo para búsquedas literales cuando se cumpla alguna de estas condiciones:

- ya tienes un índice reusable y warm para el workspace,
- te importa más la salida estructurada y los offsets verificados que la latencia cruda del shell,
- el workspace es lo bastante pequeño para que el costo de build y reuse del índice siga siendo bajo.

### MAY keep shell / grep

Shell sigue siendo una opción razonable para búsquedas literales one-shot en un repo grande cuando **no** tienes un índice reusable listo.

Por qué:

- las filas cold-build del snapshot del repo están dominadas por la construcción del índice del workspace completo en esta corrida `debug`,
- las filas literales warm-index del snapshot (`654`-`662 ms`) siguieron siendo más lentas que el baseline de shell (`237`-`249 ms`) para los casos literales medidos,
- este cambio **no** depreca la búsqueda por shell y **no** reclama preferencia nativa para casos no medidos.

## Limitaciones que importan para rollout

- La reducción de candidatos por índice para regex **no** está implementada en v1.
- `query_regex_not_supported` es una razón de fallback del planner, no un error de búsqueda.
- El runner actual reconstruye el índice para todo el workspace, así que búsquedas literales acotadas dentro de repos muy grandes pueden verse peor que shell en las filas cold-build.
- Estos números vienen de una corrida local en perfil `debug`. Si se re-ejecuta en otro entorno, los números absolutos pueden cambiar, pero las etiquetas de modo de plan y paridad deben mantenerse.

## Optimizaciones futuras (non-v1)

Estos puntos **no** son requeridos para la recomendación actual de rollout:

- reducción de candidatos regex-aware por índice,
- reducción por índice para búsqueda case-insensitive o whole-word,
- builds parciales del índice conscientes del scope del workspace,
- reruns en perfil `release` para refrescar la evidencia publicada,
- exportar un artefacto machine-readable además de las tablas markdown.

Esos puntos son trabajo futuro de optimización, no bloqueadores para la guía actual respaldada por benchmarks.
