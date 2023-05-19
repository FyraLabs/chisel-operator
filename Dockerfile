FROM rust:latest as build-env
RUN apt install -y openssl-dev
WORKDIR /app
COPY . /app
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target/release/build \
    --mount=type=cache,target=/app/target/release/deps \
    --mount=type=cache,target=/app/target/release/incremental \
    
    cargo build --release
FROM rust:latest
RUN apk add --no-cache ca-certificates openssl
COPY --from=build-env /app/target/release/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]