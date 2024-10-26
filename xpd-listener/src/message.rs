use std::collections::HashMap;

use rand::Rng;
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::payload::incoming::MessageCreate,
    guild::PartialMember,
    id::{
        marker::{GuildMarker, RoleMarker, UserMarker},
        Id,
    },
};
use xpd_common::{
    snowflake_to_timestamp, DisplayName, GuildConfig, RoleReward, DEFAULT_MAX_XP_PER_MESSAGE,
    DEFAULT_MESSAGE_COOLDOWN, DEFAULT_MIN_XP_PER_MESSAGE,
};

use crate::{Error, XpdListenerInner};

type RoleList = Vec<Id<RoleMarker>>;

impl XpdListenerInner {
    pub async fn save(&self, msg: MessageCreate) -> Result<(), Error> {
        if msg.author.bot {
            return Ok(());
        }
        if let Some(guild_id) = msg.guild_id {
            self.save_msg_send(guild_id, msg).await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, msg))]
    async fn save_msg_send(
        &self,
        guild_id: Id<GuildMarker>,
        msg: MessageCreate,
    ) -> Result<(), Error> {
        let Some(member) = &msg.member else {
            return Err(Error::NoMember);
        };

        let user_cooldown_key = (guild_id, msg.author.id);
        let this_message_sts = snowflake_to_timestamp(msg.id);

        let guild_config = self.get_guild_config(guild_id).await?;
        let config_max_xp_per_msg = guild_config
            .max_xp_per_message
            .unwrap_or(DEFAULT_MAX_XP_PER_MESSAGE);
        let config_min_xp_per_msg = guild_config
            .min_xp_per_message
            .unwrap_or(DEFAULT_MIN_XP_PER_MESSAGE);

        // if the last message timestamp plus the cooldown period is larger than the current sent at epoch,
        // we want to return immediately because the "expiry time" is still in the future
        let cooldown: i64 = guild_config
            .cooldown
            .unwrap_or(DEFAULT_MESSAGE_COOLDOWN)
            .into();
        if self
            .messages
            .read()?
            .get(&user_cooldown_key)
            .is_some_and(|last_message_sts| last_message_sts + cooldown > this_message_sts)
        {
            return Ok(());
        }

        let xp_added: i64 = if config_max_xp_per_msg == config_min_xp_per_msg {
            config_max_xp_per_msg
        } else {
            rand::thread_rng().gen_range(config_min_xp_per_msg..=config_max_xp_per_msg)
        }
        .into();

        let xp_i64 = xpd_database::add_xp(&self.db, msg.author.id, guild_id, xp_added).await?;
        let xp = u64::try_from(xp_i64).unwrap_or(0);
        let old_xp = u64::try_from(xp_i64 - xp_added).unwrap_or(0);

        self.messages
            .write()?
            .insert(user_cooldown_key, this_message_sts);

        let level_info = mee6::LevelInfo::new(xp);
        let old_level_info = mee6::LevelInfo::new(old_xp);

        let rewards = self.get_guild_rewards(guild_id).await?;

        debug!(
            ?rewards,
            guild_id = guild_id.get(),
            "Got & sorted rewards for guild"
        );

        let user_level: i64 = level_info.level().try_into().unwrap_or(-1);
        let old_user_level: i64 = old_level_info.level().try_into().unwrap_or(-1);

        debug!(user = ?msg.author.id, channel = ?msg.channel_id, old_xp, new_xp = xp, user_level, old_user_level, config = ?guild_config, "Preparing to update user");

        if user_level > old_user_level {
            self.congratulate_user(&guild_config, &msg, user_level, old_user_level, xp, old_xp)
                .await?;
        }
        self.add_user_role(
            guild_id,
            &guild_config,
            msg.author.id,
            member,
            &rewards,
            user_level,
        )
        .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self, member))]
    async fn add_user_role(
        &self,
        guild_id: Id<GuildMarker>,
        guild_config: &GuildConfig,
        user_id: Id<UserMarker>,
        member: &PartialMember,
        rewards: &[RoleReward],
        user_level: i64,
    ) -> Result<(), Error> {
        let Some(reward_idx) = get_reward_idx(rewards, user_level) else {
            // This ensures we don't delete roles or otherwise edit them if none are earned.
            return Ok(());
        };
        let roles = get_role_changes(guild_config, member, rewards, reward_idx);

        // make sure we don't make useless error requests to the API
        let can_update_roles = xpd_util::can_manage_roles(
            &self.cache,
            self.bot_id,
            guild_id,
            roles.changed_roles.as_slice(),
        )?
        .can_update_roles();
        if can_update_roles {
            debug!(user = ?user_id, old = ?member.roles, new = ?roles, "Updating roles for user");
            self.http
                .update_guild_member(guild_id, user_id)
                .roles(&roles.total_roles)
                .await?;
        } else {
            warn!(user = ?user_id, old = ?member.roles, new = ?roles, "Could not update roles for user");
        }
        Ok(())
    }

    async fn congratulate_user(
        &self,
        guild_config: &GuildConfig,
        msg: &MessageCreate,
        user_level: i64,
        old_user_level: i64,
        xp: u64,
        old_xp: u64,
    ) -> Result<(), Error> {
        let Some(template) = guild_config.level_up_message.as_ref() else {
            return Ok(());
        };
        let target_channel = guild_config.level_up_channel.unwrap_or(msg.channel_id);
        debug!(user = ?msg.author.id, channel = ?msg.channel_id, ?target_channel, old = old_user_level, new = user_level, "Congratulating user");
        if !xpd_util::can_create_message(&self.cache, self.bot_id, target_channel)? {
            warn!(channel = ?msg.channel_id, user = ?msg.author.id, guild = ?msg.guild_id, "Could not congratulate user");
            return Ok(());
        }
        let map = HashMap::from([
            ("user_id".to_string(), msg.author.id.to_string()),
            ("user_mention".to_string(), format!("<@{}>", msg.author.id)),
            ("user_username".to_string(), msg.author.name.clone()),
            (
                "user_display_name".to_string(),
                msg.author.display_name().to_string(),
            ),
            (
                "user_nickname".to_string(),
                msg.member
                    .as_ref()
                    .and_then(|v| v.nick.clone())
                    .unwrap_or_else(|| msg.author.display_name().to_string()),
            ),
            ("old_level".to_string(), old_user_level.to_string()),
            ("level".to_string(), user_level.to_string()),
            ("old_xp".to_string(), xp.to_string()),
            ("xp".to_string(), old_xp.to_string()),
        ]);
        let message = template.render(&map);

        let allowed_mentions = if let Some(false) = guild_config.ping_on_level_up {
            AllowedMentions::default()
        } else {
            AllowedMentions {
                replied_user: true,
                users: vec![msg.author.id],
                ..AllowedMentions::default()
            }
        };

        let mut congratulatory_msg = self.http.create_message(target_channel);
        if target_channel == msg.channel_id {
            // only reply to a message if it's in the same channel
            congratulatory_msg = congratulatory_msg.reply(msg.id);
        }
        congratulatory_msg
            .allowed_mentions(Some(&allowed_mentions))
            .content(&message)
            .await?;
        Ok(())
    }
}

