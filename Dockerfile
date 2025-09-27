FROM docker.io/library/rust:1.89.0-slim-trixie AS build-env

RUN apt-get update
RUN apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev clang mold git

WORKDIR /app
COPY . /app

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release

FROM docker.io/library/debian:trixie-slim

RUN groupadd -r chisel && useradd -r -g chisel -d /chisel -s /usr/sbin/nologin chisel
USER chisel

COPY --from=build-env --chown=chisel:chisel /app/target/release/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]
