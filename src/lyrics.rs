use clap::ValueEnum;

use kakasi::IsJapanese;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fetchers::lrclib::convert_lrc;

#[derive(Serialize, Deserialize, Clone, Debug, Default, ValueEnum, PartialEq)]
/// Languages currently supported to be romanized.
pub enum Language {
    /// This should be English and
    /// other latin alphabet based langauges,
    /// or unsupported languages.
    #[default]
    #[value(hide = true)]
    Other,

    /// Chinese characters into pinyin.
    #[value(name = "zh")]
    Chinese,

    /// Japanese characters (and kanji) into romanji.
    #[value(name = "ja")]
    Japanese,

    /// Korean characters to latin script.
    #[value(name = "ko")]
    Korean,
}

/// A struct that contains the language and the lyrics
/// of a song. The lyrics should be timestamped.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Lyrics {
    pub language: Language,
    pub duration: f64,
    pub lyrics: Vec<(f64, String)>,
}

impl Lyrics {
    /// Creates a [Lyrics] object from
    /// a json response [Value] from LRCLIB.net
    /// using the `syncedLyrics` attribute.
    pub fn from_lrc_json(object: &Value) -> Option<Lyrics> {
        let synced = object["syncedLyrics"].as_str()?.to_string();

        let timestamped: Vec<(f64, String)> = convert_lrc(synced)?;

        let duration = object["duration"].as_f64()?;

        let language = object["plainLyrics"].as_str().map_or_else(
            // No plainLyrics, make plain lyrics.
            || get_language_from_text(&timestamped
                .iter()
                .map(|(_, line)| line)
                .fold(String::new(), |a, b| a + b + &"\n")),
            // Yes plainLyrics, just plug that in.
            get_language_from_text
        );

        Some(Lyrics {
            language,
            duration,
            lyrics: timestamped,
        })
    }

    /// Gets a reference to the line at the timestamp.
    pub fn get_line_at_time(&self, time: f64) -> &str {
        match self.lyrics.iter().rev().find(|l| time > l.0) {
            Some(l) => &l.1,
            None => "",
        }
    }
}

fn get_language_from_text(lyrics: &str) -> Language {
    match kakasi::is_japanese(lyrics) {
        IsJapanese::Maybe => Language::Chinese,
        IsJapanese::True => Language::Japanese,
        IsJapanese::False => {
            if korean_romanize::has_korean(lyrics) {
                Language::Korean
            } else {
                Language::Other
            }
        },
    }
}
