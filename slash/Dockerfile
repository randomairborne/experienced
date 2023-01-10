FROM rust:alpine AS builder

WORKDIR /build

RUN apk add musl-dev

COPY . .

ENV SQLX_OFFLINE=true

RUN cargo build --release

FROM alpine

COPY --from=builder /build/target/release/experienced-slash /usr/bin/

EXPOSE 8080

ENTRYPOINT "/usr/bin/experienced-slash"
