use std::{collections::HashMap, time::Duration};

use rand::Rng;
use sqlx::query;
use twilight_cache_inmemory::CacheableRole;
use twilight_model::{
    channel::message::AllowedMentions,
    gateway::payload::incoming::MessageCreate,
    guild::Permissions,
    id::{
        marker::{ChannelMarker, GuildMarker, RoleMarker},
        Id,
    },
};
use xpd_common::{id_to_db, RoleReward};

const MESSAGE_COOLDOWN: Duration = Duration::from_secs(60);

use crate::{Error, XpdListenerInner};

impl XpdListenerInner {
    pub async fn save(&self, msg: MessageCreate) -> Result<(), Error> {
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
        if msg.author.bot {
            return Ok(());
        }

        let user_cooldown_key = (guild_id, msg.author.id);

        // contains will only be true if the message also has not expired
        if self.messages.read()?.contains(&user_cooldown_key) {
            return Ok(());
        }

        let guild_config = self.get_guild_config(guild_id).await?;

        let xp_added: i64 = rand::thread_rng().gen_range(15..=25);
        let xp_record = query!(
            "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) \
                ON CONFLICT (id, guild) \
                DO UPDATE SET xp=levels.xp+excluded.xp \
                RETURNING xp",
            id_to_db(msg.author.id),
            xp_added,
            id_to_db(guild_id)
        )
        .fetch_one(&self.db)
        .await?;

        let xp = u64::try_from(xp_record.xp).unwrap_or(0);
        let old_xp = u64::try_from(xp_record.xp - xp_added).unwrap_or(0);

        self.messages
            .write()?
            .insert(user_cooldown_key, MESSAGE_COOLDOWN);

        let level_info = mee6::LevelInfo::new(xp);
        let old_level_info = mee6::LevelInfo::new(old_xp);

        let rewards = self.get_guild_rewards(guild_id).await?;

        trace!(
            ?rewards,
            guild_id = guild_id.get(),
            "Got & sorted rewards for guild"
        );

        let user_level: i64 = level_info.level().try_into().unwrap_or(0);
        let old_user_level: i64 = old_level_info.level().try_into().unwrap_or(0);

        let mut reward_idx = None;
        for (idx, data) in rewards.iter().enumerate() {
            if data.requirement > user_level {
                break;
            }
            reward_idx = Some(idx);
        }

        let Some(member) = &msg.member else {
            return Err(Error::NoMember);
        };

        debug!(user = ?msg.author.id, channel = ?msg.channel_id, old = old_xp, new = xp, config = ?guild_config, "Preparing to update user");

        if let Some(reward_idx) = reward_idx {
            // remove all role IDs which are in our rewards list
            let base_roles: Vec<Id<RoleMarker>> = member
                .roles
                .iter()
                .filter(|role_id| !contains(&rewards, **role_id))
                .copied()
                .collect();

            let new_roles = if guild_config.one_at_a_time.is_some_and(|v| v) {
                vec![rewards[reward_idx].id]
            } else {
                rewards[..=reward_idx].iter().map(|v| v.id).collect()
            };

            let mut complete_role_set: Vec<Id<RoleMarker>> =
                Vec::with_capacity(new_roles.len() + base_roles.len());

            complete_role_set.extend(&base_roles);
            complete_role_set.extend(&new_roles);

            // make sure we don't make useless requests to the API
            let can_add_role = self
                .can_add_roles(guild_id, new_roles.as_slice())?
                .can_add_role();
            if member.roles != new_roles && can_add_role {
                debug!(user = ?msg.author.id, old = ?member.roles, new = ?new_roles, "Updating roles for user");
                self.http
                    .update_guild_member(guild_id, msg.author.id)
                    .roles(&new_roles)
                    .await?;
            }
        };

        if let Some(template) = guild_config.level_up_message.as_ref() {
            let target_channel = guild_config.level_up_channel.unwrap_or(msg.channel_id);
            debug!(user = ?msg.author.id, channel = ?msg.channel_id, ?target_channel, old = old_user_level, new = user_level, "Congratulating user");
            if user_level > old_user_level && self.can_create_message(target_channel)? {
                let map = HashMap::from([
                    ("user_mention".to_string(), format!("<@{}>", msg.author.id)),
                    ("level".to_string(), user_level.to_string()),
                ]);
                let message = template.render(&map);
                let allowed_mentions = AllowedMentions {
                    replied_user: true,
                    ..AllowedMentions::default()
                };
                self.http
                    .create_message(msg.channel_id)
                    .reply(msg.id)
                    .allowed_mentions(Some(&allowed_mentions))
                    .content(&message)
                    .await?;
            } else {
                warn!(channel = ?msg.channel_id, "Could not congratulate user")
            }
        }
        Ok(())
    }

    fn can_add_roles(
        &self,
        guild_id: Id<GuildMarker>,
        targets: &[Id<RoleMarker>],
    ) -> Result<CanAddRole, Error> {
        if !self
            .cache
            .permissions()
            .root(self.current_application_id.cast(), guild_id)?
            .contains(Permissions::MANAGE_ROLES)
        {
            debug!(guild = ?guild_id, "No permissions to add role to any user");
            return Ok(CanAddRole::NoManageRoles);
        }

        let highest_role = self
            .cache
            .member_highest_role(guild_id, self.current_application_id.cast())
            .ok_or(Error::NoHighestRoleForSelf)?;

        let my_position = self
            .cache
            .role(highest_role)
            .ok_or(Error::UnknownPositionForOwnHighestRole)?
            .position();
        let max_target_position = {
            let mut max_position = i64::MIN;
            for role in targets {
                let role = self.cache.role(*role).ok_or(Error::NoTargetRoleInCache)?;
                if role.managed {
                    return Ok(CanAddRole::RoleIsManaged);
                }
                max_position = std::cmp::max(max_position, role.position())
            }
            max_position
        };

        if my_position > max_target_position {
            Ok(CanAddRole::Yes)
        } else {
            Ok(CanAddRole::HighestRoleIsLowerRoleThanTarget)
        }
    }

    fn can_create_message(&self, channel_id: Id<ChannelMarker>) -> Result<bool, Error> {
        self.cache
            .permissions()
            .in_channel(self.current_application_id.cast(), channel_id)
            .map(|v| v.contains(Permissions::SEND_MESSAGES))
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CanAddRole {
    Yes,
    NoManageRoles,
    HighestRoleIsLowerRoleThanTarget,
    RoleIsManaged,
}

impl CanAddRole {
    pub fn can_add_role(&self) -> bool {
        matches!(self, CanAddRole::Yes)
    }
}

// any of the items in list are equal to item
fn contains(list: &[RoleReward], item: Id<RoleMarker>) -> bool {
    list.iter().any(|v| v.id == item)
}
