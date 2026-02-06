# Dependency cache stage - only rebuilds when Cargo.toml/Cargo.lock change
FROM rust:1.85-bookworm AS deps

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy only dependency manifests first
COPY services/data-retrieval/Cargo.toml /app/services/data-retrieval/Cargo.toml
COPY services/control-plane/Cargo.toml /app/services/control-plane/Cargo.toml
COPY services/control-plane/Cargo.lock /app/services/control-plane/Cargo.lock

# Create stub source files so cargo can resolve and build dependencies
RUN mkdir -p /app/services/data-retrieval/src && \
    echo "pub fn stub() {}" > /app/services/data-retrieval/src/lib.rs && \
    mkdir -p /app/services/control-plane/src && \
    echo "fn main() {}" > /app/services/control-plane/src/main.rs && \
    echo "pub fn stub() {}" > /app/services/control-plane/src/lib.rs

# Create stub migrations dir (needed for sqlx::migrate! macro)
RUN mkdir -p /app/services/control-plane/migrations && \
    touch /app/services/control-plane/migrations/.keep

WORKDIR /app/services/control-plane

# Build dependencies only - this layer is cached when only source changes
RUN cargo build --release 2>/dev/null || true

# Build stage - uses cached dependencies
FROM deps AS builder

# Copy real source (invalidates this layer on code changes, but deps are cached)
COPY services/data-retrieval /app/services/data-retrieval
COPY services/control-plane /app/services/control-plane

WORKDIR /app/services/control-plane

# Touch source files to force rebuild of local crates (not external dependencies)
RUN touch src/main.rs src/lib.rs ../data-retrieval/src/lib.rs

# Fix cedros-login v0.0.4 bug: duplicate migration version 20260130000001
# (encrypted_settings + treasury_config). treasury_config also references
# non-existent "orgs" table. Remove it so sqlx::migrate! only sees one file.
RUN find /usr/local/cargo/registry/src -path "*/cedros-login-server-*/migrations/20260130000001_treasury_config.sql" -delete 2>/dev/null || true

# Build release binary - only recompiles our crate, deps are cached
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/services/control-plane/target/release/control-plane /app/control-plane

# Copy migrations
COPY --from=builder /app/services/control-plane/migrations /app/migrations

# Create non-root user
RUN useradd -r -s /bin/false appuser
USER appuser

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/v1/healthz || exit 1

CMD ["./control-plane"]
