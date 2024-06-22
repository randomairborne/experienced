ARG LLVMTARGETARCH
FROM --platform=${BUILDPLATFORM} ghcr.io/randomairborne/cross-cargo-${LLVMTARGETARCH}:latest AS builder
ARG LLVMTARGETARCH

WORKDIR /build

COPY . .

RUN cargo build --release --target ${LLVMTARGETARCH}-unknown-linux-musl

FROM alpine:latest
ARG LLVMTARGETARCH

WORKDIR /experienced/

COPY --from=builder /build/target/${LLVMTARGETARCH}-unknown-linux-musl/release/xpd-gateway /usr/bin/xpd-gateway
COPY xpd-card-resources xpd-card-resources

ENTRYPOINT [ "/usr/bin/xpd-gateway" ]