FROM rust:alpine AS builder

WORKDIR /build

RUN apk add pkgconf musl-dev openssl-dev

COPY . .

RUN cargo build --release

FROM alpine

COPY --from=builder /build/target/release/anvil-listener /usr/bin/

ENTRYPOINT "/usr/bin/anvil-listener"
