use reqwest::{Client, Method, Request, RequestBuilder, Response, Url, header::USER_AGENT};

use crate::{LYRICAL_GITHUB_REPO, song::SongData};

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
    ) {
        let mut url = Url::parse(&format!("{}/api/get", self.lrc_url)).unwrap();
        url.query_pairs_mut()
            .append_pair("track_name", track_name)
            .append_pair("artist_name", artist_name)
            .append_pair("album_name", album_name)
            .append_pair("duration", duration);

        let res = self.client.get(url)
            .header(USER_AGENT, &self.user_agent)
            .send().await;

        dbg!(res);
   }

    pub async fn request_search(
        &self,
        query: &str,
    ) {
        let mut url = Url::parse(&format!("{}/api/search", self.lrc_url)).unwrap();
        url.query_pairs_mut()
            .append_pair("q", query);

        dbg!(&url.to_string());

        let res = self.client.get(url)
            .header(USER_AGENT, &self.user_agent)
            .send().await;

        dbg!(res);
    }
}
