use entity::{channels, platforms, posts};
use percent_encoding::NON_ALPHANUMERIC;
use poise::serenity_prelude::{ChannelId, Mentionable, ParseValue, RoleId};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use std::borrow::Cow;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use tracing::{error, info};

use crate::commands::PlatformType;

use super::{fetch_rss, Checker};

pub struct PostChecker {
    client: Client,
    debug_mode: bool,
    db: DatabaseConnection,
}

#[derive(Debug, Deserialize)]
struct RedditClientConfig {
    user_agent: String,
}

impl PostChecker {
    pub async fn new(debug_mode: bool, connection: DatabaseConnection) -> Arc<PostChecker> {
        let reddit_config = serde_json::from_str::<RedditClientConfig>(
            fs::read_to_string("keys/reddit-rss.json").unwrap().as_str(),
        )
        .unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&reddit_config.user_agent).unwrap(),
        );

        let client = reqwest::Client::builder()
            .https_only(true)
            .default_headers(headers)
            .build()
            .unwrap();

        Arc::new(Self {
            client,
            debug_mode,
            db: connection,
        })
    }
}

#[async_trait::async_trait]
impl Checker for PostChecker {
    fn name(&self) -> &str {
        "Reddit"
    }

    async fn check(
        &self,
        ctx: impl AsRef<serenity::http::Http> + Send + Sync,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let platform_channels = platforms::Entity::find()
            .filter(platforms::Column::PlName.eq(PlatformType::Reddit.to_string()))
            .find_with_related(channels::Entity)
            .all(&self.db)
            .await?
            .into_iter()
            .flat_map(|(_, r)| r)
            .collect::<Vec<_>>();

        for channel in platform_channels {
            let subreddit =
                percent_encoding::utf8_percent_encode(&channel.ch_name, NON_ALPHANUMERIC);

            let res = fetch_rss(
                &self.client,
                Cow::Owned(format!("https://reddit.com/r/{}/new.rss", subreddit)),
            )
            .await;

            if let Err(ref err) = res {
                error!("Reddit fetch error: {:?}", err);
                continue;
            }

            let feed = res.unwrap();

            for entry in feed.entries {
                let id = entry.id;

                let matches = posts::Entity::find()
                    .filter(posts::Column::PoName.eq(id.clone()))
                    .one(&self.db)
                    .await;

                if let Err(ref err) = matches {
                    error!("DB error checking for matches: {err}");
                    continue;
                }

                if let Ok(Some(_)) = matches {
                    continue;
                }

                info!("New post: {}, debug mode: {}", id, self.debug_mode);

                let post = posts::ActiveModel {
                    po_ch_id: Set(channel.ch_id),
                    po_name: Set(id.to_owned()),
                    po_time_added: Set(chrono::Utc::now().naive_utc()),
                    ..Default::default()
                };

                post.save(&self.db).await?;

                let mention = if let Some(role) = channel.ch_role_mention_id {
                    RoleId::from(role as u64).mention().to_string()
                } else {
                    "@everyone".to_owned()
                };

                let author = entry
                    .authors
                    .first()
                    .map_or("<unknown>", |author| &author.name);

                let url = entry.links.first().map_or("", |link| &link.href);

                let subreddit = entry
                    .categories
                    .first()
                    .and_then(|cat| cat.label.as_ref())
                    .unwrap_or(&channel.ch_description);

                let text = format!(
                    "Hey {mention}, user **{author}** has posted on **{subreddit}**!\n{url}"
                );

                ChannelId::from(channel.ch_discord_channel_id as u64)
                    .send_message(&ctx, |msg| {
                        if !self.debug_mode && channel.ch_mention_flag {
                            msg.content(text).allowed_mentions(|am| {
                                if let Some(role_id) = channel.ch_role_mention_id {
                                    am.empty_parse().roles(vec![role_id as u64])
                                } else {
                                    am.empty_parse().parse(ParseValue::Everyone)
                                }
                            })
                        } else {
                            msg.content(text).allowed_mentions(|am| am.empty_parse())
                        }
                    })
                    .await?;
            }
        }

        Ok(())
    }
}
