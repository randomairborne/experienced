FROM rust:alpine

FROM alpine

COPY /target/x86_64-unknown-linux-musl/release/experienced-listener /usr/bin/

ENTRYPOINT "/usr/bin/experienced-listener"
