---
title: Configuración de GPG para Publicar en Maven Central
---

Esta guía explica cómo configurar claves GPG para publicar artefactos en Maven Central usando GitHub Actions.

## Requisitos Previos

- GPG instalado: `brew install gnupg` (macOS) o `apt install gnupg` (Linux)
- Cuenta en Sonatype: https://central.sonatype.com

---

## Paso 1: Generar tu Par de Claves GPG

```bash
# Generar una nueva clave GPG
gpg --full-gen-key
```

**Configuración recomendada:**
- **Tipo de clave:** RSA y RSA (por defecto)
- **Tamaño:** `4096` bits (mínimo requerido por Maven Central)
- **Validez:** `0` = sin expiración (o la duración que prefieras)
- **Nombre:** Tu nombre real
- **Correo:** El correo asociado a tu cuenta de GitHub
- **Contraseña:** Una contraseña segura (guárdala bien)

---

## Paso 2: Crear una Subclave de Firma (Recomendado para CI/CD)

Usar subclaves es más seguro porque pueden ser revocadas independientemente de la clave maestra.

```bash
# Lista tus claves para encontrar el ID
gpg --list-secret-keys

# Ejemplo de salida:
# sec   rsa4096 2026-02-11 [SC]
#       ABCD1234EFGH5678IJKL9012MNOP3456QR
# uid           [ultimate] Tu Nombre <tu@email.com>
# ssb   rsa4096 2026-02-11 [E]

# El ID de tu clave es los últimos 16 caracteres después de "sec rsa4096"
# Ejemplo: ABCD1234EFGH5678IJKL9012MNOP3456QR
```

**Crear una subclave dedicada para firma:**

```bash
gpg --edit-key TU_ID_DE_CLAVE

# En el prompt de GPG:
gpg> addkey
# Selecciona: (4) RSA (solo firma)
# Tamaño: 4096
# Duración: 2y (2 años, o lo que prefieras)
# Confirmar: y
# Contraseña: ingresa o usa la misma que la clave maestra

gpg> save
gpg> quit
```

---

## Paso 3: Exportar tu Clave Privada en ASCII

**Opción A: Solo subclave (RECOMENDADO para CI/CD)**

```bash
# Exportar la CLAVE PRIVADA (subclave) en formato ASCII
gpg --export-secret-keys --armor TU_ID_SUBCLAVE > private-subkey.asc

# Verifica que comience con:
# -----BEGIN PGP PRIVATE KEY BLOCK-----

# Muestra el contenido (para copiar):
cat private-subkey.asc

# Copia TODO el contenido incluyendo las líneas BEGIN y END
```

**Opción B: Clave completa (si necesitas también la maestra)**

```bash
gpg --export-secret-keys --armor TU_ID_DE_CLAVE > private-key.asc
```

---

## Paso 4: Obtener el ID de tu Subclave

```bash
# Lista las claves secretas para identificar la subclave de firma
gpg --list-secret-keys

# Ejemplo de salida:
# sec   rsa4096 2026-02-11 [SC]
#       ABCD1234EFGH5678IJKL9012MNOP3456QR
# uid           [ultimate] Tu Nombre <tu@email.com>
# ssb   rsa4096 2026-02-11 [E]    ← Subclave de cifrado
# ssb   rsa4096 2026-02-11 [S]    ← ESTA es la subclave de firma
#       1234567890ABCDEF1234567890ABCDEF12345678

# El ID de la subclave son los últimos 16 caracteres
# Ejemplo: 1234567890ABCDEF1234567890ABCDEF12345678
```

---

## Paso 5: Publicar tu Clave Pública en un Servidor de Claves

```bash
# Enviar al keyserver (elige uno)
gpg --keyserver keyserver.ubuntu.com --send-keys TU_ID_DE_CLAVE

# Otros servidores:
# gpg --keyserver keys.openpgp.org --send-keys TU_ID_DE_CLAVE
# gpg --keyserver pgp.mit.edu --send-keys TU_ID_DE_CLAVE
```

---

## Paso 6: Configurar los Secrets en GitHub

