FROM rust:alpine AS builder

WORKDIR /build

RUN apk add pkgconf musl-dev openssl-dev

COPY . .

ARG database_url
ENV DATABASE_URL=${database_url}

RUN cargo build --release

FROM alpine

COPY --from=builder /build/target/release/anvil-slash /usr/bin/

ARG database_url
ENV DATABASE_URL=${database_url}

EXPOSE 8080

ENTRYPOINT "/usr/bin/anvil-slash"
