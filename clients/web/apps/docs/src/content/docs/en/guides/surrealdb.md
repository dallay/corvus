---
title: SurrealDB in Production with Docker Compose
---

Here are the steps to deploy **SurrealDB in production with Docker Compose** based on the official
SurrealDB documentation and standard containerization practices.

This guide includes practical corrections for SurrealDB v3 Docker behavior observed in real
operations.

## 1) Storage Types in SurrealDB

SurrealDB supports several storage engines, each with different characteristics:

| Engine                         | Docker Command          | Persistence | Use Cases                                 |
|--------------------------------|-------------------------|-------------|-------------------------------------------|
| **In-Memory** (`mem://`)       | Not specified           | ❌ No        | Tests, cache, temporary data              |
| **RocksDB** (`rocksdb://`)     | `rocksdb:/surreal/db`   | ✅ Yes       | Development, single-node production       |
| **SurrealKV** (`surrealkv://`) | `surrealkv:/surreal/db` | ✅ Yes       | Production (modern RocksDB replacement)   |
| **TiKV**                       | Cluster config          | ✅ Yes       | High availability, distributed clustering |

### Details of Each Type

#### In-Memory (`mem://`)

- Stores all data in RAM
- **Fastest possible** performance
- Data is **lost** when connection closes
- Ideal for: unit tests, cache, rapid development

#### RocksDB (`rocksdb://`)

- Key-value-based storage engine
- Persistent on disk
- Suitable for development and single-node production

#### SurrealKV (`surrealkv://`)

- **Recommended** engine by SurrealDB for production
- Modern RocksDB replacement
- Better performance and efficiency

#### TiKV (Clustering)

- For distributed high-availability deployments
- Requires more complex configuration (not included in simple Docker Compose)

---

## 2) Graph Database (Native Feature)

One of the most powerful features of SurrealDB is that **it supports graphs natively and implicitly
**. No additional configuration needed - it's part of the database engine.

### What Does This Mean?

SurrealDB works as a **multi-model database**, meaning you can use:

- **Relational model** (traditional tables)
- **Document model** (embedded JSON)
- **Graph model** (nodes and relationships)

All in the same database, without needing additional tools.

### How to Use Graphs in SurrealDB

#### Create Nodes (Regular Records)

```surql
CREATE person:alice SET name = "Alice", age = 30;
CREATE person:bob SET name = "Bob", age = 25;
CREATE post:1 SET title = "Hello World";
```

#### Create Relationships (Edges)

```surql
-- Alice writes the post
RELATE person:alice->wrote->post:1;

-- Alice follows Bob
RELATE person:alice->follows->person:bob;
```

#### Query Graphs (Arrow Syntax)

```surql
-- Find all posts by Alice
SELECT ->wrote->post FROM person:alice;

-- Find who wrote a post
SELECT <-wrote<-person FROM post:1;

-- Find who Alice follows
SELECT ->follows->person FROM person:alice;
```

### Graph vs Record Links

SurrealDB has two ways to relate records:

| Feature         | Record Links      | Graph Relations                            |
|-----------------|-------------------|--------------------------------------------|
| **Direction**   | Unidirectional    | Bidirectional                              |
| **Metadata**    | ❌ No              | ✅ Yes (you can store data in the relation) |
| **Performance** | Faster            | Flexible                                   |
| **Use Case**    | Simple references | Complex relations with context             |

**Use Record Links when:**

- You only need to reference one record from another
- Performance is critical
- You don't need metadata in the relation

**Use Graph Relations when:**

- You need bidirectional relationships
- You want to store information about the relation (e.g., "creation date", "weight")
- You're going to do complex traversal queries

