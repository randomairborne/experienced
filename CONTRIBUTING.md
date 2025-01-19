Thank you for being willing to contribute to Experienced.

Before you PR, make sure to run the `./ci/lint.sh` script and make sure it reports no errors.
Before committing, you should run `npm run prettier` and `cargo +nightly fmt`.

Experienced moves fast. Sometimes changes in your PR might get broken- I'm willing to fix them if you like. Just let me
know.

The website in xpd-web is an Astro site that does a static build. Experienced itself is made up of three bin
crates,
`xpd-cleanup`, `xpd-gateway`, and `xpd-setcommands`. Cleanup runs as a cronjob, gateway runs as a daemon, and
setcommands runs as a start job.
