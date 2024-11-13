ARG LLVMTARGETARCH
FROM --platform=${BUILDPLATFORM} ghcr.io/randomairborne/cross-cargo:${LLVMTARGETARCH} AS builder
ARG LLVMTARGETARCH

WORKDIR /build

COPY . .

RUN cargo build --release --target ${LLVMTARGETARCH}-unknown-linux-musl

FROM alpine:latest
ARG LLVMTARGETARCH

WORKDIR /experienced/

COPY --from=builder /build/target/${LLVMTARGETARCH}-unknown-linux-musl/release/xpd-setcommands /usr/bin/xpd-setcommands

ENTRYPOINT [ "/usr/bin/xpd-setcommands" ]