Ve a **GitHub → Repositorio → Settings → Secrets and variables → Actions** y crea estos 4 secrets:

| Nombre del Secret | Valor |
|-------------------|-------|
| `SIGNING_IN_MEMORY_KEY` | **TODO** el contenido de `private-subkey.asc` (incluyendo las líneas BEGIN y END) |
| `SIGNING_IN_MEMORY_KEY_PASSWORD` | La contraseña que usaste al crear la clave |
| `MAVEN_CENTRAL_USERNAME` | Tu usuario de Sonatype |
| `MAVEN_CENTRAL_PASSWORD` | **User Token** de Sonatype (NO tu contraseña) |

---

## Paso 7: Crear un User Token en Sonatype

1. Ve a https://central.sonatype.com
2. Inicia sesión con tu cuenta de Sonatype
3. Haz clic en tu avatar → **Profile**
4. Busca **User Token** o **Generate Token**
5. Crea un token y copia:
   - **Username** → `MAVEN_CENTRAL_USERNAME`
   - **Password** → `MAVEN_CENTRAL_PASSWORD`

**Importante:** Nunca uses un token sin expiración. Rota los tokens periódicamente.

---

## Paso 8: Verificar el Formato de la Clave GPG

Asegúrate de que tu secret `SIGNING_IN_MEMORY_KEY` contenga:

```
-----BEGIN PGP PRIVATE KEY BLOCK-----

lQHYBF...  (clave codificada en base64)
...
-----END PGP PRIVATE KEY BLOCK-----
```

**NO** formato binario (que se vería como caracteres aleatorios).

---

## Pruebas Locales

```bash
# Exporta variables y prueba la publicación
export ORG_GRADLE_PROJECT_signingInMemory_KEY="$(cat private-subkey.asc)"
export ORG_GRADLE_PROJECT_signingInMemory_KEY_PASSWORD="tu_contraseña"
export ORG_GRADLE_PROJECT_mavenCentralUsername="tu_usuario_sonatype"
export ORG_GRADLE_PROJECT_mavenCentralPassword="tu_token_sonatype"

# Prueba (modo dry-run si está disponible)
./gradlew publishToMavenCentral --dry-run

# O publicación real
./gradlew publishToMavenCentral
```

---

## Solución de Problemas

### "gpg: signing failed: Inappropriate ioctl for device"

Esto ocurre en entornos CI sin TTY. La configuración de este proyecto usa claves en memoria, lo que debería evitar este problema.

### Error "No secret key"

Verifica que:
1. La clave se exportó con el flag `--armor` (formato ASCII)
2. El secret incluye `-----BEGIN PGP PRIVATE KEY BLOCK-----`
3. Se usa el ID de clave correcto (si usas subclave)

### Clave expirada

Para extender la expiración:

```bash
gpg --edit-key TU_ID_DE_CLAVE
gpg> list
gpg> key 1        # Selecciona la subclave de firma (segunda clave)
gpg> expire
# Establece nueva expiración
gpg> save
gpg> quit

# Vuelve a publicar en el keyserver
gpg --keyserver keyserver.ubuntu.com --send-keys TU_ID_DE_CLAVE
```

---

## Buenas Prácticas de Seguridad

1. **NUNCA** subas archivos `.asc` o claves privadas al control de versiones
2. **USA SIEMPRE User Tokens** en Sonatype, no tu contraseña de cuenta
3. **Las subclaves expiran**: pon recordatorios para renovarlas antes de que expiren
4. **Haz backup de tu clave maestra** de forma segura (USB cifrado, hardware wallet, etc.)
5. **Considera usar llaves de hardware** (YubiKey) para máxima seguridad
6. **Rota los tokens** periódicamente, especialmente si sospechas de un compromiso

---

## Referencias

- [Maven Central: GPG Signing Requirements](https://central.sonatype.org/publish/requirements/gpg/)
- [GitHub: Usar secretos en workflows](https://docs.github.com/en/actions/how-tos/write-workflows/choose-what-workflows-do/use-secrets)
- [GnuPG Documentation](https://gnupg.org/documentation/)