fn get_reward_idx(rewards: &[RoleReward], user_level: i64) -> Option<usize> {
    let mut reward_idx = None;
    for (idx, data) in rewards.iter().enumerate() {
        if data.requirement > user_level {
            break;
        }
        reward_idx = Some(idx);
    }
    reward_idx
}

#[derive(Debug)]
struct RoleChangeList {
    total_roles: RoleList,
    changed_roles: RoleList,
}

fn get_role_changes(
    guild_config: &GuildConfig,
    member: &PartialMember,
    rewards: &[RoleReward],
    reward_idx: usize,
) -> RoleChangeList {
    let one_at_a_time = guild_config.one_at_a_time.is_some_and(|v| v);

    let previous_role = rewards[reward_idx.saturating_sub(1)].id;
    let achieved_roles = if one_at_a_time {
        &rewards[reward_idx..=reward_idx]
    } else {
        &rewards[..=reward_idx]
    };
    let roles_to_add = achieved_roles.iter().filter_map(|v| {
        if !member.roles.contains(&v.id) {
            Some(v.id)
        } else {
            None
        }
    });

    let mut changed_roles = Vec::with_capacity(8);

    let total_roles: RoleList = member
        .roles
        .iter()
        .copied()
        .chain(roles_to_add)
        // if we're not doing one at a time, we always return true.
        // If the reward index is 0, we won't be removing any roles ever.
        // Otherwise, we return true if v is not the previous role.
        // If we're removing it, or the member didn't have it before
        // because it was added in the chain, we also add it to the changelist.
        // If we return false, we want to know that we are REMOVING that role.
        .filter(|v| {
            let keeper = !one_at_a_time || reward_idx == 0 || *v != previous_role;
            if !keeper || !member.roles.contains(v) {
                changed_roles.push(*v);
            };
            keeper
        })
        .collect();

    RoleChangeList {
        total_roles,
        changed_roles,
    }
}

#[cfg(test)]
mod tests {
    use twilight_model::guild::MemberFlags;

    use super::*;

