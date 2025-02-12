# Experienced

A powerful, flexible discord leveling bot

## How to Self-Host Experienced

The easiest way to self-host experienced is on Debian or Ubuntu Linux. Thus, this is how this tutorial will set up
Experienced.

If you'd rather just have a hosted bot, that's fine! I really appreciate
it. [Click here to invite the official instance.](https://discord.com/api/oauth2/authorize?client_id=1035970092284002384&permissions=0&scope=bot%20applications.commands)

While you are legally within your rights to do so, please do not self-host public instances of Experienced.

### Preparing your server

To run experienced, you need [docker](https://docs.docker.com/engine/install/)
or [podman](https://podman.io/docs/installation). We'll use docker for this tutorial.

## Using Docker Compose

You can grab the file from [here](/docker-compose.yml)

## Env File:

You can grab the variables from [here](/.env.example)

Make sure you replace `<token>` and `<db_pass>` with your own bot token and database password for postgres.

## Finally, start the bot with:

```bash
docker compose up
```

If there are any errors, ping valkyrie_pilot on the official discord [here](https://valk.sh/discord)

## Invite the bot with:

`https://discord.com/oauth2/authorize?client_id=<yourclientid>&permissions=414733126656&scope=bot+applications.commands`

Make sure to replace the `<yourclientid>` with your bots.
