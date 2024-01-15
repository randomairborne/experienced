FROM alpine AS compressor

RUN apk add zstd brotli gzip

COPY /xpd-web/static/ /xpd-web/static/

RUN find /xpd-web/static/ -type f -exec gzip -k9 '{}' \; -exec brotli -k9 '{}' \; -exec zstd -qk19 '{}' \;

FROM alpine

COPY /executables/xpd-web /usr/bin/
COPY /xpd-web/templates/ /var/www/xpd-web-templates/
COPY --from=compressor /xpd-web/static/ /var/www/xpd-web-static/

EXPOSE 8080

ENV ASSET_DIR="/var/www/xpd-web-static/"
ENV TEMPLATE_DIR="/var/www/xpd-web-templates/"

ENTRYPOINT "/usr/bin/xpd-web"
