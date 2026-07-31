use serde::{Deserialize, Serialize};

use crate::{lyrics::Lyrics};

/// Stores the metadata and the lyrics
#[derive(Clone, Debug)]
pub struct Song {
    pub data: SongData,
    pub lyrics: Option<Lyrics>,
}

impl Song {
    /// Constructor for [Song].
    pub fn new(data: SongData, lyrics: Option<Lyrics>) -> Self {
        Song {
            data,
            lyrics,
        }
    }
}

/// Data about the song, can be gathered from playerctl.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SongData {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Duration in seconds.
    pub duration: Option<f64>,
}

impl SongData {
    /// Gets the metadata, prioritising spotify, if exists.
    pub fn get_data(player: &mpris::Player) -> Option<Self> {
        let metadata = player.get_metadata().ok()?;
        Some(Self {
            title: metadata.title()?.to_string(),
            artist: metadata.artists().and_then(|a| a.first().map(|a| a.to_string())),
            album: metadata.album_name().map(|a| a.to_string()),
            duration: metadata.length().map(|d| d.as_secs_f64()),
        })
    }

    /// Gets the title of the song with a max size.
    pub fn get_title_truncated(&self, max_length: usize) -> String {
        if self.title.len() <= max_length {
            self.title.clone()
        } else {
            format!("{}...", self.title.chars().take(max_length).collect::<String>())
        }
    }
}
