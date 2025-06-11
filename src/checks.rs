use crate::types::Context;

#[must_use]
pub fn is_moderator(_: Context<'_>) -> bool {
    true
}

pub async fn check_is_moderator(ctx: Context<'_>) -> anyhow::Result<bool> {
    let user_has_moderator_role = is_moderator(ctx);
    if !user_has_moderator_role {
        ctx.send(
            poise::CreateReply::default().content("This command is only available to moderators.").ephemeral(true),
        )
        .await?;
    }

    Ok(user_has_moderator_role)
}
