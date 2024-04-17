FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev

RUN \
    --mount=type=cache,target=/build/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --release && cp /build/target/release/xpd-gateway /build/xpd-gateway

FROM alpine:latest

WORKDIR /

COPY --from=builder /build/xpd-gateway /usr/bin/xpd-gateway

EXPOSE 8080

CMD [ "/usr/bin/xpd-gateway" ]