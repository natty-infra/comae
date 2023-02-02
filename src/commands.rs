use crate::sp;
use crate::Data;
use entity::{channels, platforms};
use migration::OnConflict;
use poise::serenity_prelude::Mentionable;
use sea_orm::ModelTrait;
use sea_orm::PaginatorTrait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
pub(crate) async fn account_age(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<sp::User>,
) -> Result<(), Error> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let response = format!(
        "{}'s account was created at <t:{}:f>",
        sp::Mentionable::mention(u),
        u.created_at().unix_timestamp()
    );
    ctx.say(response).await?;
    Ok(())
}

#[derive(poise::ChoiceParameter)]
pub enum PlatformType {
    #[name = "YouTube"]
    YouTube,
}

impl PlatformType {
    fn str_repr(&self) -> &'static str {
        match *self {
            Self::YouTube => "YouTube",
        }
    }
}

#[poise::command(slash_command, prefix_command)]
pub(crate) async fn add_channel(
    ctx: Context<'_>,
    #[description = "Platform"] platform: PlatformType,
    #[description = "Channel ID"] channel_id: String,
    #[description = "Channel name"] channel_name: String,
) -> Result<(), Error> {
    let db = ctx.framework().user_data.database.clone();

    let platform_name = platform.str_repr();

    let platform_info = platforms::Entity::find()
        .filter(platforms::Column::PlName.eq(platform_name))
        .one(&db)
        .await?;

    if platform_info.is_none() {
        let response = "No such platform.";
        ctx.say(response).await?;
        return Ok(());
    }

    let platform_info = platform_info.unwrap();

    let cnt = platform_info
        .find_related(channels::Entity)
        .count(&db)
        .await?;

    const LIMIT: u64 = 12;

    if cnt >= LIMIT {
        let response =
            format!("Too many linked channels in this Discord channel (limit: {LIMIT}).");
        ctx.say(response).await?;
        return Ok(());
    }

    let channel = channels::ActiveModel {
        ch_name: Set(channel_id),
        ch_description: Set(channel_name.clone()),
        ch_pl_id: Set(platform_info.pl_id),
        ch_discord_channel_id: Set(ctx.channel_id().into()),
        ..Default::default()
    };

    channels::Entity::insert(channel)
        .on_conflict(
            OnConflict::columns([
                channels::Column::ChName,
                channels::Column::ChDiscordChannelId,
            ])
            .update_columns([channels::Column::ChDescription])
            .to_owned(),
        )
        .exec(&db)
        .await?;

    ctx.send(|f| {
        f.content(format!(
            "Channel configuration updated: **{channel_name}** -> **{}**.",
            ctx.channel_id().mention()
        ))
        .allowed_mentions(|m| m.empty_parse())
    })
    .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub(crate) async fn list_channels(
    ctx: Context<'_>,
    #[description = "Platform"] platform: Option<PlatformType>,
) -> Result<(), Error> {
    let db = ctx.framework().user_data.database.clone();

    let platform_name = platform.as_ref().map(PlatformType::str_repr);

    let sel_base = channels::Entity::find()
        .filter(channels::Column::ChDiscordChannelId.eq(ctx.channel_id().0 as i64));

    let sel = if let Some(platform_name_str) = platform_name {
        sel_base
            .find_also_related(platforms::Entity)
            .filter(platforms::Column::PlName.eq(platform_name_str))
            .all(&db)
            .await?
            .into_iter()
            .map(|(m, _)| m)
            .collect::<Vec<_>>()
    } else {
        sel_base.all(&db).await?
    };

    if sel.is_empty() {
        let response = "No channel links found.";
        ctx.say(response).await?;
        return Ok(());
    };

    let filter = if let Some(platform) = platform {
        format!(" filtered by **{}**", platform.name())
    } else {
        "".to_owned()
    };

    ctx.send(|f| {
        f.embed(|e| {
            e.title("Linked Channels")
                .description(format!(
                    "The list of active channel links in {}{filter}.",
                    ctx.channel_id().mention()
                ))
                .colour((149, 66, 245))
                .fields(
                    sel.into_iter()
                        .map(|ch| (ch.ch_description, ch.ch_name, true)),
                )
        })
        .allowed_mentions(|m| m.empty_parse())
    })
    .await?;
    Ok(())
}
