FROM rust:alpine as build-env
RUN apk add --no-cache openssl-dev musl-dev
WORKDIR /app
COPY . /app
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release
FROM alpine:latest
RUN apk add --no-cache ca-certificates openssl
COPY --from=build-env /app/target/release/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]