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

pub trait ReinterpretPrimitiveBits<O> {
    fn reinterpret_bits(&self) -> O;
}

macro_rules! impl_primitive_reinterpret {
    ($from:ty, $to:ty) => {
        impl ReinterpretPrimitiveBits<$to> for $from {
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            fn reinterpret_bits(&self) -> $to {
                *self as $to
            }
        }
    };
}

impl_primitive_reinterpret!(u8, i8);
impl_primitive_reinterpret!(u16, i16);
impl_primitive_reinterpret!(u32, i32);
impl_primitive_reinterpret!(u64, i64);
impl_primitive_reinterpret!(u128, i128);
impl_primitive_reinterpret!(i8, u8);
impl_primitive_reinterpret!(i16, u16);
impl_primitive_reinterpret!(i32, u32);
impl_primitive_reinterpret!(i64, u64);
impl_primitive_reinterpret!(i128, u128);

/// Fetches the raw ID data from Twilight and returns it as an i64, so it can be stored in Postgres
/// easily.
/// Essentially a no-op.
#[must_use]
#[inline]
pub fn id_to_db<T>(id: Id<T>) -> i64 {
    id.get().reinterpret_bits()
}

/// Create a new checked twilight id from an i64. Only get this from the DB!
/// Essentially a no-op.
#[inline]
#[must_use]
pub fn db_to_id<T>(db: i64) -> Id<T> {
    Id::new(db.reinterpret_bits())
}
