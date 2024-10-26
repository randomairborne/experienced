---
title: "Documentation"
layout: "../layouts/Docs.astro"
description: "Learn how to configure experienced"
---

## Config

The entrypoint of most configuration is the `/config` command. It has subcommands, `rewards` and `leveling`, for
configuring level-up behavior and role-reward assignment behavior. Values cannot yet be cleared once set, so you must
reset your settings if you wish to disable a setting. This will be improved soon.

### Leveling

The variables available in level up messages are:

- `level` The user's level after leveling up.
- `old_level` The user's level prior to leveling up.
- `xp` The numeric value of the user's total XP.
- `xp` The numeric value of the user's total XP prior to leveling up.
- `user_mention` @mention ping for the user who leveled up.
- `user_username` The @username of the user who leveled up.
- `user_display_name` The Discord global display name of the user who leveled up. Defaults to `user_username`.
- `user_nickname` The current guild nickname of the user who leveled up, or their display name if no nick exists.
- `user_id` The ID of the user who leveled up.

You can use the variables by surounding their names in curly brackets, like so:
`{user_mention} has leveled up to level {level}!`.
The level-up channel may only be enabled if the level-up message is set.

### Rewards

The boolean `one_at_a_time` determines if a user is given all the reward roles they have earned, or only the highest
one.

## Management

The entrypoint of most of the bot-management commands is the `/xp` command. It has two subcommands, `experience`, which
allows you to manipulate users' XP counts in your server, and `rewards`, which allows you to configure leveling rewards
in your server.

### Experience

The `xp experience` command has six subcommands. They all manipulate the XP of the users in your server.

- `add`: Simple enough. Gives a user more XP. Events that trigger on level-up will not trigger until they next send a
  message (or in some cases, the next time they organically level up).
- `remove`: Same as add, but with a negative sign on the front.
- `set`: This will set a user's experience value to _exactly_ the value you specify. It shares the same non-triggering
  caveats as `add`.
- `reset`: This allows you to quickly reset a user's XP in your server to 0.
- `reset-guild`: This deletes all the leveling data associated with your server. It doesn't delete configuration
  settings, or role rewards.
- `export`: Exports this server's leveling data into a JSON format supported by the `import` command.
- `import`: Imports a leveling JSON file exported by scrape6.py, the `export` command, or any other method you wish.

### XP import & export format

The JSON format used by `xp experience import` and `xp experience export` is a list of structs, with the below
definition:

| Key  |  Value   | Description                 |
| :--: | :------: | --------------------------- |
| `id` | `string` | Stringified discord user ID |
| `xp` |  `int`   | XP count for this user      |

### Rewards

The `xp rewards` command has three subcommands: `add`, `list`, and `remove`.

- `add`: Adds a role that will be given when you reach a specified level.
- `remove`: Removes a role reward. You only need to specify either the level or the target role.
- `list`: List currently active rewards
