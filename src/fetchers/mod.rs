use crate::{fetchers::lrclib::Lrclib, info_log, lyrics::Lyrics, song::SongData};

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
pub async fn fetch_all(data: &SongData) -> Option<Lyrics> {
    info_log(format!("Requesting {}", data.get_title_truncated(MAX_TITLE_LENGTH)));

    let choices = Lrclib::fetch(data).await?;
    choices.into_iter().nth(0)
}
