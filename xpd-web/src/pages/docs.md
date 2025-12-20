---
title: "Documentation"
layout: "../layouts/Docs.astro"
description: "Learn how to configure experienced"
---

## Config

The entrypoint of most configuration is the `/config` command. It has subcommands, `rewards` and `levels`, for
configuring level-up behavior and role-reward assignment behavior. Values cannot yet be cleared once set, so you must
reset your settings if you wish to disable a setting. This will be improved soon.

### Leveling Configuration

The variables available in level up messages are:

- `level`: The user's level after leveling up.
- `old_level`: The user's level prior to leveling up.
- `xp`: The numeric value of the user's total XP.
- `old_xp`: The numeric value of the user's total XP prior to leveling up.
- `user_mention`: @mention ping for the user who leveled up.
- `user_username`: The @username of the user who leveled up.
- `user_display_name`: The Discord global display name of the user who leveled up. Defaults to `user_username`.
- `user_nickname`: The current guild nickname of the user who leveled up, or their display name if no nick exists.
- `user_id`: The ID of the user who leveled up.

You can use the variables by surounding their names in curly brackets, like so:
`{user_mention} has leveled up to level {level}!`.
The level-up channel may only be enabled if the level-up message is set.

Newlines can be added to the level up message by using the escape code `\n`.

### Rewards Configuration

The boolean `one_at_a_time` determines if a user is given all the reward roles they have earned, or only the highest
one.

## Management

There are three main entrypoints for managing bot behavior.

- `/xp`, which allows you to manipulate users' XP counts in your server
- `/rewards`, which allows you to configure leveling rewards in your server.
- `/manage`, with these subcommands:
  - `/reset-guild`: This deletes all the leveling data & audit logs associated with your server.
    It doesn't delete configuration settings, or role rewards.
  - `/export`: Exports this server's leveling data into a JSON format supported by the `import` command.
  - `/import`: Imports a leveling JSON file exported by scrape6.py, the `export` command, or any other method you wish.

### Experience

The `/xp` command has six subcommands. They all manipulate the XP of the users in your server.

- `add`: Simple enough. Gives a user more XP. Events that trigger on level-up will not trigger until they next send a message (or in some cases, the next time they organically level up).
- `remove`: Same as add, but with a negative sign on the front.
- `set`: This will set a user's experience value to _exactly_ the value you specify. It shares the same non-triggering caveats as `add`.
- `reset`: This allows you to quickly reset a user's XP in your server to 0.

## XP import & export format

The JSON format used by `/manage import` and `/manage export` is a list of structs, with the below
definition:

| Key  |  Value   | Description                 |
| :--: | :------: | --------------------------- |
| `id` | `string` | Stringified discord user ID |
| `xp` |  `int`   | XP count for this user      |

## Rewards

The `/rewards` command has three subcommands: `add`, `list`, and `remove`.

- `add`: Adds a role that will be given when you reach a specified level.
- `remove`: Removes a role reward. You only need to specify either the level or the target role.
- `list`: List currently active rewards

## Audit

The `audit` command allows you to take an audit log of all manual XP modification actions except imports and resets.
The audit log will be cleared by `/manage reset-guild`. The user's audit log can also be cleared if the user uses the `/gdpr delete` command, or if the user is banned. However, these three events always reset the user to 0 XP.

The audit command has two options:

- `moderator`: Filters to return only audit logs in your server where a specific _moderator_ modified someone's XP
- `user`: Filters to return only audit logs in your server where a specific _user_ had their XP modified

These filters can be combined. If you set both of them, only actions taken by that moderator against that user
will be returned.

## XP resetting

XP is automatically reset when a user is banned or kicked from your server, if the "View Audit Log" permission
is enabled for the bot in the server. This is logged to experienced's audit log.
