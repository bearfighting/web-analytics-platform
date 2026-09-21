FROM rust:1.96-bookworm AS builder

WORKDIR /workspace

ENV CARGO_HOME=/usr/local/cargo

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY services/analytics-api/Cargo.toml services/analytics-api/Cargo.toml
COPY services/collector/Cargo.toml services/collector/Cargo.toml
COPY services/processor/Cargo.toml services/processor/Cargo.toml
COPY tools/db-migrator/Cargo.toml tools/db-migrator/Cargo.toml
RUN mkdir -p services/analytics-api/src services/collector/src services/processor/src tools/db-migrator/src \
  && printf 'fn main() {}\n' > services/analytics-api/src/main.rs \
  && printf 'fn main() {}\n' > services/collector/src/main.rs \
  && printf 'fn main() {}\n' > services/processor/src/main.rs \
  && printf 'fn main() {}\n' > tools/db-migrator/src/main.rs \
  && cargo fetch --locked

COPY tools/db-migrator/src tools/db-migrator/src
COPY tools/db-migrator/build.rs tools/db-migrator/build.rs
COPY migrations migrations
RUN cargo build --locked --release -p db-migrator

FROM rust:1.96-bookworm

COPY --from=builder /workspace/target/release/db-migrator /usr/local/bin/db-migrator

ENTRYPOINT ["/usr/local/bin/db-migrator"]
