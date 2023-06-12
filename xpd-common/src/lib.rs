#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RedisUser {
    pub id: u64,
    pub username: Option<String>,
    pub discriminator: Option<String>,
}
