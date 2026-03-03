# SonarQube Issues - Proyecto Corvus

## Resumen

| Severidad | Cantidad |
|-----------|----------|
| BLOCKER   | 2        |
| CRITICAL  | ~50+     |
| MAJOR     | ~30+     |
| MINOR     | ~15+     |

---

## 🚨 BLOCKER - Tokens Hardcodeados (RESUELTO)

> **⚠️ IMPORTANTE**: Los BLOCKERs eran FALSOS POSITIVOS - son tests de seguridad que verifican que el sistema sanea correctamente tokens sensibles.

### Issue 1: Token Bearer en gateway/mod.rs línea 2442 ✅ RESUELTO
- **Status**: Marcado como FALSE POSITIVE (es un test de seguridad)

### Issue 2: Token Bearer en gateway/mod.rs línea 2507 ✅ RESUELTO
- **Status**: Marcado como FALSE POSITIVE (es un test de seguridad)

---

## ⚠️ CRITICAL - Cognitive Complexity

### Rust - agent-runtime

| Archivo | Línea | Complejidad | Máximo | Key |
|---------|-------|-------------|--------|-----|
| onboard/wizard.rs | 2770 | 151 | 15 | AZye4oPnIngfSClMZ_ec |
| wizard.rs | 1469 | 97 | 15 | AZye4oPnIngfSClMZ_ea |
| channels/irc.rs | 399 | 91 | 15 | AZye4oJFIngfSClMZ_dt |
| main.rs | 564 | 63 | 15 | AZye4oPLIngfSClMZ_eY |
| config/schema.rs | 2204 | 65 | 15 | AZye4oO8IngfSClMZ_eX |
| channels/mod.rs | 412 | 59 | 15 | AZye4oKCIngfSClMZ_dz |
| memory/sqlite.rs | 480 | 54 | 15 | AZye4oIIIngfSClMZ_dl |
| dashboard/App.vue | 344 | 51 | 15 | AZye4oHdIngfSClMZ_df |
| providers/reliable.rs | 154 | 46 | 15 | AZye4oNkIngfSClMZ_eL |
| memory/surreal.rs | 421 | 41 | 15 | AZye4oIcIngfSClMZ_do |
| doctor/mod.rs | 104 | 34 | 15 | AZye4oSEIngfSClMZ_eu |
| gateway/admin.rs | 309 | 33 | 15 | AZyvhc3zSyFxhe7Jxyxn |
| channels/telegram.rs | 461 | 31 | 15 | AZye4oKwIngfSClMZ_d6 |
| doctor/mod.rs | 385 | 30 | 15 | AZye4oSEIngfSClMZ_ev |
| channels/telegram.rs | 146 | 29 | 15 | AZye4oKwIngfSClMZ_d5 |
| memory/sqlite.rs | 686 | 27 | 15 | AZye4oIIIngfSClMZ_dm |
| gateway/mod.rs | 1408 | 101 | 15 | AZye4oSaIngfSClMZ_ex |
| gateway/mod.rs | 329 | 145 | 15 | AZye4oSaIngfSClMZ_ew |
| channels/signal.rs | 290 | 77 | 15 | AZye4oLRIngfSClMZ_d- |
| providers/reliable.rs | 260 | 44 | 15 | AZye4oNkIngfSClMZ_eM |
| providers/reliable.rs | 363 | 44 | 15 | AZye4oNkIngfSClMZ_eN |
| channels/telegram.rs | 685 | 22 | 15 | AZye4oKwIngfSClMZ_d7 |
| gateway/mod.rs | 1660 | 22 | 15 | AZyzxRvnPYwQPzQi6jnD |
| agent/agent.rs | 472 | 22 | 15 | AZyzxRuMPYwQPzQi6jnC |
| channels/imessage.rs | 133 | 23 | 15 | AZye4oJlIngfSClMZ_dx |
| channels/mod.rs | 283 | 26 | 15 | AZyzxRlWPYwQPzQi6jnB |
| tools/http_request.rs | 82 | 21 | 15 | AZye4oOZIngfSClMZ_eU |
| channels/telegram.rs | 1522 | 24 | 15 | AZye4oKwIngfSClMZ_d8 |
| channels/mod.rs | 1199 | 17 | 15 | AZye4oKCIngfSClMZ_d1 |
| channels/mod.rs | 1372 | 25 | 15 | AZye4oKCIngfSClMZ_d2 |

### TypeScript/Vue

| Archivo | Línea | Complejidad | Máximo | Key |
|---------|-------|-------------|--------|-----|
| dashboard/App.vue | 344 | 51 | 15 | AZye4oHdIngfSClMZ_df |
| dashboard/App.vue | 370 | ternary | - | AZye4oHdIngfSClMZ_dg |
| chat/App.vue | 115 | 17 | 15 | AZye4oHMIngfSClMZ_dd |
| chat/App.vue | 204 | 17 | 15 | AZye4oHMIngfSClMZ_de |

---

## 🐚 MAJOR - Shell Scripts

### sync-version-with-tag.sh
- Línea 25: S7682 - Add explicit return (AZye4oXfIngfSClMZ_gw)
- Línea 39: S7682 - Add explicit return (AZye4oXfIngfSClMZ_gx)
- Línea 46: S7682 - Add explicit return (AZye4oXfIngfSClMZ_gy)
- Línea 102: S7682 - Add explicit return (AZye4oXfIngfSClMZ_g0)
- Línea 109: S7677 - Redirect to stderr (AZye4oXfIngfSClMZ_gq)
- Línea 130: S7677 - Redirect to stderr (AZye4oXfIngfSClMZ_gr)

### pre-commit.sh
- Línea 25: S7677 - Redirect to stderr (AZye4oW1IngfSClMZ_gh)

### analyze-apk-size.sh
- Líneas 24, 61, 77, 88, 102, 108, 126, 140, 143, 153, 195, 203, 210: S7688 - Use [[ instead of [

### analyze-build-time.sh
- Similar issues de S7688

---

## 🐳 MINOR - Docker

### Dockerfile.alpine
- Línea 19: S7031 - Merge RUN instructions
- Línea 19: S7018 - Sort packages alphanumerically

### Dockerfile.ubuntu
- Línea 20: S7031 - Merge RUN instructions
- Línea 20: S7018 - Sort packages alphanumerically

---

## 🧪 MINOR - Kotlin Test Naming

### DyTest.kt
- Línea 12: S100 - Rename function (AZyvhc-6SyFxhe7Jxyxo)
- Línea 19: S100 - Rename function (AZyvhc-6SyFxhe7Jxyxp)
- Línea 30: S100 - Rename function (AZyvhc-6SyFxhe7Jxyxq)
- Línea 43: S100 - Rename function (AZyvhc-6SyFxhe7Jxyxr)
- Línea 50: S100 - Rename function (AZyvhc-6SyFxhe7Jxyxs)
- Línea 57: S100 - Rename function (AZyvhc-6SyFxhe7Jxyxt)

---

## Plan de Acción

### Fase 1: CRÍTICO - Security
1. Eliminar tokens hardcodeados en gateway/mod.rs

### Fase 2: CRITICAL - Refactoring Cognitive Complexity
1. Priorizar funciones con complejidad > 50
2. Aplicar estrategias de refactoring:
   - Extraer funciones helper
   - Usar patrón Strategy/Command
   - Simplificar condiciones anidadas

### Fase 3: MAJOR - Shell Scripts
1. Corregir scripts uno por uno
2. Usar [[ en lugar de [
3. Agregar returns explícitos

### Fase 4: MINOR - Naming y Docker
1. Renombrar tests de Kotlin
2. Optimizar Dockerfiles
