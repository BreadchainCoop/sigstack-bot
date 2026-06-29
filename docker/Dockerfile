# Build stage
FROM rust:1.88-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Retry slow registry downloads inside Docker
ENV CARGO_NET_RETRY=10 \
    CARGO_HTTP_TIMEOUT=300 \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

# Cache dependencies (omit rust-toolchain.toml — it forces cross-target rustup
# downloads inside the container; use the image's native musl target instead)
COPY Cargo.toml Cargo.lock ./
COPY crates/signal-bot/Cargo.toml crates/signal-bot/
COPY crates/near-ai-client/Cargo.toml crates/near-ai-client/
COPY crates/conversation-store/Cargo.toml crates/conversation-store/
COPY crates/dstack-client/Cargo.toml crates/dstack-client/
COPY crates/signal-client/Cargo.toml crates/signal-client/
COPY crates/signal-registration-proxy/Cargo.toml crates/signal-registration-proxy/
COPY crates/tools/Cargo.toml crates/tools/
COPY crates/whisper-client/Cargo.toml crates/whisper-client/
COPY crates/x402-payments/Cargo.toml crates/x402-payments/

# Minimal stubs so `cargo fetch` can resolve path dependencies
RUN mkdir -p crates/signal-bot/src/commands && \
    echo "pub mod commands; pub mod config; pub mod error; pub mod group_preferences_store; pub mod transcribe_store;" > crates/signal-bot/src/lib.rs && \
    echo "fn main() {}" > crates/signal-bot/src/main.rs && \
    echo "" > crates/signal-bot/src/commands/mod.rs && \
    echo "" > crates/signal-bot/src/config.rs && \
    echo "" > crates/signal-bot/src/error.rs && \
    echo "" > crates/signal-bot/src/group_preferences_store.rs && \
    echo "" > crates/signal-bot/src/transcribe_store.rs && \
    mkdir -p crates/near-ai-client/src && echo "pub fn dummy() {}" > crates/near-ai-client/src/lib.rs && \
    mkdir -p crates/conversation-store/src && echo "pub fn dummy() {}" > crates/conversation-store/src/lib.rs && \
    mkdir -p crates/dstack-client/src && echo "pub fn dummy() {}" > crates/dstack-client/src/lib.rs && \
    mkdir -p crates/signal-client/src && echo "pub fn dummy() {}" > crates/signal-client/src/lib.rs && \
    mkdir -p crates/whisper-client/src && echo "pub fn dummy() {}" > crates/whisper-client/src/lib.rs && \
    mkdir -p crates/x402-payments/src && echo "pub fn dummy() {}" > crates/x402-payments/src/lib.rs && \
    mkdir -p crates/tools/src && echo "pub fn dummy() {}" > crates/tools/src/lib.rs && \
    mkdir -p crates/signal-registration-proxy/src && \
    echo "fn main() {}" > crates/signal-registration-proxy/src/main.rs

# Download all registry crates (cached layer; survives source COPY)
RUN cargo fetch --locked

# Warm compile cache for signal-bot dependency graph (ignore stub compile errors)
RUN cargo build --release -p signal-bot 2>/dev/null || true

# Copy actual source
COPY crates crates

# Touch source files to trigger rebuild
RUN find crates -name "*.rs" -exec touch {} \;

# Build release binary (offline: deps already fetched in cached layer above)
RUN cargo build --release --locked --offline --bin signal-bot

# Runtime stage
FROM alpine:3.21

# Install CA certificates for HTTPS
RUN apk add --no-cache ca-certificates

# Copy binary
COPY --from=builder /app/target/release/signal-bot /signal-bot

# Non-root user + writable data dir for encrypted group preferences
RUN adduser -D -u 1000 appuser && mkdir -p /data && chown appuser:appuser /data
USER appuser

ENTRYPOINT ["/signal-bot"]
