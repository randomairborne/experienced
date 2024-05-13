#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{fmt::Display, str::FromStr};

use twilight_model::{
    id::{
        marker::{GuildMarker, UserMarker},
        Id,
    },
    util::ImageHash,
};

pub trait Tag {
    #[must_use]
    fn tag(&self) -> String;
}

impl Tag for twilight_model::user::User {
    fn tag(&self) -> String {
        name_discrim_to_tag(&self.name, self.discriminator)
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

/// Fetches the raw ID data from Twilight and returns it as an i64, so it can be stored in Postgres
/// easily.
/// Essentially a no-op.
#[must_use]
#[inline]
pub const fn id_to_db<T>(id: Id<T>) -> i64 {
    i64::from_le_bytes(id.get().to_le_bytes())
}

/// Create a new checked twilight id from an i64. Only get this from the DB!
/// Essentially a no-op.
#[inline]
#[must_use]
pub const fn db_to_id<T>(db: i64) -> Id<T> {
    Id::new(u64::from_le_bytes(db.to_le_bytes()))
}