> 📖 **More Info**: See [Graph Database in SurrealDB](https://surrealdb.com/docs/surrealdb)
> for advanced
> examples.

---

## 3) Design Keys for a Production Stack

You must satisfy these points before running SurrealDB in production:

- **Data persistence** on external volume (e.g., RocksDB/SurrealKV).
- **Secure credentials** configuration via environment variables.
- **Port exposure** controlled by environment (no indiscriminate open mappings).
- **Automatic restart policy** (`restart: unless-stopped` or `always`).
- **Fixed image with version tag**, NOT `latest`.
- **Logging** and, if applicable, TLS.
- **Healthcheck** for service health verification.

## 4) Production `docker-compose.yml` Examples

> Critical for the SurrealDB v3 image:
>
> - The image entrypoint is already `/surreal`.
> - Use `command: start ...` (not `surreal start ...`).
> - `sh -c ...` does not work because `/bin/sh` is not available in the image.

### With SurrealKV (recommended for production)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    env_file:
      - ./secrets/surreal.env                 # chmod 600, never commit
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

### With RocksDB (single-node)

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
    # same ports / volumes / restart / logging / healthcheck settings
```

### In-Memory (development/tests only)

```yaml
services:
  surrealdb:
    image: surrealdb/surrealdb:v3.0.1
    command: >
      start
      --bind 0.0.0.0:8000
      mem
    # NO volumes for in-memory
    # WARNING: data is lost on restart
```

## 5) Environment Variables and Secrets

### For Production: protected env file (Compose)

Create a protected env file (never commit this):

```bash
mkdir -p secrets
cat > secrets/surreal.env << 'EOF'
SURREAL_USER=root
SURREAL_PASS=S0m3$3cur3P@ss
SURREAL_PORT=8000
EOF
chmod 600 secrets/surreal.env
```

Add to `.gitignore`:

```
secrets/
.env
```

### For Development: `.env` file

This file **must NOT be committed to a public repository**:

```bash
# .env - Development only!
SURREAL_USER=root
SURREAL_PASS=S0m3$3cur3P@ss
SURREAL_PORT=8000
```

> If you bootstrap with `--user`/`--pass` in command arguments, remove them after first successful
> start to avoid credential exposure through process inspection.

### SSH tunnel from macOS

```bash
ssh -o ExitOnForwardFailure=yes -fN -L 8000:127.0.0.1:8000 corvus
curl -i http://localhost:8000/status
```

## 6) Key Technical Notes

### Data Persistence

The command `start ... rocksdb:/surreal/db` (or `surrealkv:/surreal/db`) makes SurrealDB use a
disk-based storage engine. **Without it, it stays in-memory** (non-persistent):

- `rocksdb:/surreal/db` → Persistent with RocksDB
- `surrealkv:/surreal/db` → Persistent with SurrealKV (recommended)
- `mem` → RAM only (lost when closed)

### Credentials and Authentication

SurrealDB enables authentication by default. You can provide `SURREAL_USER` / `SURREAL_PASS`
for bootstrap, but they are only used to initialize root users when no root users exist yet.

If root users already exist, SurrealDB ignores bootstrap credentials and logs a warning.

### Common startup pitfalls

- `command: surreal start ...` causes restart loops with this image.
- `command: sh -c ...` fails because `/bin/sh` is not present.
- Use `surreal isready` for healthchecks (not `is-ready`).

### Reset from scratch (destructive)

```bash
docker compose down
docker rm -f surrealdb 2>/dev/null || true
rm -rf /surreal-data/surrealdb-data/*
docker compose up -d
```

### Rotate root password

```surql
DEFINE USER OVERWRITE root ON ROOT PASSWORD 'new-strong-password' ROLES OWNER;
```

### Resilience

Include a healthcheck and restart policy to ensure continuous operation if the container fails.
This is standard in production.

### Logs

The logging configuration limits size and rotation. Adjust based on observability requirements.

## 7) Security and Networking Considerations

- In production, it is recommended to place SurrealDB **behind a reverse proxy** (NGINX/Traefik)
  with
  **TLS termination**.
- Use private Docker networks if running other services (e.g., backend) to avoid exposing the
  port publicly.
- **Do NOT use `latest` in production**; pinning the version enforces release control.

## 8) Enterprise Environment Options

If you need to autoscale or have multiple nodes:

- Replicate the configuration with **clustering** (SurrealDB supports multi-node, but not with this
  simple Compose; check the official SurrealDB repository for clusters).
- Add **automated backup** of volumes.
- Integration with metrics systems (**Prometheus**, ELK).

## Resources

### Official Documentation

- [Running SurrealDB with Docker](https://surrealdb.com/docs/surrealdb/installation/running/docker) -
  Official Docker installation guide
- [SurrealQL](https://surrealdb.com/docs/surrealql) - SurrealDB query language
- [Graph Database in SurrealDB](https://surrealdb.com/docs/surrealdb) - Native graph features

### Repositories

- [SurrealDB Docker GitHub](https://github.com/surrealdb/docker.surrealdb.com) - Official Docker
  configurations
- [SurrealDB GitHub](https://github.com/surrealdb/surrealdb) - Main repository

### Learning

- [SurrealDB Fundamentals Course](https://surrealdb.com/learn/fundamentals) - Free official course
- [Aeon's Surreal Renaissance](https://surrealdb.com/learn/book) - Advanced book on SurrealDB
