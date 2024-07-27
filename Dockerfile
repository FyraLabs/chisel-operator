FROM rust:latest as build-env
RUN apt update
RUN apt install -y libssl-dev
WORKDIR /app
COPY . /app
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse
RUN wget https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
    && tar xzf sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
    && mv sccache-v0.2.15-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
    && chmod +x /usr/local/bin/sccache
ENV RUSTC_WRAPPER=/usr/local/bin/sccache
ENV SCCACHE_DIR=/root/.cache/sccache
# copy build artifact somewhere accessible so we can copy it in the next stage
RUN --mount=type=cache,target=/root/.cargo \
 --mount=type=cache,target=/root/.cache/sccache \
 cargo build --release

FROM redhat/ubi9-micro:latest
# RUN useradd -u 1001 chisel
USER 1001
COPY --from=build-env --chown=chisel /app/target/release/chisel-operator /usr/bin/chisel-operator
CMD ["chisel-operator"]
