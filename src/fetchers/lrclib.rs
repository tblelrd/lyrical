use std::time::Duration;

use futures::{StreamExt, stream};
use reqwest::Url;
use serde_json::Value;
use tokio::pin;

use crate::{fetchers::{Fetcher, MAX_DEVIATION, MAX_TITLE_LENGTH, RESPONSE_TIMEOUT}, info_log, lyrics::Lyrics, song::SongData};

pub struct Lrclib;

const BASE_URL: &str = "https://lrclib.net/";

impl Fetcher for Lrclib {
    async fn fetch(data: &SongData) -> Option<Vec<Lyrics>> {
        // As precise as possible
        let mut url_get = Url::parse(BASE_URL).unwrap();
        url_get.set_path("api/get");
        url_get.query_pairs_mut().append_pair("track_name", &data.title);

        if let Some(artist) = &data.artist {
            url_get.query_pairs_mut().append_pair("artist_name", artist);
        }
        if let Some(album) = &data.album {
            url_get.query_pairs_mut().append_pair("album_name", album);
        }
        if let Some(duration) = &data.duration {
            url_get.query_pairs_mut().append_pair("duration", &duration.to_string());
        }

        // An imprecise search
        let mut url_search = Url::parse(BASE_URL).unwrap();
        url_search.set_path("api/search");
        url_search.query_pairs_mut().append_pair("q", &data.title);

        // Request them at the same time.
        let race = stream::iter(vec![
            request_and_parse(url_get),
            request_and_parse(url_search),
        ]).buffer_unordered(2)
            .filter_map(|choices| async move {
                let choices = choices?;
                let Some(duration) = data.duration else { return Some(choices) };

                // Only allow a max deviation of duration.
                let choices: Vec<Lyrics> = choices
                    .into_iter()
                    .filter(|l| (l.duration - duration).abs() < MAX_DEVIATION)
                    .collect();

                Some(choices)
            })
            .filter(|v| {
                let empty = v.is_empty();
                async move { !empty }
            });
        pin!(race);

        let mut choices: Vec<Lyrics> = vec![];
        let mut timeout = Box::pin(
            tokio::time::sleep(Duration::from_secs_f64(RESPONSE_TIMEOUT)),
        );

        loop {
            tokio::select! {
                n = race.next() => {
                    let Some(mut n) = n else {
                        // No more responses
                        break;
                    };
                    choices.append(&mut n);
                }
                // Timeout and at least one response.
                _ = (&mut timeout) => if !choices.is_empty() {
                    info_log("Timed out, only some lyrics found");
                    break;
                }
            }
        }

        if choices.is_empty() {
            info_log(format!("Couldn't retrieve lyrics for {}", data.get_title_truncated(MAX_TITLE_LENGTH)));
            return None;
        }

        // Sort lyrics by closest duration.
        if let Some(duration) = data.duration {
            choices.sort_by(|a, b| (a.duration - duration).abs().total_cmp(&(b.duration - duration).abs()));
        }

        Some(choices)
    }
}

/// Just requests and parses an lrclib response into a list of lyrics.
async fn request_and_parse(request: Url) -> Option<Vec<Lyrics>> {
    let response = match reqwest::get(request).await {
        Ok(res) => res,
        Err(e) => {
            info_log(format!("Error requesting: {:?}", e));
            return None;
        },
    };

    let body = response.text().await.ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;

    if json.is_array() {
        let lyrics: Vec<Lyrics> = json.as_array()?
            .iter()
            .filter_map(|json| Lyrics::from_json(json))
            .collect();

        Some(lyrics)
    } else {
        Lyrics::from_json(&json).map(|l| vec![l])
    }
}
