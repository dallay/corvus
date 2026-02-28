---
title: SurrealDB in Production with Docker Compose
---

Here are the steps to deploy **SurrealDB in production with Docker Compose** based on the official
SurrealDB documentation and standard containerization practices.

## 1) Storage Types in SurrealDB

SurrealDB supports several storage engines, each with different characteristics:

| Engine | Docker Command | Persistence | Use Cases |
|--------|----------------|-------------|-----------|
| **In-Memory** (`mem://`) | Not specified | ❌ No | Tests, cache, temporary data |
| **RocksDB** (`rocksdb://`) | `rocksdb:/surreal/db` | ✅ Yes | Development, single-node production |
| **SurrealKV** (`surrealkv://`) | `surrealkv:/surreal/db` | ✅ Yes | Production (modern RocksDB replacement) |
| **TiKV** | Cluster config | ✅ Yes | High availability, distributed clustering |

### Details of Each Type

#### In-Memory (`mem://`)
- Stores all data in RAM
- **Fastest possible** performance
- Data is **lost** when connection closes
- Ideal for: unit tests, cache, rapid development

#### RocksDB (`rocksdb://`)
- Key-value based storage engine
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

One of the most powerful features of SurrealDB is that **it supports graphs natively and implicitly**. No additional configuration needed - it's part of the database engine.

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

| Feature | Record Links | Graph Relations |
|---------|-------------|-----------------|
| **Direction** | Unidirectional | Bidirectional |
| **Metadata** | ❌ No | ✅ Yes (you can store data in the relation) |
| **Performance** | Faster | Flexible |
| **Use Case** | Simple references | Complex relations with context |

**Use Record Links when:**
- You only need to reference one record from another
- Performance is critical
- You don't need metadata in the relation

**Use Graph Relations when:**
- You need bidirectional relationships
- You want to store information about the relation (e.g., "creation date", "weight")
- You're going to do complex traversal queries

> 📖 **More Info**: See [Graph Database in SurrealDB](https://surrealdb.com/docs) for advanced examples.

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

### With RocksDB (standard)

```yaml
version: "3.9"

services:
  surrealdb:
    image: surrealdb/surrealdb:2.6.2         # pin a specific version
    container_name: surrealdb
    command: >
      start
      --bind 0.0.0.0:8000                   # listen on all interfaces
      --user ${SURREAL_USER}                # root/non-root user
      --pass ${SURREAL_PASS}                # secret password
      rocksdb:/surreal/db                    # RocksDB persistence on volume
    env_file: 
      - .env                                 # define sensitive variables
    ports:
      - "${SURREAL_PORT:-8000}:8000"         # expose only if needed
    volumes:
      - surreal_data:/surreal/db             # persistent volume
    restart: unless-stopped                  # prod restart policy
    logging:
      driver: "json-file"
      options:
        max-size: "50m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "wget", "--spider", "-q", "http://localhost:8000/ready"]
      interval: 30s
      timeout: 10s
      retries: 5

volumes:
  surreal_data:
```

### With SurrealKV (recommended for production)

```yaml
version: "3.9"

services:
  surrealdb:
    image: surrealdb/surrealdb:2.6.2
    container_name: surrealdb
    command: >
      start
      --bind 0.0.0.0:8000
      --user ${SURREAL_USER}
      --pass ${SURREAL_PASS}
      surrealkv:/surreal/db                  # SurrealKV (recommended)
    # ... rest of configuration same
```

### In-Memory (development/tests only)

```yaml
version: "3.9"

services:
  surrealdb:
    image: surrealdb/surrealdb:2.6.2
    container_name: surrealdb
    command: >
      start
      --bind 0.0.0.0:8000
      --user ${SURREAL_USER}
      --pass ${SURREAL_PASS}
      mem                                        # In-memory (NOT persistent)
    # NO volumes for in-memory
    # WARNING: data is lost on restart
```

## 4) Environment Variables (`.env` file)

This file **must NOT be committed to public repository**:

```bash
SURREAL_USER=root
SURREAL_PASS=S0m3$3cur3P@ss
SURREAL_PORT=8000
```

## 5) Key Technical Notes

### Data Persistence

The command `start ... rocksdb:/surreal/db` (or `surrealkv:/surreal/db`) makes SurrealDB use a
disk-based storage engine. **Without it, it stays in-memory** (non-persistent):

- `rocksdb:/surreal/db` → Persistent with RocksDB
- `surrealkv:/surreal/db` → Persistent with SurrealKV (recommended)
- `mem` → RAM only (lost when closed)

### Credentials and Authentication

SurrealDB enables authentication by default. You must supply `--user` and `--pass` or use
environment variables to initialize root.

### Resilience

Include a healthcheck and restart policy to ensure continuous operation if the container fails.
This is standard in production.

### Logs

The logging configuration limits size and rotation. Adjust based on observability requirements.

## 6) Security and Networking Considerations

- In production, it is recommended to place SurrealDB **behind a reverse proxy** (NGINX/Traefik) with
  **TLS termination**.
- Use private Docker networks if running other services (e.g., backend) to avoid exposing the
  port publicly.
- **Do NOT use `latest` in production**; pinning the version enforces release control.

## 7) Enterprise Environment Options

If you need to autoscale or have multiple nodes:

- Replicate the configuration with **clustering** (SurrealDB supports multi-node, but not with this
  simple Compose; check the official SurrealDB repository for clusters).
- Add **automated backup** of volumes.
- Integration with metrics systems (**Prometheus**, ELK).

## Resources

### Official Documentation

- [Running SurrealDB with Docker](https://surrealdb.com/docs/surrealdb/installation/running/docker) - Official Docker installation guide
- [SurrealQL](https://surrealdb.com/docs/surrealql) - SurrealDB query language
- [Graph Database in SurrealDB](https://surrealdb.com/docs) - Native graph features

### Repositories

- [SurrealDB Docker GitHub](https://github.com/surrealdb/docker.surrealdb.com) - Official Docker configurations
- [SurrealDB GitHub](https://github.com/surrealdb/surrealdb) - Main repository

### Learning

- [SurrealDB Fundamentals Course](https://surrealdb.com/learn/fundamentals) - Free official course
- [Aeon's Surreal Renaissance](https://surrealdb.com/learn/book) - Advanced book on SurrealDB
