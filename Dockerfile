# Build stage (use bookworm for Rust 1.85+ / edition 2024)
FROM rust:bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src ./src

# Verbose build so Docker shows progress (cargo often buffers output otherwise)
ENV CARGO_TERM_COLOR=always
RUN cargo build --release --verbose

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/pye-cli /usr/local/bin/pye-cli

# Entrypoint: write keypair from env to file, then run validator-lockup-manager
RUN echo '#!/bin/sh\n\
set -e\n\
if [ -z "$PAYER_KEYPAIR_JSON" ]; then\n\
  echo "PAYER_KEYPAIR_JSON is not set"; exit 1\n\
fi\n\
echo "$PAYER_KEYPAIR_JSON" > /tmp/payer.json\n\
export PAYER=/tmp/payer.json\n\
exec pye-cli validator-lockup-manager "$@"\n\
' > /entrypoint.sh && chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
