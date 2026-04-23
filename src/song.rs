use serde::{Deserialize, Serialize};

use crate::{command, lyrics::Lyrics};

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
    pub duration: Option<f64>,
    pub player: Option<Player>,
}

/// The player that is used to get the metadata.
/// This is only if a specific player is used.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Player {
    Spotify
}

impl ToString for Player {
    fn to_string(&self) -> String {
        match self {
            Self::Spotify => "spotify"
        }.to_string()
    }
}

pub fn get_flag_from_player(player: &Option<Player>) -> String {
    if let Some(player) = &player {
        format!("-p {}", player.to_string())
    } else { "".to_string() }
}

impl SongData {
    /// Gets the metadata, prioritising spotify, if exists.
    pub fn get_data() -> Option<Self> {
        // Prefer spotify data.
        let spotify_data = SongData::get_data_from_player(Some(Player::Spotify));

        // Or just get it from nothing.
        spotify_data.or(SongData::get_data_from_player(None))
    }

    /// Gets the title of the song with a max size.
    pub fn get_title_truncated(&self, max_length: usize) -> String {
        if self.title.len() <= max_length {
            self.title.clone()
        } else {
            format!("{}...", self.title.chars().take(max_length).collect::<String>())
        }
    }

    /// Gets the metadata of a song from a specified player.
    fn get_data_from_player(player: Option<Player>) -> Option<Self> {
        let flag = get_flag_from_player(&player);
        let metadata_command = format!("playerctl {flag} metadata ");

        let get_attr = |name: &str|
            Some(
                command(&(metadata_command.clone() + name)).trim().to_string(),
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
            player,
        })
    }
}
