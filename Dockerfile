## Chef
# FROM clux/muslrust:stable AS chef
FROM rust:1.81.0-alpine3.20 as chef
USER root
RUN apk add musl-dev libressl-dev
RUN cargo install cargo-chef
WORKDIR /build

## Planner
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo chef prepare --recipe-path recipe.json

## Builder
FROM chef AS builder 
COPY --from=planner /build/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo build --release --target x86_64-unknown-linux-musl --bin sculptor

## Runtime
FROM alpine:3.20.0 AS runtime
WORKDIR /app
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/sculptor /app/sculptor

RUN apk add --no-cache tzdata
ENV TZ=Etc/UTC

VOLUME [ "/app/data" ]
VOLUME [ "/app/logs" ]
EXPOSE 6665/tcp

ENTRYPOINT [ "./sculptor" ]