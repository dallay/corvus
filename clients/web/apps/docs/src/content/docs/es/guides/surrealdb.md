---
title: SurrealDB en Producción con Docker Compose
---

Aquí tienes los pasos para desplegar **SurrealDB en producción con Docker Compose** basado en la
documentación oficial de SurrealDB y prácticas estándar de contenedorización.

Esta guía incluye correcciones prácticas de operación para SurrealDB v3 validadas en un servidor
real.

## 1) Tipos de almacenamiento en SurrealDB

SurrealDB soporta varios motores de almacenamiento, cada uno con características distintas:

| Motor                          | Comando Docker          | Persistencia | Casos de uso                                |
|--------------------------------|-------------------------|--------------|---------------------------------------------|
| **In-Memory** (`mem://`)       | Sin especificar         | ❌ No         | Tests, caché, datos temporales              |
| **RocksDB** (`rocksdb://`)     | `rocksdb:/surreal/db`   | ✅ Sí         | Desarrollo, producción single-node          |
| **SurrealKV** (`surrealkv://`) | `surrealkv:/surreal/db` | ✅ Sí         | Producción (reemplazo moderno de RocksDB)   |
| **TiKV**                       | Configuración cluster   | ✅ Sí         | Alta disponibilidad, clustering distribuido |

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

Una de las características más potentes de SurrealDB es que **soporta grafos de forma nativa e
implícita**. No necesitas configuración adicional - es parte del motor de base de datos.

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

| Característica  | Record Links        | Graph Relations                            |
|-----------------|---------------------|--------------------------------------------|
| **Dirección**   | Unidireccional      | Bidireccional                              |
| **Metadatos**   | ❌ No                | ✅ Sí (puedes guardar datos en la relación) |
| **Rendimiento** | Más rápido          | Flexible                                   |
| **Caso de uso** | Referencias simples | Relaciones complejas con contexto          |

**Usa Record Links cuando:**

- Solo necesitas referenciar un registro desde otro
- El rendimiento es crítico
- No necesitas metadatos en la relación

**Usa Graph Relations cuando:**

- Necesitas relaciones bidireccionales
- Quieres guardar información sobre la relación (ej: "fecha de creación", "peso")
- Vas a hacer consultas complejas de traversal

> 📖 **Más información**: Ver [Graph Database en SurrealDB](https://surrealdb.com/docs/surrealdb)
> para ejemplos
> avanzados.

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

> Importante para la imagen de SurrealDB v3:
>
> - El entrypoint ya es `/surreal`.
> - En Compose se usa `command: start ...` (no `surreal start ...`).
> - `sh -c ...` no funciona porque la imagen no trae `/bin/sh`.

### Con SurrealKV (recomendado para producción)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    env_file:
      - ./secrets/surreal.env                 # chmod 600, nunca hacer commit
    command: >
      start
      --bind 0.0.0.0:8000
      surrealkv:/surreal/db
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
      test: [ "CMD", "surreal", "isready", "--endpoint", "http://localhost:8000" ]
      interval: 30s
      timeout: 10s
      retries: 5

volumes:
  surreal_data:
```

### Con RocksDB (single-node)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    env_file:
      - ./secrets/surreal.env
    command: >
      start
      --bind 0.0.0.0:8000
      rocksdb:/surreal/db
    # mismos ajustes de ports / volumes / restart / logging / healthcheck
```

### In-Memory (solo desarrollo/tests)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    command: >
      start
      --bind 0.0.0.0:8000
      mem
    # NO volumes para in-memory
    # ADVERTENCIA: datos se pierden al reiniciar
```

## 5) Variables de entorno y Secrets

### Para Producción: env file protegido (Compose)

Crea un archivo protegido (nunca hacer commit):

```bash
mkdir -p secrets
cat > secrets/surreal.env << 'EOF'
SURREAL_USER=root
SURREAL_PASS=S0m3$3cur3P@ss
SURREAL_PORT=8000
EOF
chmod 600 secrets/surreal.env
```

Añadir a `.gitignore`:

```
secrets/
.env
```

### Para Desarrollo: archivo `.env`

Este archivo **no debe guardarse en repositorio público**:

```bash
# .env - ¡Solo desarrollo!
SURREAL_USER=root
SURREAL_PASS=S0m3$3cur3P@ss
SURREAL_PORT=8000
```

> Si haces bootstrap con `--user`/`--pass` en argumentos del comando, elimínalos después del
> primer arranque correcto para evitar exponer credenciales en inspecciones de procesos.

### Túnel SSH desde macOS

```bash
ssh -o ExitOnForwardFailure=yes -fN -L 8000:127.0.0.1:8000 corvus
curl -i http://localhost:8000/status
```

## 6) Notas técnicas clave

### Persistencia de datos

El comando `start ... rocksdb:/surreal/db` (o `surrealkv:/surreal/db`) hace que SurrealDB use un
motor de almacenamiento en disco. **Sin él se queda in-memory** (no persistente):

- `rocksdb:/surreal/db` → Persistente con RocksDB
- `surrealkv:/surreal/db` → Persistente con SurrealKV (recomendado)
- `mem` → Solo RAM (se pierde al cerrar)

### Credenciales y autenticación

SurrealDB habilita autenticación por defecto. Puedes pasar `SURREAL_USER` / `SURREAL_PASS` para
bootstrap, pero esas credenciales solo se usan para crear usuarios root cuando no existe ninguno.

Si ya existen usuarios root, SurrealDB ignora esas credenciales de bootstrap y lo reporta en logs.

### Errores comunes de arranque

- `command: surreal start ...` provoca restart loops con esta imagen.
- `command: sh -c ...` falla porque la imagen no trae `/bin/sh`.
- Usa `surreal isready` en healthcheck (no `is-ready`).

### Reset desde cero (destructivo)

```bash
docker compose down
docker rm -f surrealdb 2>/dev/null || true
rm -rf /surreal-data/surrealdb-data/*
docker compose up -d
```

### Rotar password root

```surql
DEFINE USER OVERWRITE root ON ROOT PASSWORD 'new-strong-password' ROLES OWNER;
```

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

- [Running SurrealDB with Docker](https://surrealdb.com/docs/surrealdb/installation/running/docker) -
  Guía oficial de instalación con Docker
- [SurrealQL](https://surrealdb.com/docs/surrealql) - Lenguaje de consulta de SurrealDB
- [Graph Database en SurrealDB](https://surrealdb.com/docs/surrealdb) - Funcionalidades de grafos nativas

### Repositorios

- [SurrealDB Docker GitHub](https://github.com/surrealdb/docker.surrealdb.com) - Configuraciones
  Docker oficiales
- [SurrealDB GitHub](https://github.com/surrealdb/surrealdb) - Repositorio principal

### Aprendizaje

- [SurrealDB Fundamentals Course](https://surrealdb.com/learn/fundamentals) - Curso oficial gratuito
- [Aeon's Surreal Renaissance](https://surrealdb.com/learn/book) - Libro avanzado sobre SurrealDB
