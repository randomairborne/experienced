FROM alpine

COPY /executables/xpd-web /usr/bin/
COPY /xpd-web/templates/ /xpd-web/templates/
COPY /xpd-web/static/ /xpd-web/static/

EXPOSE 8080

WORKDIR "/xpd-web/"

ENTRYPOINT "/usr/bin/xpd-web"
