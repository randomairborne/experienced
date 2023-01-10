FROM rust:alpine AS builder

WORKDIR /build

RUN apk add musl-dev

COPY .. .

ENV SQLX_OFFLINE=true

RUN cargo build --release --target experienced-listener

FROM alpine

COPY --from=builder /build/target/release/experienced-listener /usr/bin/

ENTRYPOINT "/usr/bin/experienced-listener"
