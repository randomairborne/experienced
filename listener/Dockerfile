FROM rust:alpine AS builder

WORKDIR /build

RUN apk add musl-dev

COPY . .

ARG database_url
ENV DATABASE_URL=${database_url}

RUN cargo build --release

FROM alpine

COPY --from=builder /build/target/release/experienced-listener /usr/bin/

ENTRYPOINT "/usr/bin/experienced-listener"
