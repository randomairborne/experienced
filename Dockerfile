FROM rust:alpine AS builder

WORKDIR /build
COPY . .

RUN apk add musl-dev

ENV SQLX_OFFLINE=1

RUN \
    --mount=type=cache,target=/build/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --release

RUN --mount=type=cache,target=/build/target/ cp /build/target/release/xpd-gateway /xpd-gateway

FROM alpine:latest

WORKDIR /experienced/

COPY --from=builder /xpd-gateway /usr/bin/xpd-gateway
COPY xpd-card-resources xpd-card-resources

EXPOSE 8080

CMD [ "/usr/bin/xpd-gateway" ]