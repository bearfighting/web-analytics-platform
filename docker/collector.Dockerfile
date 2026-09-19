FROM rust:1.96-bookworm

WORKDIR /workspace

ENV CARGO_HOME=/usr/local/cargo

RUN apt-get update \
  && apt-get install --no-install-recommends --yes curl \
  && rm -rf /var/lib/apt/lists/*

# Cache the Rust registry and git dependencies in the image. Source code is
# mounted by Compose for development, so the target directory remains an
# incremental named volume at runtime.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY services/analytics-api/Cargo.toml services/analytics-api/Cargo.toml
COPY services/collector/Cargo.toml services/collector/Cargo.toml
COPY services/processor/Cargo.toml services/processor/Cargo.toml
RUN mkdir -p services/analytics-api/src services/collector/src services/processor/src \
  && printf 'fn main() {}\n' > services/analytics-api/src/main.rs \
  && printf 'fn main() {}\n' > services/collector/src/main.rs \
  && printf 'fn main() {}\n' > services/processor/src/main.rs \
  && cargo fetch --locked

EXPOSE 4001

HEALTHCHECK --interval=2s --timeout=2s --start-period=5s --retries=15 \
  CMD curl --fail --silent http://127.0.0.1:4001/health || exit 1

CMD ["cargo", "run", "-p", "collector", "--", "serve", "--config", "/workspace/protocol/http/config/collector.example.toml", "--host", "0.0.0.0", "--port", "4001"]
