FROM rust:1.92.0 AS builder

WORKDIR /usr/src/app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        protobuf-compiler \
        pkg-config \
        libssl-dev \
        git \
        clang \
        llvm-dev \
        libclang-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto ./proto
COPY src ./src

RUN cargo fetch

RUN cargo build --release


FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
        netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --uid 10001 --create-home --home-dir /nonexistent --shell /usr/sbin/nologin graveyard \
    && mkdir -p /data /tmp \
    && chown -R graveyard:graveyard /data /tmp

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/target/release/graveyar_db /usr/local/bin/graveyar_db

ENV SCYLLA_URI=scylla:9042
ENV SCYLLA_KEYSPACE=graveyard
ENV DB_PATH=/data/rocksdb
ENV RUST_LOG=info

EXPOSE 50051

USER graveyard

CMD ["/usr/local/bin/graveyar_db"]
