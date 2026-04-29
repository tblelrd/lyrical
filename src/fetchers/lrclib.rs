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
            .filter_map(|json| Lyrics::from_lrc_json(json))
            .collect();

        Some(lyrics)
    } else {
        Lyrics::from_lrc_json(&json).map(|l| vec![l])
    }
}

/// Converts LRC formatted lyrics to the timestamped lyrics tuple
pub fn convert_lrc(unformatted: String) -> Option<Vec<(f64, String)>> {
    unformatted.split('\n')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| {
            get_timestamp(s)
        })
        // Ignore non timestamp tags.
        .filter(|s| match s {
            // s.as_ref().is_some_and(|(t, _)| *t > 0.)
            Some((t, _)) => *t > 0.,
            None => true,
        })
        .collect()
}

/// Is [None] when:
/// - no tag with non-empty line
/// - should stop parsing.
/// Is Some((-1, line)) when:
/// - Should be ignored
/// Is Some((n, line)) where n > 0 when:
/// - valid timestamp, should keep
fn get_timestamp(line: String) -> Option<(f64, String)> {
    // If theres no tag, then there's no timestamp.
    let (Some(tag), line) = split_on_tag(&line) else {
        if line.is_empty() { return None } else { return Some((-1., line)) }
    };

    let Some((minutes, seconds, milliseconds)): Option<(f64, f64, f64)> = tag.split_once(':')
        .and_then(|(minutes, rest)| rest
            .split_once('.')
            .and_then(
                |(seconds, milliseconds)| Some((
                    minutes.parse().ok()?,
                    seconds.parse().ok()?,
                    milliseconds.parse().ok()?,
                )),
            )
        )
    else {
        // Non timestamp tag, should ignore
        return Some((-1., line));
    };

    Some(((minutes * 60.) + (seconds) + (milliseconds / 100.), line))
}

/// Takes a string in format
/// `"[X]Y"` and returns
/// `(Option<X>], Y)`
fn split_on_tag(line: &str) -> (Option<String>, String){
    let line = line.trim();

    if line.starts_with('[') {
        match line.split_once(']') {
            Some((tag, line)) => (
                tag.strip_prefix('[').map(|s| s.to_string()),
                line.trim().to_string(),
            ),
            None => (None, line.to_string()),
        }
    } else {
        (None, line.to_string())
    }
}
