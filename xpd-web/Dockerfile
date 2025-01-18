FROM node:alpine AS builder

COPY ./xpd-web/ /build

WORKDIR /build/

RUN npm install

RUN npm run build

FROM ghcr.io/randomairborne/asset-squisher:latest AS compressor

COPY --from=builder /build/dist/ /build/dist/

RUN asset-squisher --no-compress-images /build/dist/ /build/compressed/

FROM ghcr.io/randomairborne/tunnelbana:latest

COPY --from=compressor /build/compressed/ /var/www/html/

CMD ["tunnelbana", "/var/www/html"]

