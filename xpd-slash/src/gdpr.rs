use std::sync::Arc;

use base64::Engine;
use twilight_model::{
    http::attachment::Attachment,
    id::{marker::GuildMarker, Id},
    user::User,
};
use twilight_util::builder::embed::EmbedBuilder;
use xpd_common::Tag;
use xpd_rank_card::{
    cards::Card,
    customizations::{Color, Customizations},
    Font, Toy,
};

use crate::{Error, SlashState, XpdSlashResponse};

pub async fn process_gdpr(state: SlashState, invoker: User) {}

pub async fn delete(state: SlashState, invoker: User) -> Result<XpdSlashResponse, Error> {}

pub async fn download(state: SlashState, invoker: User) -> Result<XpdSlashResponse, Error> {}
