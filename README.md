# Experienced

A powerful, flexible discord leveling bot

## How to Self-Host Experienced

The easisest way to self-host experienced is on Debian or Ubuntu Linux. Thus, this is how this tutorial will set up Experienced.

If you'd rather just have a hosted bot, that's fine! I really appreciate it. [Click here to invite the official instance.](https://discord.com/api/oauth2/authorize?client_id=1035970092284002384&permissions=0&scope=bot%20applications.commands)

### Creating a Discord application

First, we need to create a Discord application. Go to [https://discord.com/developers/applications](https://discord.com/developers/applications) and click the `New Application` button in the top right corner.
Give it a nice name, then click continue. Now we need to create a .env file, which should look like this:

```dotenv
DISCORD_TOKEN=
DATABASE_URL=
REDIS_URL=
```

Go to the `Bot` tab. This will show you a `Reset Token` button. Clicking this should reveal and copy your bot token,
which should then be filled into the `DISCORD_TOKEN`. Then, customize your bot to your heart's content. No gateway intents are needed. While you are legally within your rights to do so, please do not self-host public instances of Experienced.

### Preparing your server

To run experienced, you need [docker](https://docs.docker.com/engine/install/) or [podman](https://podman.io/docs/installation). We'll use docker for this tutorial.
You also need postgres and redis. You can get these by running

```bash
sudo apt install redis-server postgresql-15
```

Then, you can create a new user with

```bash
sudo su postgres
psql -U postgres -c "CREATE USER xpd PASSWORD 'xpd'"
psql -U postgres -c "CREATE DATABASE xpd OWNER xpd"
exit
```

Create a file called .env, filling in the contents as nessecary:

```dotenv
DISCORD_TOKEN=
DATABASE_URL=postgres://xpd:xpd@host.docker.internal/xpd
REDIS_URL=redis://host.docker.internal:6379
```

### Starting the bot

Finally, you can actually run the bot!

```bash
docker run ghcr.io/randomairborne/xpd-lite --env-file .env --add-host=host.docker.internal:host-gateway --detach
```

And you're done!
