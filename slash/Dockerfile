FROM rust:alpine AS builder

WORKDIR /build

RUN apk add pkgconf openssl-dev

COPY . .

RUN cargo build --release

FROM alpine

COPY --from builder /build/target/release/anvil-slash /usr/bin/

EXPOSE 8080

ENTRYPOINT "/usr/bin/anvil-slash"
