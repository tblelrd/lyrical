use std::time::Duration;

use futures::{StreamExt, stream};
use reqwest::Url;
use serde_json::Value;
use tokio::time::timeout;

use crate::{command, info_log, lyrics::Lyrics};

const BASE_URL: &str = "https://lrclib.net/api/";

/// Maximum duration difference for lyrics to be considered.
const MAX_DEVIATION: f64 = 5.;

/// Maximum number of concurrent requests (This doesn't need to be above 4).
const MAX_CONCURRENCE: usize = 4;

/// How many characters can be displayed before truncated.
const MAX_TITLE_LENGTH: usize = 30;

/// How long each request waits before giving up.
const RESPONSE_TIMEOUT: f64 = 5.0;

/// Stores the metadata and the lyrics
pub struct Song {
    pub data: SongData,
    pub lyrics: Option<Lyrics>,
}

impl Song {
    /// Creates a song object using the song metadata.
    /// The lyrics will be [None] if couldn't find.
    pub async fn request_song(data: SongData) -> Song {
        info_log(format!("Requesting {}", data.get_title_truncated(MAX_TITLE_LENGTH)));

        // Use 0 to 3 for the precision.
        let mut choices: Vec<Lyrics> = stream::iter((0..=3).into_iter())
            .map(|i| data.request_lyrics(i))
            // Concurrently request those lyrics.
            .buffer_unordered(MAX_CONCURRENCE)
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
            // Remove empty lists.
            .filter(|v| {
                let empty = v.is_empty();
                async move { !empty }
            })
            // Convert Vec to Stream then flattens.
            .flat_map(stream::iter)
            .collect().await;

        if choices.is_empty() {
            info_log(format!("Couldn't retrieve lyrics for {}", data.get_title_truncated(MAX_TITLE_LENGTH)));
            return Song {
                data,
                lyrics: None,
            }
        }

        // Sort lyrics by closest duration.
        if let Some(duration) = data.duration {
            choices.sort_by(|a, b| (a.duration - duration).abs().total_cmp(&(b.duration - duration).abs()));
        }

        // Pick the lyrics with the lowest duration
        let lyrics = choices.into_iter().nth(0);
        info_log("Lyrics successfully found");
        Song {
            data,
            lyrics,
        }
    }
}

/// Data about the song, can be gathered from playerctl.
#[derive(Debug, PartialEq)]
pub struct SongData {
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration: Option<f64>,
}

impl SongData {
    /// Gets the metadata, prioritising spotify, if exists.
    pub fn get_data() -> Option<Self> {
        // Prefer spotify data.
        let spotify_data = SongData::get_data_from_player("spotify");

        // Or just get it from nothing.
        spotify_data.or(SongData::get_data_from_player(""))
    }

    /// Gets the title of the song with a max size.
    fn get_title_truncated(&self, max_length: usize) -> String {
        if self.title.len() <= max_length - 3 {
            self.title.clone()
        } else {
            format!("{}...", self.title[..max_length-3].to_string())
        }
    }

    /// Gets the metadata of a song from a specified player.
    fn get_data_from_player(player: &str) -> Option<Self> {
        let flag = format!(" -p {player}");
        let playerctl = "playerctl".to_string() +
            if player.is_empty() { "" } else { &flag } +
            " metadata ";

        let get_attr = |name: &str|
            Some(
                command(&(playerctl.clone() + name)).trim().to_string(),
            ).filter(|s| !s.is_empty());
    
        // Title required.
        let title = get_attr("title")?;

        let artist = get_attr("artist");
        let album = get_attr("album");
        let duration = get_attr("mpris:length")
            // Maps to an Option<Result<_>>
            .map(|d| d.parse::<f64>())
            // Flattens to just Option<_> and None if Err.
            .and_then(|result| result.ok())
            .map(|d| d / 1e6);

        Some(Self {
            title,
            artist,
            album,
            duration,
        })
    }

    /// Request and parse lyrics from the website.
    async fn request_lyrics(&self, precision: u8) -> Option<Vec<Lyrics>> {
        let request = self.format_request(precision);

        // Don't spend forever waiting for lyrics.
        let res = timeout(
            Duration::from_secs_f64(RESPONSE_TIMEOUT),
            reqwest::get(request),
        ).await.ok().or_else(|| {
            info_log(" Timed Out");
            None
        })?.ok()?;

        let body = res.text().await.ok()?;
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

    /// Format the request with variable precision.
    /// Most precise at 0,
    /// Least precise at 3.
    fn format_request(&self, precision: u8) -> Url {
        let precise =
            precision == 0 &&
            self.artist.is_some() &&
            self.album.is_some() &&
            self.duration.is_some();

        let mut attributes =
            self.artist.is_some() as u8 +
            self.album.is_some() as u8 +
            self.duration.is_some() as u8;

        // Reduce by precision.
        attributes -= precision;

        // Don't use duration when imprecise.
        if !precise && attributes == 3 { attributes -= 1}

        let mut url = Url::parse(BASE_URL).expect("Invalid url??");
        url.set_path(if precise {
            "api/get"
        } else {
            "api/search"
        });
        url.query_pairs_mut()
            .append_pair("track_name", &self.title);

        let add_pair = |precision: &mut u8, url: &mut Url, key: &str, value: &str| {
            if *precision <= 0 { return }
            *precision -= 1;

            url.query_pairs_mut().append_pair(key, value);
        };


        if let Some(artist) = &self.artist {
            add_pair(&mut attributes, &mut url, "artist_name", artist);
        }

        if let Some(album) = &self.album {
            add_pair(&mut attributes, &mut url, "album_name", album);
        }

        if let Some(duration) = &self.duration {
            add_pair(&mut attributes, &mut url, "duration", &duration.to_string());
        }

        url
    }
}
