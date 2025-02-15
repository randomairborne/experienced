FROM rust:alpine AS builder

RUN apk add musl-dev git

WORKDIR /build

COPY . .

ENV SQLX_OFFLINE=1
RUN cargo build --release --package xpd-gateway

FROM alpine:latest

COPY --from=builder /build/target/release/xpd-gateway /usr/bin/xpd-gateway
COPY xpd-card-resources xpd-card-resources

ENTRYPOINT [ "/usr/bin/xpd-gateway" ]