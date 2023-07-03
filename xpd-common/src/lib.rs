#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RedisUser {
    pub id: u64,
    pub username: Option<String>,
    pub discriminator: Option<u16>,
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
        let Some(discriminator) = self.discriminator else { return self.id.to_string(); };
        let Some(name) = &self.username else { return self.id.to_string(); };
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
