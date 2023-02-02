use entity::{channels, platforms, posts};
use google_youtube3::hyper::client::HttpConnector;
use google_youtube3::hyper_rustls::HttpsConnector;
use google_youtube3::{hyper, hyper_rustls, oauth2, YouTube};
use poise::serenity_prelude::{ChannelId, ParseValue};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::error::Error;
use std::fs;
use std::sync::Arc;
use tracing::{error, info};

use crate::commands::PlatformType;

pub struct UploadChecker {
    hub: YouTube<HttpsConnector<HttpConnector>>,
    debug_mode: bool,
    db: DatabaseConnection,
}

impl<'a> UploadChecker {
    pub async fn new(debug_mode: bool, connection: DatabaseConnection) -> Arc<UploadChecker> {
        let connector = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http2()
            .build();
        let youtube_client = hyper::Client::builder().build(connector);

        let service_account_key = serde_json::from_str::<oauth2::ServiceAccountKey>(
            fs::read_to_string("keys/youtube-service-account.json")
                .unwrap()
                .as_str(),
        )
        .unwrap();

        let auth = oauth2::ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
            .unwrap();

        Arc::new(Self {
            hub: YouTube::new(youtube_client, auth),
            debug_mode,
            db: connection.clone(),
        })
    }

    pub async fn check(&self, ctx: impl AsRef<serenity::http::Http>) -> Result<(), Box<dyn Error>> {
        let platform_channels = platforms::Entity::find()
            .filter(platforms::Column::PlName.eq(PlatformType::YouTube.to_string()))
            .find_with_related(channels::Entity)
            .all(&self.db)
            .await?
            .into_iter()
            .flat_map(|(_, r)| r)
            .collect::<Vec<_>>();

        for channel in platform_channels {
            let result = self
                .hub
                .playlist_items()
                .list(&vec!["contentDetails".to_string()])
                .playlist_id(&channel.ch_name)
                .doit()
                .await;

            if let Err(ref err) = result {
                error!("YouTube API error: {:?}", err);
                continue;
            }

            let (_, response) = result.unwrap();

            let Some(items) = &response.items else { continue; };

            for item in items {
                let id_opt = item
                    .content_details
                    .as_ref()
                    .and_then(|d| d.video_id.as_ref());

                if id_opt.is_none() {
                    continue;
                }

                let id = id_opt.unwrap();

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

                let channel_name = &channel.ch_description;

                let text = format!(
                        "Hey @everyone, **{channel_name}** has released a new video!\nhttps://youtube.com/watch?v={id}"
                    );

                ChannelId::from(channel.ch_discord_channel_id as u64)
                    .send_message(&ctx, |msg| {
                        if self.debug_mode {
                            msg.content(text).allowed_mentions(|am| am.empty_parse())
                        } else {
                            msg.content(text)
                                .allowed_mentions(|am| am.empty_parse().parse(ParseValue::Everyone))
                        }
                    })
                    .await?;
            }
        }

        Ok(())
    }
}
