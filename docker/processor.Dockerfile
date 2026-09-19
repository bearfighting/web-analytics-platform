FROM rust:1.96-bookworm

WORKDIR /workspace

ENV CARGO_HOME=/usr/local/cargo

RUN apt-get update \
  && apt-get install --no-install-recommends --yes curl \
  && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY services/collector/Cargo.toml services/collector/Cargo.toml
COPY services/processor/Cargo.toml services/processor/Cargo.toml
RUN mkdir -p services/collector/src services/processor/src \
  && printf 'fn main() {}\n' > services/collector/src/main.rs \
  && printf 'fn main() {}\n' > services/processor/src/main.rs \
  && cargo fetch --locked

CMD ["cargo", "run", "-p", "processor", "--", "--poll-interval-ms", "1000"]
