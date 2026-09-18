FROM rust:1.96-bookworm

WORKDIR /workspace

ENV CARGO_HOME=/usr/local/cargo

EXPOSE 4001

CMD ["cargo", "run", "-p", "collector", "--", "serve", "--config", "/workspace/protocol/http/config/collector.example.toml", "--host", "0.0.0.0", "--port", "4001"]
