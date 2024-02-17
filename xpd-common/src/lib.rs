#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{fmt::Display, str::FromStr};

use twilight_model::{
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
    util::ImageHash,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RedisUser {
    pub id: Id<UserMarker>,
    pub username: Option<String>,
    pub discriminator: Option<u16>,
    pub avatar_hash: Option<ImageHash>,
    pub banner_hash: Option<ImageHash>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RedisGuild {
    pub id: Id<GuildMarker>,
    pub name: String,
    pub banner_hash: Option<ImageHash>,
    pub icon_hash: Option<ImageHash>,
}

pub trait Tag {
    #[must_use]
    fn tag(&self) -> String;
}

impl Tag for twilight_model::user::User {
    fn tag(&self) -> String {
        if self.discriminator == 0 {
            self.name.clone()
        } else {
            format!("{}#{}", self.name, self.discriminator())
        }
    }
}

impl Tag for RedisUser {
    fn tag(&self) -> String {
        let Some(discriminator) = self.discriminator else {
            return self.id.to_string();
        };
        let Some(name) = &self.username else {
            return self.id.to_string();
        };
        name_discrim_to_tag(name, discriminator)
    }
}

fn name_discrim_to_tag(name: &str, discriminator: u16) -> String {
    if discriminator == 0 {
        name.to_string()
    } else {
        format!(
            "{}#{}",
            name,
            twilight_model::user::DiscriminatorDisplay::new(discriminator)
        )
    }
}

/// Get environment variable and parse it, panicking on failure
/// # Panics
/// If the environment variable cannot be found or parsed
#[must_use]
pub fn parse_var<T>(key: &str) -> T
where
    T: FromStr,
    T::Err: Display,
{
    get_var(key)
        .parse()
        .unwrap_or_else(|e| panic!("{key} could not be parsed: {e}"))
}

/// Get environment variable and parse it, panicking on failure
/// # Panics
/// If the environment variable cannot be found or parsed
#[must_use]
pub fn get_var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|e| panic!("Expected {key} in environment: {e}"))
}

/// This is basically a no-op. It fetches a field on a struct through a method and casts it
#[must_use]
#[inline]
pub const fn id_to_db<T>(id: Id<T>) -> i64 {
    #[allow(clippy::cast_possible_wrap)]
    {
        id.get() as i64
    }
}

/// Create a new checked twilight id from an i64. Only get this from the DB!
#[must_use]
#[inline]
pub const fn db_to_id<T>(db: i64) -> Id<T> {
    #[allow(clippy::cast_sign_loss)]
    Id::new(db as u64)
}
