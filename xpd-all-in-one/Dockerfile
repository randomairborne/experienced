ARG TAG=latest
FROM ghcr.io/randomairborne/experienced-gateway:${TAG} AS gateway

ARG TAG=latest
FROM ghcr.io/randomairborne/experienced-setcommands:${TAG} AS setcommands

FROM alpine:latest

WORKDIR /var/experienced

COPY --from=gateway /usr/bin/xpd-gateway /usr/bin/xpd-gateway
COPY --from=setcommands /usr/bin/xpd-setcommands /usr/bin/xpd-setcommands
COPY xpd-card-resources xpd-card-resources
COPY xpd-all-in-one/all-in-one.sh /usr/bin/xpd-all-in-one.sh

CMD [ "/usr/bin/xpd-all-in-one.sh" ]