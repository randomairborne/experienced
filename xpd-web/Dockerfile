FROM ghcr.io/randomairborne/asset-squisher AS compressor

COPY /xpd-web/static/ /xpd-web/static/

RUN asset-squisher /xpd-web/static/

FROM alpine

COPY /executables/xpd-web /usr/bin/
COPY /xpd-web/templates/ /var/www/xpd-web-templates/
COPY --from=compressor /xpd-web/static/ /var/www/xpd-web-static/

EXPOSE 8080

ENV ASSET_DIR="/var/www/xpd-web-static/"
ENV TEMPLATE_DIR="/var/www/xpd-web-templates/"

ENTRYPOINT "/usr/bin/xpd-web"
