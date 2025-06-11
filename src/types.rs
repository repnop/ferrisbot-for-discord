use crate::{commands, Config};
use anyhow::Error;
use poise::serenity_prelude as serenity;

#[derive(Debug)]
pub struct Data {
    pub discord_guild_id: serenity::GuildId,
    pub application_id: serenity::UserId,
    pub bot_start_time: std::time::Instant,
    pub http: reqwest::Client,
    pub godbolt_metadata: std::sync::Mutex<commands::godbolt::GodboltMetadata>,
}

impl Data {
    pub fn new(config: &Config) -> Self {
        Self {
            discord_guild_id: config.discord.guild_id.into(),
            application_id: config.discord.application_id.into(),
            bot_start_time: std::time::Instant::now(),
            http: reqwest::Client::new(),
            godbolt_metadata: std::sync::Mutex::new(commands::godbolt::GodboltMetadata::default()),
        }
    }
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

// const EMBED_COLOR: (u8, u8, u8) = (0xf7, 0x4c, 0x00);
pub const EMBED_COLOR: (u8, u8, u8) = (0xb7, 0x47, 0x00); // slightly less saturated
