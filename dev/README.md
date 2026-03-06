# Corvus Development Environment

A fully containerized development sandbox for Corvus agents. This environment allows you to develop, test, and debug the agent in isolation without modifying your host system.

## Directory Structure

- **`agent/`**: (Merged into clients/agent-runtime/Dockerfile)
  - The development image is built from `clients/agent-runtime/Dockerfile` using the `dev` stage (`target: dev`).
  - Based on `debian:bookworm-slim` (unlike production `distroless`).
  - Includes `bash`, `curl`, and debug tools.
- **`sandbox/`**: Dockerfile for the simulated user environment.
  - Based on `ubuntu:22.04`.
  - Pre-loaded with `git`, `python3`, `nodejs`, `npm`, `gcc`, `make`.
  - Simulates a real developer machine.
- **`docker-compose.yml`**: Defines the services and `dev-net` network.
- **`cli.sh`**: Helper script to manage the lifecycle.

## Usage

Run all commands from the repository root using the helper script:

### 1. Start Environment
```bash
./dev/cli.sh up
```
Builds the agent from source and starts both containers.

To start with the web dashboard too:

```bash
./dev/cli.sh up-dashboard
```

Dashboard URL: <http://127.0.0.1:4324>

### Provider Configuration (Per Developer)
The dev stack is provider-agnostic. Choose provider via shell environment before `./dev/cli.sh up`.

Ollama on host (macOS + Docker Desktop/OrbStack):
```bash
export PROVIDER=ollama
export CORVUS_MODEL=llama3.2
./dev/cli.sh up
```

OpenRouter (or other remote provider):
```bash
export PROVIDER=openrouter
export API_KEY=your_openrouter_api_key
export CORVUS_MODEL=anthropic/claude-sonnet-4-20250514
./dev/cli.sh up
```

### 2. Enter Agent Container (`corvus-dev`)
```bash
./dev/cli.sh agent
```
Use this to run `corvus` CLI commands manually, debug the binary, or check logs internally.
- **Path**: `/corvus-data`
- **User**: `nobody` (65534)

### 3. Enter Sandbox (`sandbox`)
```bash
./dev/cli.sh shell
```
Use this to act as the "user" or "environment" the agent interacts with.
- **Path**: `/home/developer/workspace`
- **User**: `developer` (sudo-enabled)

### 4. Development Cycle
1. Make changes to Rust code in `src/`.
2. Rebuild the agent:
   ```bash
   ./dev/cli.sh build
   ```
3. Test changes inside the container:
   ```bash
   ./dev/cli.sh agent
   # inside container:
   corvus --version
   ```

If you changed dashboard code and want to rebuild only that image:

```bash
./dev/cli.sh build-dashboard
```

Quick smoke checks (gateway + optional dashboard if running):

```bash
./dev/cli.sh smoke
```

### 5. Persistence & Shared Workspace
The local `playground/` directory (in repo root) is mounted as the shared workspace:
- **Agent**: `/corvus-data/workspace`
- **Sandbox**: `/home/developer/workspace`

Files created by the agent are visible to the sandbox user, and vice versa.

The agent configuration lives in `clients/agent-runtime/target/.corvus` (mounted to `/corvus-data/.corvus`), so settings persist across container rebuilds.

### 6. Cleanup

Stop containers and remove volumes and generated config:

```bash
./dev/cli.sh clean
```

**Note:** This removes `clients/agent-runtime/target/.corvus` (config/DB) but leaves the `playground/` directory intact. To fully wipe everything, manually delete `playground/`.
