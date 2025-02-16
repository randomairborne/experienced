FROM rust:alpine AS builder

RUN apk add musl-dev git mold

WORKDIR /build

COPY . .

ENV SQLX_OFFLINE=1
RUN cargo build --release --package xpd-setcommands

FROM alpine:latest

COPY --from=builder /build/target/release/xpd-setcommands /usr/bin/xpd-setcommands

ENTRYPOINT [ "/usr/bin/xpd-setcommands" ]