#![warn(rust_2018_idioms, clippy::pedantic)]
#![allow(
	clippy::too_many_lines,
	clippy::missing_errors_doc,
	clippy::missing_panics_doc,
	clippy::cast_possible_wrap,
	clippy::module_name_repetitions,
	clippy::assigning_clones, // Too many false triggers
)]

mod checks;
mod commands;
mod helpers;
mod types;

use anyhow::Error;
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use types::Data;

#[derive(Deserialize)]
struct Config {
    discord: DiscordConfig,
}

#[derive(Deserialize)]
struct DiscordConfig {
    token: String,
    guild_id: u64,
    application_id: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    const FAILED_CODEBLOCK: &str = "\
Missing code block. Please use the following markdown:
`` `code here` ``
or
```ansi
`\x1b[0m`\x1b[0m`rust
code here
`\x1b[0m`\x1b[0m`
```";

    let config_path = std::env::args().nth(1).expect("path to config file as the first argument");
    let config: Config = toml::from_str(&std::fs::read_to_string(config_path).expect("failed to read config file"))
        .expect("failed to deserialize config file");

    let token = config.discord.token.clone();

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                let data = Data::new(&config);

                debug!("Registering commands...");
                poise::builtins::register_in_guild(ctx, &framework.options().commands, data.discord_guild_id).await?;

                debug!("Setting activity text");
                ctx.set_activity(Some(serenity::ActivityData::listening("/help")));

                info!("rustbot logged in as {}", ready.user.name);
                Ok(data)
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::man::man(),
                commands::crates::crate_(),
                commands::crates::doc(),
                commands::godbolt::godbolt(),
                commands::godbolt::mca(),
                commands::godbolt::llvmir(),
                commands::godbolt::targets(),
                commands::utilities::go(),
                commands::utilities::source(),
                commands::utilities::help(),
                commands::utilities::register(),
                commands::utilities::uptime(),
                commands::utilities::conradluget(),
                commands::utilities::cleanup(),
                commands::utilities::ban(),
                commands::utilities::selftimeout(),
                commands::thread_pin::thread_pin(),
                commands::playground::play(),
                commands::playground::playwarn(),
                commands::playground::eval(),
                commands::playground::miri(),
                commands::playground::expand(),
                commands::playground::clippy(),
                commands::playground::fmt(),
                commands::playground::microbench(),
                commands::playground::procmacro(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("?".into()),
                additional_prefixes: vec![
                    poise::Prefix::Literal("🦀 "),
                    poise::Prefix::Literal("🦀"),
                    poise::Prefix::Literal("<:ferris:358652670585733120> "),
                    poise::Prefix::Literal("<:ferris:358652670585733120>"),
                    poise::Prefix::Literal("<:ferrisballSweat:678714352450142239> "),
                    poise::Prefix::Literal("<:ferrisballSweat:678714352450142239>"),
                    poise::Prefix::Literal("<:ferrisCat:1183779700485664820> "),
                    poise::Prefix::Literal("<:ferrisCat:1183779700485664820>"),
                    poise::Prefix::Literal("<:ferrisOwO:579331467000283136> "),
                    poise::Prefix::Literal("<:ferrisOwO:579331467000283136>"),
                    poise::Prefix::Regex(
                        "(yo |hey )?(crab|ferris|fewwis),? can you (please |pwease )?".parse().unwrap(),
                    ),
                ],
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    Duration::from_secs(60 * 5), // 5 minutes
                ))),
                ..Default::default()
            },
            // The global error handler for all error cases that may occur
            on_error: |error| {
                Box::pin(async move {
                    warn!("Encountered error: {:?}", error);
                    if let poise::FrameworkError::ArgumentParse { error, ctx, .. } = error {
                        let response = if error.is::<poise::CodeBlockError>() {
                            FAILED_CODEBLOCK.to_owned()
                        } else if let Some(multiline_help) = &ctx.command().help_text {
                            format!("**{error}**\n{multiline_help}")
                        } else {
                            error.to_string()
                        };

                        if let Err(e) = ctx.say(response).await {
                            warn!("{}", e);
                        }
                    } else if let poise::FrameworkError::Command { ctx, error, .. } = error {
                        if error.is::<poise::CodeBlockError>() {
                            if let Err(e) = ctx.say(FAILED_CODEBLOCK.to_owned()).await {
                                warn!("{}", e);
                            }
                        }
                        if let Err(e) = ctx.say(error.to_string()).await {
                            warn!("{}", e);
                        }
                    }
                })
            },
            // This code is run before every command
            pre_command: |ctx| {
                Box::pin(async move {
                    let channel_name = &ctx.channel_id().name(&ctx).await.unwrap_or_else(|_| "<unknown>".to_owned());
                    let author = &ctx.author().name;

                    info!("{} in {} used slash command '{}'", author, channel_name, &ctx.invoked_command_name());
                })
            },
            // This code is run after a command if it was successful (returned Ok)
            post_command: |ctx| {
                Box::pin(async move {
                    info!("Executed command {}!", ctx.command().qualified_name);
                })
            },
            // Every command invocation must pass this check to continue execution
            command_check: Some(|_ctx| Box::pin(async move { Ok(true) })),
            // Enforce command checks even for owners (enforced by default)
            // Set to true to bypass checks, which is useful for testing
            skip_checks_for_owners: false,
            event_handler: |_ctx, _event, _framework, _data| Box::pin(std::future::ready(Ok(()))),
            // Disallow all mentions (except those to the replied user) by default
            allowed_mentions: Some(serenity::CreateAllowedMentions::new().replied_user(true)),
            ..Default::default()
        })
        .build();

    let intents = serenity::GatewayIntents::all();

    let mut client =
        serenity::ClientBuilder::new(token, intents).framework(framework).await.expect("failed to create client");

    client.start().await.expect("failed to run ferrisbot");
}
