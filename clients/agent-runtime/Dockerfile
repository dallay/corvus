# syntax=docker/dockerfile:1

# ── Stage 1: Build ────────────────────────────────────────────
FROM rust:1.96-slim-trixie AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    clang \
    libclang-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# 1. Copy manifests to cache dependencies
COPY clients/agent-runtime/Cargo.toml clients/agent-runtime/Cargo.lock ./
COPY clients/agent-runtime/crates/robot-kit/Cargo.toml ./crates/robot-kit/Cargo.toml
COPY clients/cerebro/Cargo.toml /clients/cerebro/Cargo.toml

# Create dummy sources to build dependency graph for workspace members
RUN mkdir -p src crates/robot-kit/src benches /clients/cerebro/src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn _dummy() {}" > crates/robot-kit/src/lib.rs && \
    echo "pub fn _dummy() {}" > /clients/cerebro/src/lib.rs && \
    touch benches/agent_benchmarks.rs
RUN cargo build --release --locked
RUN rm -rf src crates/robot-kit/src benches

# 2. Copy source code
COPY clients/agent-runtime/ .
COPY clients/cerebro /clients/cerebro
# Touch main.rs to force rebuild
RUN touch src/main.rs
RUN cargo build --release --locked && \
    strip target/release/corvus

# ── Stage 2: Permissions & Config Prep ───────────────────────
FROM busybox:latest AS permissions
# Create directory structure (simplified workspace path)
RUN mkdir -p /corvus-data/.corvus /corvus-data/workspace

# Create minimal config for PRODUCTION (allows binding to public interfaces)
# NOTE: Provider configuration must be done via environment variables at runtime
RUN cat > /corvus-data/.corvus/config.toml << 'EOF'
api_key = ""
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4-20250514"
default_temperature = 0.7

[gateway]
port = 3000
host = "0.0.0.0"
EOF

RUN chown -R 65534:65534 /corvus-data

# ── Stage 3: Development Runtime (Debian) ────────────────────
FROM debian:trixie-slim AS dev

# Install runtime dependencies + basic debug tools
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    git \
    iputils-ping \
    vim \
    && rm -rf /var/lib/apt/lists/*

COPY --from=permissions --chown=65534:65534 /corvus-data /corvus-data
COPY --from=builder /app/target/release/corvus /usr/local/bin/corvus

# Overwrite minimal config with DEV template (Ollama defaults)
COPY --chown=65534:65534 dev/config.template.toml /corvus-data/.corvus/config.toml

# Environment setup
# Use consistent workspace path
ENV CORVUS_WORKSPACE=/corvus-data/workspace
ENV HOME=/corvus-data
# Defaults for local dev (Ollama) - matches config.template.toml
ENV PROVIDER="ollama"
ENV CORVUS_MODEL="llama3.2"
ENV CORVUS_GATEWAY_PORT=3000

# Note: API_KEY is intentionally NOT set here to avoid confusion.
# It is set in config.toml as the Ollama URL.

WORKDIR /corvus-data
USER 65534:65534
EXPOSE 3000
ENTRYPOINT ["corvus"]
CMD ["gateway", "--port", "3000", "--host", "0.0.0.0"]

# ── Stage 4: Production Runtime (Distroless) ─────────────────
FROM gcr.io/distroless/cc-debian13:nonroot AS release

COPY --from=builder /app/target/release/corvus /usr/local/bin/corvus
COPY --from=permissions --chown=65534:65534 /corvus-data /corvus-data

# Environment setup
ENV CORVUS_WORKSPACE=/corvus-data/workspace
ENV HOME=/corvus-data
# Defaults for prod (OpenRouter)
ENV PROVIDER="openrouter"
ENV CORVUS_MODEL="anthropic/claude-sonnet-4-20250514"
ENV CORVUS_GATEWAY_PORT=3000

# API_KEY must be provided at runtime!

WORKDIR /corvus-data
USER 65534:65534
EXPOSE 3000
ENTRYPOINT ["corvus"]
CMD ["gateway", "--port", "3000", "--host", "0.0.0.0"]
