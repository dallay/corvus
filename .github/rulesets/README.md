# Configuración de Rulesets para GitHub

Este directorio contiene las configuraciones de rulesets para proteger las ramas del repositorio.

## Archivos

- `main-protection.json` - Protección estricta para la rama principal
- `minor-protection.json` - Protección moderada para la rama minor

## Cómo importar los rulesets

### Opción 1: Importar desde la interfaz web de GitHub

1. Ve a la página principal del repositorio: https://github.com/dallay/corvus
2. Haz clic en **Settings** (pestaña de configuración)
3. En el menú lateral izquierdo, bajo **Code and automation**, haz clic en **Rules** → **Rulesets**
4. Haz clic en **New ruleset** → **Import a ruleset**
5. Selecciona el archivo JSON correspondiente (ej: `main-protection.json`)
6. Haz clic en **Create**
7. Repite el proceso para el otro archivo

### Opción 2: Usando GitHub CLI (con token de admin)

```bash
# Para main
gh api \
  --method POST \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/dallay/corvus/rulesets \
  --input .github/rulesets/main-protection.json

# Para minor
gh api \
  --method POST \
  -H "Accept: application/vnd.github+json" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  /repos/dallay/corvus/rulesets \
  --input .github/rulesets/minor-protection.json
```

## Qué hace cada ruleset

### main-protection

**Objetivo**: Rama `main` (default)

**Reglas**:

- ✅ Requiere Pull Request con al menos 1 aprobación
- ✅ Requiere que los checks de CI pasen (`core-check`)
- ✅ Requiere historial lineal (no merge commits)
- ✅ Bloquea borrado de la rama
- ✅ Bloquea force push
- ✅ Requiere commits firmados

**Bypass**: Solo administradores pueden hacer bypass via PR

### minor-protection

**Objetivo**: Rama `minor`

**Reglas**:

- ✅ Pull Request recomendado (0 aprobaciones requeridas)
- ✅ Checks de CI opcionales
- ✅ Bloquea borrado de la rama
- ✅ Bloquea force push

**Bypass**: Administradores pueden hacer bypass directo

## Notas de seguridad

Los rulesets son más flexibles que las branch protection rules tradicionales porque:

- Permiten bypass granular por roles
- Soportan múltiples condiciones (ref name patterns)
- Permiten evaluación sin bloqueo (modo "evaluate")
- Funcionan con fnmatch patterns para ramas

## Referencias

- [Documentación de GitHub sobre Rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
- [Available rules for rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)
