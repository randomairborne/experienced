# How to Self-Host Experienced

The easisest way to self-host experienced is with Docker Compose on Linux. Thus, this is how this tutorial will set up Experienced. \

If you'd rather just have a hosted bot, that's fine! [Click here to invite it.](https://discord.com/api/oauth2/authorize?client_id=1035970092284002384&permissions=0&scope=bot%20applications.commands) \

## Creating a Discord application

First, we need to create a Discord application. Go to [https://discord.com/developers/applications](https://discord.com/developers/applications) and click the `New Application` button in the top right corner.
Give it a nice name, then click continue. Now we need to create a .env file, which should look like this:

```dotenv
DISCORD_TOKEN=
DISCORD_PUBKEY=
```

Then, fill `DISCORD_PUBKEY` with the Public Key shown on the main page.
Go to the `Bot` tab and click `Add Bot`. This will also show you a `Reset Token` button. Clicking this should reveal and copy your bot token,
which should then be filled into the `DISCORD_TOKEN`. Then, customize your bot to your heart's content. No gateway intents are needed. While you are legally
within your rights to do so, please do not self-host public instances of Experienced.

## Preparing your server

To run experienced, you need [docker](https://docs.docker.com/engine/install/) with the [compose extension](https://docs.docker.com/compose/install/linux/).
You also need to have a proxying webserver. I use Nginx and Cloudflare, but any proxying web server will do.

Create a web server configuration to proxy port `443` with TLS to localhost port `5389`. You **must** enable SSL support, which you can do with a certificate tool like [certbot](https://certbot.eff.org/), a web server like [caddy](https://caddyserver.com/),
or a CDN proxy like [cloudflare](https://cloudflare.com/).
If you don't have a domain to use, you can purchase one for a few dollars a year from [porkbun](https://porkbun.com/) or from Cloudflare. Domains are great to have, and you can have anything from a special custom invite to a webpage about your server.

## Starting the bot

Now we need to get the docker-compose file for experienced. You can download it with cURL.

```bash
curl -o compose.yaml https://raw.githubusercontent.com/randomairborne/experienced/main/compose.yaml
```

Put this in the same folder as your `.env` file. Then, you can run the bot with

```bash
docker compose up -d
```

You can stop the bot with

```bash
docker compose down
```

Every once in a while, update the bot with

```bash
docker compose pull && docker compose down && docker compose up -d
```
