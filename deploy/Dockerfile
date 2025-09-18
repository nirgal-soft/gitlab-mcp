# Build stage
FROM rust:1.86 AS builder

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Build application
COPY . .
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -r -s /bin/false -m -d /app appuser

COPY --from=builder /app/target/release/gitlab-mcp /usr/local/bin/mcp-server

# Create config directory
RUN mkdir -p /app/.config

# Switch to app user
USER appuser
WORKDIR /app

ENV RUST_LOG=info
ENV HOME=/app

ENTRYPOINT ["mcp-server"]
