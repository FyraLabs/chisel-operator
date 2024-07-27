FROM rust:latest AS build-env
RUN apt update
RUN apt install -y libssl-dev mold
WORKDIR /app
COPY . /app
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
ENV RUSTFLAGS="-C link-arg=-B/usr/bin/mold"
# copy build artifact somewhere accessible so we can copy it in the next stage
RUN --mount=type=cache,target=/root/.cargo \
    cargo build --release

FROM redhat/ubi9-micro:latest
# RUN useradd -u 1001 chisel
USER 1001
COPY --from=build-env --chown=chisel /app/target/release/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]
