---
title: "Documentation"
layout: "../layouts/Layout.astro"
description: "Learn how to configure experienced"
---

## Concepts

The entrypoint of most configuration is the `/config` command. It has subcommands, `rewards` and `leveling`, for configuring level-up behavior and role-reward assignment behavior. Values cannot yet be cleared once set, so you must reset your settings if you wish to disable a setting. This will be improved soon.

## Leveling

The variables available in level up messages are `user_mention` and `level`. These are a ping for the user who leveled up, and the numeric value of the user's level, respectively.
The level-up channel may only be enabled if the level-up message is set.

## Rewards

The boolean `one_at_a_time` determines if a user is given all of the reward roles they have earned, or only the highest one.
