# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
# Note: control-plane depends on data-retrieval, so we need both
COPY services/data-retrieval /app/services/data-retrieval
COPY services/control-plane /app/services/control-plane

WORKDIR /app/services/control-plane

# Build release binary
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

# Expose default port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/v1/healthz || exit 1

CMD ["./control-plane"]
