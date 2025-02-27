# Structure of experienced

- `.github` holds CI scripts.
- `.sqlx` holds automatically generated SQLx data. You can delete it safely, just regenerate it before
  commit with `cargo sqlx prepare`
- `ci` holds various CI scripts and a pre-commit script.
- `mee6` is a crates.io published crate which handles level calculations
- `migrations` holds the database migrations for experienced. These are not the nicest for setting
  up a new DB!
- `simpleinterpolation` is a crates.io-published crate which is used in... interpolation.
- `xpd-all-in-one` is a shell script and dockerfile for a docker container that automatically
  registers its commands.
- `xpd-card-resources` is a default resource-set for Experienced, which DOES NOT share the same license.
- `xpd-cleanup` is a crate which uses `xpd-database` to clean up unused database rows.
- `xpd-common` is a crate for common data structures and communication data
- `xpd-database` holds a database abstraction for experienced, keeping all SQL in the same place.
  The rationale for this is that lots of queries were reused previously, so this allows them to be reused
  over the entire bot.
- `xpd-gateway` is the core entrypoint for the bot. It handles the event bus and recieving from discord.
- `xpd-listener` handles events from discord that require nontrivial logic (i.e. more than one function call)
- `xpd-rank-card` is an abstraction over resvg to render loaded rank cards from `xpd-card-resources`
- `xpd-setcommands` is a binary that informs discord of the bot's command tree
- `xpd-slash` handles all `INTERACTION_CREATE` events from Discord, and their responses. It also
  sends messages on the event bus, to cause cache clears and similar.
- `xpd-slash-defs` defines experienced's slash command structure with `twilight-interactions`
- `xpd-util` contains general utilities for experienced
- `xpd-web` is NOT a cargo crate, but an [astro](https://astro.build) website that is packed into a
  [tunnelbana](https://tunnelbana.valk.sh) server to be deployed.
