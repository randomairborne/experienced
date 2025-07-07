ARG TAG=latest
FROM ghcr.io/randomairborne/experienced-gateway:${TAG} AS gateway

ARG TAG=latest
FROM ghcr.io/randomairborne/experienced-setcommands:${TAG} AS setcommands

FROM debian:bookworm-slim

WORKDIR /var/experienced

COPY --from=gateway /usr/local/bin/xpd-gateway /usr/local/bin/xpd-gateway
COPY --from=setcommands /usr/local/bin/xpd-setcommands /usr/local/bin/xpd-setcommands
COPY xpd-card-resources xpd-card-resources
COPY xpd-all-in-one/all-in-one.sh /usr/local/bin/xpd-all-in-one.sh

CMD [ "/usr/local/bin/xpd-all-in-one.sh" ]