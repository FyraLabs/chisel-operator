FROM rust:latest as build-env
RUN apt update
RUN apt install -y libssl-dev
WORKDIR /app
COPY . /app
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release
# copy build artifact somewhere accessible so we can copy it in the next stage
RUN cp /app/target/release/chisel-operator /app/chisel-operator

FROM rust:latest
RUN apt update
RUN apt install -y ca-certificates libssl
COPY --from=build-env /app/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]