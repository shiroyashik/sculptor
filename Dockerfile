ARG ALPINE_VERSION="3.21"
ARG RUST_VERSION="1.85"
## Chef
FROM --platform=$BUILDPLATFORM rust:${RUST_VERSION}-alpine${ALPINE_VERSION} AS chef
USER root
RUN apk add --no-cache musl-dev libressl-dev zig perl make
RUN cargo install --locked cargo-chef cargo-zigbuild
WORKDIR /build
ENV PKG_CONFIG_SYSROOT_DIR=/

## Planner
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo chef prepare --recipe-path recipe.json

## Builder
FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
# Map Docker's TARGETPLATFORM to Rust's target
# and save the result to a .env file
ARG TARGETPLATFORM
RUN <<EOT
case "${TARGETPLATFORM}" in
    linux/amd64) export CARGO_BUILD_TARGET=x86_64-unknown-linux-musl ;;
    linux/arm64|linux/arm64/v8) export CARGO_BUILD_TARGET=aarch64-unknown-linux-musl ;;
    *) echo "Unsupported target platform: ${TARGETPLATFORM}" >&2; exit 1;;
esac
echo export CARGO_BUILD_TARGET="${CARGO_BUILD_TARGET}" > /tmp/builder.env
rustup target add "${CARGO_BUILD_TARGET}"
EOT
# Build dependencies - this is the caching Docker layer!
RUN . /tmp/builder.env && \
    cargo chef cook --recipe-path recipe.json --release --zigbuild
# Build application
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN . /tmp/builder.env && \
    cargo zigbuild -r --bin sculptor && \
    # Link the right output directory to a well known location for easier access when copying to the runtime image
    ln -s "$PWD/target/$CARGO_BUILD_TARGET/release" /tmp/build-output

## Runtime
FROM alpine:${ALPINE_VERSION} AS runtime
WORKDIR /app
COPY --from=builder /tmp/build-output/sculptor /app/sculptor

RUN apk add --no-cache tzdata
ENV TZ=Etc/UTC

VOLUME [ "/app/data" ]
VOLUME [ "/app/logs" ]
EXPOSE 6665/tcp

ENTRYPOINT [ "./sculptor" ]
