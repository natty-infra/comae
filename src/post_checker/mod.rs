pub mod reddit_posts;
pub mod youtube_uploads;

use std::{borrow::Cow, error::Error};

use feed_rs::model::Feed;
use reqwest::Client;

#[async_trait::async_trait]
pub trait Checker {
    fn name(&self) -> &str;

    async fn check(
        &self,
        ctx: impl AsRef<serenity::http::Http> + Send + Sync,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

async fn fetch_rss(
    client: &Client,
    url: Cow<'_, str>,
) -> Result<Feed, Box<dyn Error + Send + Sync>> {
    let bytes = client
        .get(url.as_ref())
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    feed_rs::parser::parse(bytes.as_ref()).map_err(|e| e.into())
}
