ARG LLVMTARGETARCH
FROM --platform=${BUILDPLATFORM} ghcr.io/randomairborne/cross-cargo:${LLVMTARGETARCH} AS builder
ARG LLVMTARGETARCH

WORKDIR /build

COPY . .

RUN cargo build --release --target ${LLVMTARGETARCH}-unknown-linux-musl

FROM alpine:latest
ARG LLVMTARGETARCH

ENTRYPOINT [ "/usr/bin/xpd-cleanup" ]