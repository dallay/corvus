---
title: SurrealDB en Producción con Docker Compose
---

Aquí tienes los pasos para desplegar **SurrealDB en producción con Docker Compose** basado en la
documentación oficial de SurrealDB y prácticas estándar de contenedorización.

## 1) Tipos de almacenamiento en SurrealDB

SurrealDB soporta varios motores de almacenamiento, cada uno con características distintas:

| Motor | Comando Docker | Persistencia | Casos de uso |
|-------|----------------|--------------|---------------|
| **In-Memory** (`mem://`) | Sin especificar | ❌ No | Tests, caché, datos temporales |
| **RocksDB** (`rocksdb://`) | `rocksdb:/surreal/db` | ✅ Sí | Desarrollo, producción single-node |
| **SurrealKV** (`surrealkv://`) | `surrealkv:/surreal/db` | ✅ Sí | Producción (reemplazo moderno de RocksDB) |
| **TiKV** | Configuración cluster | ✅ Sí | Alta disponibilidad, clustering distribuido |

### Detalles de cada tipo

#### In-Memory (`mem://`)
- Almacena todos los datos en RAM
- **Rendimiento más rápido** posible
- Los datos se **pierden** al cerrar la conexión
- Ideal para: tests unitarios, caché, desarrollo rápido

#### RocksDB (`rocksdb://`)
- Motor de almacenamiento basado en key-value
- Persistente en disco
- Adecuado para desarrollo y producción single-node

#### SurrealKV (`surrealkv://`)
- **Motor recomendado** por SurrealDB para producción
- Reemplazo moderno de RocksDB
- Mejor rendimiento y eficiencia

#### TiKV (Clustering)
- Para despliegues distribuidos de alta disponibilidad
- Requiere configuración más compleja (no incluido en Docker Compose simple)

---

## 2) Base de datos de Grafos (funcionalidad nativa)

Una de las características más potentes de SurrealDB es que **soporta grafos de forma nativa e implícita**. No necesitas configuración adicional - es parte del motor de base de datos.

### ¿Qué significa esto?

SurrealDB funciona como base de datos **multi-modelo**, lo que significa que puedes usar:

- **Modelo relacional** (tablas tradicionales)
- **Modelo de documentos** (JSON embebido)
- **Modelo de grafos** (nodos y relaciones)

Todo en la misma base de datos, sin necesidad de herramientas adicionales.

### Cómo usar grafos en SurrealDB

#### Crear nodos (registros normales)

```surql
CREATE person:alice SET name = "Alice", age = 30;
CREATE person:bob SET name = "Bob", age = 25;
CREATE post:1 SET title = "Hello World";
```

#### Crear relaciones (edges)

```surql
-- Alice escribe el post
RELATE person:alice->wrote->post:1;

-- Alice sigue a Bob
RELATE person:alice->follows->person:bob;
```

#### Consultar grafos (sintaxis de flechas)

```surql
-- Encontrar todos los posts de Alice
SELECT ->wrote->post FROM person:alice;

-- Encontrar quién escribió un post
SELECT <-wrote<-person FROM post:1;

-- Encontrar a quién sigue Alice
SELECT ->follows->person FROM person:alice;
```

### Graph vs Record Links

SurrealDB tiene dos formas de relacionar registros:

| Característica | Record Links | Graph Relations |
|----------------|--------------|-----------------|
| **Dirección** | Unidireccional | Bidireccional |
| **Metadatos** | ❌ No | ✅ Sí (puedes guardar datos en la relación) |
| **Rendimiento** | Más rápido | Flexible |
| **Caso de uso** | Referencias simples | Relaciones complejas con contexto |

**Usa Record Links cuando:**
- Solo necesitas referenciar un registro desde otro
- El rendimiento es crítico
- No necesitas metadatos en la relación

**Usa Graph Relations cuando:**
- Necesitas relaciones bidireccionales
- Quieres guardar información sobre la relación (ej: "fecha de creación", "peso")
- Vas a hacer consultas complejas de traversal

