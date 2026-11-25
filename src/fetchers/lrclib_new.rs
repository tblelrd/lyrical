use crate::LYRICAL_GITHUB_REPO;
use futures::future::OptionFuture;
use reqwest::{Client, Url, header::USER_AGENT};
use serde_json::Value;

const BASE_URL: &str = "https://lrclib.net";

#[derive(Debug)]
pub struct LRCClient {
    lrc_url: String,
    user_agent: String,

    // Internally an arc so clonable i think.
    client: Client,
}

impl LRCClient {
    pub fn new(client: Client) -> Self {
        let version = env!("CARGO_PKG_VERSION");

        Self {
            lrc_url: BASE_URL.into(),
            user_agent: format!(
                "Lyrical v{} ({})",
                version,
                LYRICAL_GITHUB_REPO,
            ),
            client: client,
        }
    }

    pub async fn request_get(
        &self,
        track_name: &str,
        artist_name: &str,
        album_name: &str,
        duration: &str,
    ) -> Option<Value> {
        let mut url = Url::parse(&format!("{}/api/get", self.lrc_url)).unwrap();
        url.query_pairs_mut()
            .append_pair("track_name", track_name)
            .append_pair("artist_name", artist_name)
            .append_pair("album_name", album_name)
            .append_pair("duration", duration);

        self.request_url(url).await
   }

    pub async fn request_search(
        &self,
        query: &str,
    ) -> Option<Value> {
        let mut url = Url::parse(&format!("{}/api/search", self.lrc_url)).unwrap();
        url.query_pairs_mut()
            .append_pair("q", query);

        self.request_url(url).await
    }

    async fn request_url(&self, url: Url) -> Option<Value> {
        Into::<OptionFuture<_>>::into(
            self.client.get(url)
                .header(USER_AGENT, &self.user_agent)
                .send()
                .await
                .ok()
                .map(|res| async { res.text().await }),
        ).await
            .and_then(|res| res.ok())
            .and_then(|text| serde_json::from_str(&text).ok())
    }
}
