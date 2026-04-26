use crate::{cache::Cache, fetchers::lrclib::Lrclib, info_log, lyrics::Lyrics, song::SongData};

pub mod lrclib;

/// How many characters can be displayed before truncated.
const MAX_TITLE_LENGTH: usize = 25;

/// How long each request waits before giving up.
const RESPONSE_TIMEOUT: f64 = 5.0;

/// Maximum duration difference for lyrics to be considered.
const MAX_DEVIATION: f64 = 1.;

pub(crate) trait Fetcher {
    async fn fetch(data: &SongData) -> Option<Vec<Lyrics>>;
}

/// Fetch the lyrics from every source.
/// Currently just LRCLIB tho.
pub async fn fetch_all(data: &SongData, cache: &mut Cache) -> Option<Lyrics> {
    if let Some(lyrics) = cache.get_lyrics(data) {
        info_log(format!("Found cached lyrics for {}", data.get_title_truncated(MAX_TITLE_LENGTH - 10)));
        return Some(lyrics);
    }

    info_log(format!("Requesting {}", data.get_title_truncated(MAX_TITLE_LENGTH)));
    let choices = Lrclib::fetch(data).await?;
    info_log(format!("Found {} lyrics", choices.len()));

    let lyrics = choices.into_iter().nth(0);
    cache.save_lyrics(data, &lyrics, 1);

    // Save 
    match cache.clone().save_to_file().await {
        Ok(_) => {},
        Err(e) => info_log(format!("Error saving cache: {}", e)),
    };

    // Reload cache (This is also quite bad, it would be better
    // just to remove what we need, so this is temporary)
    *cache = Cache::read_from_file(&cache.location, cache.max_size).await.unwrap();

    lyrics
}
