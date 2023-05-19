FROM rust:alpine as build-env
RUN apk add --no-cache openssl-dev
WORKDIR /app
COPY . /app
RUN cargo build --release
FROM alpine:latest
RUN apk add --no-cache ca-certificates openssl
COPY --from=build-env /app/target/release/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]