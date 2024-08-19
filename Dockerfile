FROM rust:1.78.0-alpine3.20 as builder

WORKDIR /build

RUN apk add musl-dev libressl-dev

COPY Cargo.toml Cargo.lock ./
COPY src src

RUN cargo build --release

FROM alpine:3.20.0

WORKDIR /app

COPY --from=builder /build/target/release/sculptor /app/sculptor

VOLUME [ "/app/avatars" ]
VOLUME [ "/app/logs" ]
EXPOSE 6665/tcp

CMD ["./sculptor"]