    fn member_with_roles(roles: impl Into<RoleList>) -> PartialMember {
        PartialMember {
            avatar: None,
            communication_disabled_until: None,
            deaf: false,
            flags: MemberFlags::empty(),
            joined_at: None,
            mute: false,
            nick: None,
            permissions: None,
            premium_since: None,
            roles: roles.into(),
            user: None,
        }
    }

    // Non-one at a time only changes the behavior to not remove the previous role
    fn conf_one_at_time() -> GuildConfig {
        GuildConfig {
            one_at_a_time: Some(true),
            ..Default::default()
        }
    }

    #[test]
    fn no_changes() {
        let rewards = [RoleReward {
            id: Id::new(1),
            requirement: 2,
        }];
        let reward_idx = get_reward_idx(&rewards, 2).unwrap();
        let member = member_with_roles([Id::new(1)]);
        let changes = get_role_changes(&conf_one_at_time(), &member, &rewards, reward_idx);
        assert_eq!(changes.changed_roles, RoleList::new());
        assert_eq!(changes.total_roles, [Id::new(1)]);
    }

    #[test]
    fn minecraft_discord() {
        let rewards = [RoleReward {
            id: Id::new(1),
            requirement: 5,
        }];
        let reward_idx = get_reward_idx(&rewards, 5).unwrap();
        let member = member_with_roles([]);
        let changes = get_role_changes(&conf_one_at_time(), &member, &rewards, reward_idx);
        assert_eq!(changes.changed_roles, [Id::new(1)]);
        assert_eq!(changes.total_roles, [Id::new(1)]);
    }

    #[test]
    fn add_one_role() {
        let rewards = [
            RoleReward {
                id: Id::new(1),
                requirement: 2,
            },
            RoleReward {
                id: Id::new(2),
                requirement: 10,
            },
        ];
        let reward_idx = get_reward_idx(&rewards, 4).unwrap();
        let member = member_with_roles([]);
        let changes = get_role_changes(&conf_one_at_time(), &member, &rewards, reward_idx);
        assert_eq!(changes.changed_roles, vec![Id::new(1)]);
        assert_eq!(changes.total_roles, [Id::new(1)]);
    }

    const TEST_REWARDS: [RoleReward; 3] = [
        RoleReward {
            id: Id::new(1),
            requirement: 2,
        },
        RoleReward {
            id: Id::new(2),
            requirement: 4,
        },
        RoleReward {
            id: Id::new(3),
            requirement: 10,
        },
    ];

    #[test]
    fn skip_roles() {
        let reward_idx = get_reward_idx(&TEST_REWARDS, 10).unwrap();
        let member = member_with_roles([]);
        let changes = get_role_changes(&conf_one_at_time(), &member, &TEST_REWARDS, reward_idx);
        assert_eq!(changes.changed_roles, [Id::new(3)]);
        assert_eq!(changes.total_roles, [Id::new(3)]);
    }
    #[test]
    fn stop_on_role() {
        let reward_idx = get_reward_idx(&TEST_REWARDS, 5).unwrap();
        let member = member_with_roles([Id::new(1)]);
        let changes = get_role_changes(&conf_one_at_time(), &member, &TEST_REWARDS, reward_idx);
        assert_eq!(changes.changed_roles, [Id::new(1), Id::new(2)]);
        assert_eq!(changes.total_roles, [Id::new(2)]);
    }

    #[test]
    fn conf_many_doesnt_nuke() {
        let reward_idx = get_reward_idx(&TEST_REWARDS, 5).unwrap();
        let member = member_with_roles([Id::new(1)]);
        let changes = get_role_changes(&GuildConfig::default(), &member, &TEST_REWARDS, reward_idx);
        assert_eq!(changes.changed_roles, [Id::new(2)]);
        assert_eq!(changes.total_roles, [Id::new(1), Id::new(2)]);
    }

    #[test]
    fn conf_many_adds_many() {
        let reward_idx = get_reward_idx(&TEST_REWARDS, 11).unwrap();
        let member = member_with_roles([]);
        let changes = get_role_changes(&GuildConfig::default(), &member, &TEST_REWARDS, reward_idx);
        assert_eq!(changes.changed_roles, [Id::new(1), Id::new(2), Id::new(3)]);
        assert_eq!(changes.total_roles, [Id::new(1), Id::new(2), Id::new(3)]);
    }

    #[test]
    fn leave_alone_higher_roles() {
        let reward_idx = get_reward_idx(&TEST_REWARDS, 3).unwrap();
        let member = member_with_roles([Id::new(3)]);
        let changes = get_role_changes(&GuildConfig::default(), &member, &TEST_REWARDS, reward_idx);
        assert_eq!(changes.changed_roles, [Id::new(1)]);
        assert_eq!(changes.total_roles, [Id::new(3), Id::new(1)]);
    }
}
