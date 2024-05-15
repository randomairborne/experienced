use twilight_model::{
    application::command::CommandOptionChoice,
    channel::message::{AllowedMentions, Component, Embed, MessageFlags},
    http::{
        attachment::Attachment,
        interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    },
};
use twilight_util::builder::embed::EmbedBuilder;

#[derive(Debug, Default, Clone)]
#[allow(clippy::module_name_repetitions)]
pub struct XpdSlashResponse {
    pub allowed_mentions: Option<AllowedMentions>,
    pub attachments: Option<Vec<Attachment>>,
    pub choices: Option<Vec<CommandOptionChoice>>,
    pub components: Option<Vec<Component>>,
    pub content: Option<String>,
    pub custom_id: Option<String>,
    pub embeds: Option<Vec<Embed>>,
    pub flags: Option<MessageFlags>,
    pub title: Option<String>,
    pub tts: Option<bool>,
}

impl XpdSlashResponse {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_embed_text(text: impl Into<String>) -> Self {
        let embed = EmbedBuilder::new().description(text).build();
        Self::new().embeds([embed])
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn allowed_mentions_o(self, allowed_mentions: Option<AllowedMentions>) -> Self {
        Self {
            allowed_mentions,
            ..self
        }
    }

    #[must_use]
    pub fn attachments_o(self, attachments: Option<impl Into<Vec<Attachment>>>) -> Self {
        Self {
            attachments: attachments.map(std::convert::Into::into),
            ..self
        }
    }

    #[must_use]
    pub fn choices_o(self, choices: Option<impl Into<Vec<CommandOptionChoice>>>) -> Self {
        Self {
            choices: choices.map(Into::into),
            ..self
        }
    }

    #[must_use]
    pub fn components_o(self, components: Option<impl Into<Vec<Component>>>) -> Self {
        Self {
            components: components.map(Into::into),
            ..self
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]

    pub fn content_o(self, content: Option<String>) -> Self {
        Self { content, ..self }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]

    pub fn custom_id_o(self, custom_id: Option<String>) -> Self {
        Self { custom_id, ..self }
    }

    #[must_use]
    pub fn embeds_o(self, embeds: Option<impl Into<Vec<Embed>>>) -> Self {
        Self {
            embeds: embeds.map(Into::into),
            ..self
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn flags_o(self, flags: Option<MessageFlags>) -> Self {
        Self { flags, ..self }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn title_o(self, title: Option<String>) -> Self {
        Self { title, ..self }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn tts_o(self, tts: Option<bool>) -> Self {
        Self { tts, ..self }
    }

    #[must_use]
    pub fn allowed_mentions(self, allowed_mentions: AllowedMentions) -> Self {
        self.allowed_mentions_o(Some(allowed_mentions))
    }

    #[must_use]
    pub fn attachments(self, attachments: impl Into<Vec<Attachment>>) -> Self {
        self.attachments_o(Some(attachments))
    }

    #[must_use]
    pub fn choices(self, choices: impl Into<Vec<CommandOptionChoice>>) -> Self {
        self.choices_o(Some(choices))
    }

    #[must_use]
    pub fn components(self, components: impl Into<Vec<Component>>) -> Self {
        self.components_o(Some(components))
    }

    #[must_use]
    pub fn content(self, content: String) -> Self {
        self.content_o(Some(content))
    }

    #[must_use]
    pub fn custom_id(self, custom_id: String) -> Self {
        self.custom_id_o(Some(custom_id))
    }

    #[must_use]
    pub fn embeds(self, embeds: impl Into<Vec<Embed>>) -> Self {
        self.embeds_o(Some(embeds))
    }

    #[must_use]
    pub fn flags(self, flags: MessageFlags) -> Self {
        self.flags_o(Some(flags))
    }

    #[must_use]
    pub fn title(self, title: String) -> Self {
        self.title_o(Some(title))
    }

    #[must_use]
    pub fn tts(self, tts: bool) -> Self {
        self.tts_o(Some(tts))
    }

    #[must_use]
    pub fn ephemeral(mut self, ephemeral: bool) -> Self {
        if let Some(flags) = &mut self.flags {
            if ephemeral {
                flags.insert(MessageFlags::EPHEMERAL);
            } else {
                flags.remove(MessageFlags::EPHEMERAL);
            }
        } else if ephemeral {
            self.flags = Some(MessageFlags::EPHEMERAL);
        }
        self
    }
}

impl From<XpdSlashResponse> for InteractionResponseData {
    fn from(value: XpdSlashResponse) -> Self {
        Self {
            allowed_mentions: value.allowed_mentions,
            attachments: value.attachments,
            choices: value.choices,
            components: value.components,
            content: value.content,
            custom_id: value.custom_id,
            embeds: value.embeds,
            flags: value.flags,
            title: value.title,
            tts: value.tts,
        }
    }
}

impl From<InteractionResponseData> for XpdSlashResponse {
    fn from(value: InteractionResponseData) -> Self {
        Self {
            allowed_mentions: value.allowed_mentions,
            attachments: value.attachments,
            choices: value.choices,
            components: value.components,
            content: value.content,
            custom_id: value.custom_id,
            embeds: value.embeds,
            flags: value.flags,
            title: value.title,
            tts: value.tts,
        }
    }
}

impl From<XpdSlashResponse> for InteractionResponse {
    fn from(value: XpdSlashResponse) -> Self {
        Self {
            data: Some(value.into()),
            kind: InteractionResponseType::ChannelMessageWithSource,
        }
    }
}
