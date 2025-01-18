FROM rust:alpine AS builder

RUN apk add musl-dev

WORKDIR /build

COPY . .

ENV SQLX_OFFLINE=1
RUN cargo build --release --package xpd-cleanup

FROM alpine:latest

COPY --from=builder /build/target/release/xpd-cleanup /usr/bin/xpd-cleanup

ENTRYPOINT [ "/usr/bin/xpd-cleanup" ]