use std::time::Duration;

use rand::Rng;
use sqlx::query;
use twilight_model::{
    gateway::payload::incoming::MessageCreate,
    id::{
        marker::{GuildMarker, RoleMarker},
        Id,
    },
};
use xpd_common::{id_to_db, RoleReward};
use xpd_permission_cache::CanAddRolesInfo;

const MESSAGE_COOLDOWN: Duration = Duration::from_secs(60);

use crate::{Error, XpdListenerInner};

impl XpdListenerInner {
    pub async fn save(&self, msg: MessageCreate) -> Result<(), Error> {
        if let Some(guild_id) = msg.guild_id {
            self.save_msg_send(guild_id, msg).await?;
        }
        Ok(())
    }

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

        let xp_count: i64 = rand::thread_rng().gen_range(15..=25);
        let xp_record = query!(
            "INSERT INTO levels (id, xp, guild) VALUES ($1, $2, $3) \
                ON CONFLICT (id, guild) \
                DO UPDATE SET xp=levels.xp+excluded.xp \
                RETURNING xp",
            id_to_db(msg.author.id),
            xp_count,
            id_to_db(guild_id)
        )
        .fetch_one(&self.db)
        .await?;

        let xp = u64::try_from(xp_record.xp).unwrap_or(0);
        self.messages
            .write()?
            .insert(user_cooldown_key, MESSAGE_COOLDOWN);

        let level_info = mee6::LevelInfo::new(xp);

        let rewards = self.get_guild_rewards(guild_id).await?;

        debug!(
            ?rewards,
            guild_id = guild_id.get(),
            "Got & sorted rewards for guild"
        );

        let user_level: i64 = level_info.level().try_into().unwrap_or(0);

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

        let Some(reward_idx) = reward_idx else {
            return Ok(());
        };

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
        
        trace!(cache = ?self.cache, "Have cache");
        
        // ensure we have perms to add roles
        match self
            .cache
            .can_add_roles(guild_id, new_roles.as_slice())
            .unwrap_or(CanAddRolesInfo::CanAddRoles)
        {
            CanAddRolesInfo::CanAddRoles => {}
            role_adding_issue => return Err(Error::NoPermsToAddRoles(guild_id, role_adding_issue)),
        }

        let mut complete_role_set: Vec<Id<RoleMarker>> =
            Vec::with_capacity(new_roles.len() + base_roles.len());

        complete_role_set.extend(&base_roles);
        complete_role_set.extend(&new_roles);

        // make sure we don't make useless requests to the API
        if member.roles != new_roles {
            debug!(user = ?msg.author.id, old = ?member.roles, new = ?new_roles, "Updating roles for user");
            self.http
                .update_guild_member(guild_id, msg.author.id)
                .roles(&new_roles)
                .await?;
        }

        Ok(())
    }
}

// any of the items in list are equal to item
fn contains(list: &[RoleReward], item: Id<RoleMarker>) -> bool {
    list.iter().any(|v| v.id == item)
}
