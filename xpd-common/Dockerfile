FROM rust:alpine AS builder

RUN apk add musl-dev git

WORKDIR /build

COPY . .

ENV SQLX_OFFLINE=1
RUN cargo build --release --package xpd-common

FROM alpine:latest

COPY --from=builder /build/target/release/print-versions /usr/bin/print-versions

ENTRYPOINT [ "/usr/bin/print-versions" ]