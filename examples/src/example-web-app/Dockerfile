FROM rust:1-bookworm AS builder

RUN apt-get update && \
    apt-get install -y cmake pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY arena/Cargo.toml arena/Cargo.toml
COPY arena-kafka/Cargo.toml arena-kafka/Cargo.toml
COPY arena-postgres/Cargo.toml arena-postgres/Cargo.toml
COPY arena-mssql/Cargo.toml arena-mssql/Cargo.toml
COPY arena-http/Cargo.toml arena-http/Cargo.toml
COPY arena-executable-component/Cargo.toml arena-executable-component/Cargo.toml
COPY arena-container/Cargo.toml arena-container/Cargo.toml
COPY arena-container-component/Cargo.toml arena-container-component/Cargo.toml
COPY arena-ffi/Cargo.toml arena-ffi/Cargo.toml
COPY examples/Cargo.toml examples/Cargo.toml

RUN mkdir -p arena/src && echo "" > arena/src/lib.rs && \
    mkdir -p arena-kafka/src && echo "" > arena-kafka/src/lib.rs && \
    mkdir -p arena-postgres/src && echo "" > arena-postgres/src/lib.rs && \
    mkdir -p arena-mssql/src && echo "" > arena-mssql/src/lib.rs && \
    mkdir -p arena-http/src && echo "" > arena-http/src/lib.rs && \
    mkdir -p arena-executable-component/src && echo "" > arena-executable-component/src/lib.rs && \
    mkdir -p arena-container/src && echo "" > arena-container/src/lib.rs && \
    mkdir -p arena-container-component/src && echo "" > arena-container-component/src/lib.rs && \
    mkdir -p arena-ffi/src && echo "" > arena-ffi/src/lib.rs && \
    mkdir -p examples/src && echo "" > examples/src/lib.rs && \
    mkdir -p examples/src/example-web-app && echo "fn main() {}" > examples/src/example-web-app/main.rs && \
    mkdir -p examples/src/example-arena-console && echo "fn main() {}" > examples/src/example-arena-console/main.rs

RUN cargo build --release --bin web-app -p arena-examples 2>/dev/null || true

COPY arena/ arena/
COPY arena-kafka/ arena-kafka/
COPY arena-postgres/ arena-postgres/
COPY arena-mssql/ arena-mssql/
COPY arena-http/ arena-http/
COPY arena-executable-component/ arena-executable-component/
COPY arena-container/ arena-container/
COPY arena-container-component/ arena-container-component/
COPY arena-ffi/ arena-ffi/
COPY examples/ examples/

RUN rm -rf target/release/.fingerprint/arena-* \
           target/release/.fingerprint/arena_* \
           target/release/deps/arena* \
           target/release/deps/libarena* \
           target/release/web-app \
           target/release/libarena* && \
    cargo build --release --bin web-app -p arena-examples

# --- Runtime ---
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/web-app /usr/local/bin/web-app

EXPOSE 3000

ENTRYPOINT ["web-app"]