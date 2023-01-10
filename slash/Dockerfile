FROM rust:alpine

FROM alpine

COPY /target/x86_64-unknown-linux-musl/release/experienced-slash /usr/bin/

ENTRYPOINT "/usr/bin/experienced-slash"