> 📖 **Más información**: Ver [Graph Database en SurrealDB](https://surrealdb.com/docs) para ejemplos avanzados.

---

## 3) Claves de diseño para un Stack de producción

Debes satisfacer estos puntos antes de poner SurrealDB en producción:

- **Persistencia de datos** en volumen externo (ej. RocksDB/SurrealKV).
- **Configuración de credenciales** seguras mediante variables de entorno.
- **Exposición de puertos** controlada por ambiente (no mapeos abiertos indiscriminados).
- **Política de reinicio automática** (`restart: unless-stopped` o `always`).
- **Imagen fija con tag de versión**, NO `latest`.
- **Logging** y, si aplica, TLS.
- **Healthcheck** para verificación de salud del servicio.

## 4) Ejemplo de `docker-compose.yml` para producción

> ⚠️ **Best Practice de Seguridad**: Para producción, usa **secrets** de Docker para datos sensibles
> (usuarios, contraseñas) en lugar de variables de entorno. Ver [documentación de Docker Secrets](https://docs.docker.com/engine/swarm/secrets/).

### Con RocksDB (estándar) - Usando Secrets

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1         # fija una versión concreta
    command:
      - sh
      - -c
      - >
        surreal start
        --bind 0.0.0.0:8000
        --user "$$(cat /run/secrets/surreal_user)"
        --pass "$$(cat /run/secrets/surreal_pass)"
        rocksdb:/surreal/db                   # persistencia RocksDB en volumen
    secrets:
      - surreal_user
      - surreal_pass
    ports:
      - "${SURREAL_PORT:-8000}:8000"         # solo expone si es necesario
    volumes:
      - surreal_data:/surreal/db             # volumen persistente
    restart: unless-stopped                  # política de reinicio prod
    logging:
      driver: "json-file"
      options:
        max-size: "50m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "surreal", "is-ready", "--endpoint", "http://localhost:8000"]
      interval: 30s
      timeout: 10s
      retries: 5

secrets:
  surreal_user:
    file: ./secrets/surreal_user.txt        # contenido: root
  surreal_pass:
    file: ./secrets/surreal_pass.txt         # contenido: tu_contraseña_segura

volumes:
  surreal_data:
```

### Con RocksDB - Usando archivo .env (solo desarrollo)

Para desarrollo local, puedes usar archivos `.env` (añadir a `.gitignore`):

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    command: >
      start
      --bind 0.0.0.0:8000
      --user ${SURREAL_USER}
      --pass ${SURREAL_PASS}
      rocksdb:/surreal/db
    env_file:
      - .env                                 # solo para dev local - ¡AÑADIR A .gitignore!
    ports:
      - "${SURREAL_PORT:-8000}:8000"
    volumes:
      - surreal_data:/surreal/db
    restart: unless-stopped
    logging:
      driver: "json-file"
      options:
        max-size: "50m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "surreal", "is-ready", "--endpoint", "http://localhost:8000"]
      interval: 30s
      timeout: 10s
      retries: 5

volumes:
  surreal_data:
```

### Con SurrealKV (recomendado para producción)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    command:
      - sh
      - -c
      - >
        surreal start
        --bind 0.0.0.0:8000
        --user "$$(cat /run/secrets/surreal_user)"
        --pass "$$(cat /run/secrets/surreal_pass)"
        surrealkv:/surreal/db                # SurrealKV (recomendado)
    secrets:
      - surreal_user
      - surreal_pass
    # ... resto de configuración igual
```

### In-Memory (solo desarrollo/tests)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    command:
      - sh
      - -c
      - >
        surreal start
        --bind 0.0.0.0:8000
        --user "$$(cat /run/secrets/surreal_user)"
        --pass "$$(cat /run/secrets/surreal_pass)"
        mem                                      # In-memory (NO persistente)
    secrets:
      - surreal_user
      - surreal_pass
    # NO volumes para in-memory
    # ADVERTENCIA: datos se pierden al reiniciar
```

## 5) Variables de entorno y Secrets

> Nota: en Docker Compose se usa `$$` para escapar `$` y evitar interpolación en parseo.

### Para Producción: Docker Secrets

Crea archivos de secrets (nunca hacer commit):

```bash
mkdir -p secrets
echo "root" > secrets/surreal_user.txt
echo "S0m3$3cur3P@ss" > secrets/surreal_pass.txt
chmod 600 secrets/*.txt
```

Añadir a `.gitignore`:
```
secrets/
.env
```

### Para Desarrollo: archivo .env

Este archivo **no debe guardarse en repositorio público** (añadir a `.gitignore`):

```bash
# .env - ¡Solo desarrollo!
SURREAL_USER=root
SURREAL_PASS=S0m3$3cur3P@ss
SURREAL_PORT=8000
```

> 📖 Ver [Docker Compose Environment Variables Best Practices](https://docs.docker.com/compose/how-tos/environment-variables/best-practices/) para más detalles.

## 6) Notas técnicas clave

### Persistencia de datos

El comando `start ... rocksdb:/surreal/db` (o `surrealkv:/surreal/db`) hace que SurrealDB use un
motor de almacenamiento en disco. **Sin él se queda in-memory** (no persistente):

- `rocksdb:/surreal/db` → Persistente con RocksDB
- `surrealkv:/surreal/db` → Persistente con SurrealKV (recomendado)
- `mem` → Solo RAM (se pierde al cerrar)

### Credenciales y autenticación

SurrealDB habilita autenticación por defecto. Debes suministrar `--user` y `--pass` o usar
variables de entorno para inicializar el root.

### Resiliencia

Incluye un healthcheck y una política de reinicio para asegurar operación continua si el
contenedor falla. Esto es estándar en producción.

### Logs

La configuración de logs limita tamaño y rotación. Ajusta en función de observabilidad requerida.

## 7) Consideraciones de seguridad y networking

- En producción es recomendable colocar SurrealDB **detrás de un proxy inverso** (NGINX/Traefik) y
  **TLS termination**.
- Usa redes privadas Docker si ejecutas otros servicios (p. ej., backend) para no exponer el
  puerto al público.
- **No uses `latest` en producción**; fijar la versión obliga a control de releases.

## 8) Opcionales para entornos enterprise

Si vas a autoscalar o tener múltiples nodos:

- Replica la configuración con **clustering** (SurrealDB soporta multi-nodo, pero no con este
  simple Compose; revisa el repositorio oficial de SurrealDB para clusters).
- Añade **backup automatizado** de los volúmenes.
- Integración con sistemas de métricas (**Prometheus**, ELK).

## Recursos

### Documentación Oficial

- [Running SurrealDB with Docker](https://surrealdb.com/docs/surrealdb/installation/running/docker) - Guía oficial de instalación con Docker
- [SurrealQL](https://surrealdb.com/docs/surrealql) - Lenguaje de consulta de SurrealDB
- [Graph Database en SurrealDB](https://surrealdb.com/docs) - Funcionalidades de grafos nativas

### Repositorios

- [SurrealDB Docker GitHub](https://github.com/surrealdb/docker.surrealdb.com) - Configuraciones Docker oficiales
- [SurrealDB GitHub](https://github.com/surrealdb/surrealdb) - Repositorio principal

### Aprendizaje

- [SurrealDB Fundamentals Course](https://surrealdb.com/learn/fundamentals) - Curso oficial gratuito
- [Aeon's Surreal Renaissance](https://surrealdb.com/learn/book) - Libro avanzado sobre SurrealDB
