# syntax=docker/dockerfile:1

FROM rust:1.95-slim-trixie AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang=1:19.0-63 \
    libclang-dev=1:19.0-63 \
    pkg-config=1.8.1-4 \
    && rm -rf /var/lib/apt/lists/*

# Build from repo context so path dependencies continue to resolve.
COPY . .
RUN cargo build --manifest-path clients/rook/Cargo.toml --release --locked --bin rook && \
    strip clients/rook/target/release/rook

FROM busybox:1.36.1 AS prep

RUN mkdir -p /rook-data

RUN chown -R 65532:65532 /rook-data

FROM gcr.io/distroless/cc-debian13:nonroot AS release

COPY --from=builder /app/clients/rook/target/release/rook /usr/local/bin/rook
COPY --from=prep --chown=65532:65532 /rook-data /rook-data

ENV HOME=/rook-data

WORKDIR /rook-data
USER 65532:65532
EXPOSE 4141

# Container images intentionally bind to 0.0.0.0 so the service is reachable when the operator
# explicitly publishes the container port. This does not change Rook's product-level local-first
# default outside Docker.
ENTRYPOINT ["/usr/local/bin/rook"]
CMD ["serve", "--host", "0.0.0.0", "--port", "4141", "--db-path", "/rook-data/rook.db"]
