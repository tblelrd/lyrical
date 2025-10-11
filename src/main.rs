use anyhow::Result;
use kakasi::{is_japanese, IsJapanese};
use serde_json::Value;
use std::{process::Command, thread, time::Duration};

const BASE_URL: &str = "https://lrclib.net/api/search";

#[derive(Clone, Debug, Default)]
struct Metadata {
    title: String,
    artist: String,
    album: String,
}

impl PartialEq for Metadata {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.artist == other.artist && self.album == other.album
    }
}

#[derive(Debug, Default)]
struct LyricObject {
    metadata: Metadata,
    lyrics: Vec<(f32, String)>,
}

impl LyricObject {
    fn with_metadata(metadata: Metadata) -> Self {
        Self {
            metadata,
            lyrics: vec![],
        }
    }

    async fn from_metadata(metadata: Metadata) -> Option<Self> {
        let request = format!(
            "{}?track_name={}&artist_name={}&album_name={}", 
            BASE_URL,
            metadata.title,
            metadata.artist,
            metadata.album,
        );

        eprintln!("Requesting: {}", request);

        let res = reqwest::get(request).await.ok()?;
        let body = res.text().await.ok()?;
        let json: Value = serde_json::from_str(&body).ok()?;

        if !json.is_array() {
            eprintln!("Not an array");
            return None;
        }

        let results = json.as_array()?;
        let lyrics = get_lyrics_from_results(results)?;
        eprintln!("Found lyrics");
        
        let lyrics: Vec<(f32, String)> = lyrics
            .split("\n")
            .map(|s| s.to_string())
            .filter(|s| s.len() > 0)
            .map(|s| (get_time_from_string(&s[..10]).expect("Somehow can't get time from string"), s[10..].trim().to_string()))
            .collect();

        Some(Self {
            metadata,
            lyrics,
        })
    }

    fn get_line_at_time(&self, seconds: f32) -> (usize, String) {
        match self.lyrics.iter().enumerate().rev().find(|(_i, lyric)| seconds > lyric.0) {
            Some((i, lyric)) => (i, lyric.1.clone()),
            None => (usize::MAX, "".to_string()), // Ignore this horrible line
        }
    }
}

fn get_lyrics_from_results(results: &Vec<Value>) -> Option<String> {
    for result in results {
        if result["syncedLyrics"].is_string() {
            return Some(result["syncedLyrics"].as_str()?.to_string());
        }
    }
    None
}

fn command(command: &str) -> String {
    let mut parts = command.split_whitespace().collect::<Vec<&str>>();

    let stdout = Command::new(parts.remove(0))
        .args(parts)
        .output()
        .unwrap_or_else(|error| panic!("Failed to execute command '{}' with error '{}'", command, error))
        .stdout;

    String::from_utf8(stdout).expect("Stdout was not valid UTF-8")
}

fn get_position() -> f32 {
    command("playerctl -p spotify position")
        .trim()
        .parse()
        .unwrap_or(0.)
        // .expect("Command returned with non-float answer")
}

fn get_metadata() -> Option<Metadata> {
    let title = command("playerctl -p spotify metadata title").trim().to_string();
    let artist = command("playerctl -p spotify metadata artist").trim().to_string();
    let album = command("playerctl -p spotify metadata album").trim().to_string();

    // No title and artist means not searchable
    if title.is_empty() && artist.is_empty() {
        return None;
    }

    Some(Metadata {
        title,
        artist,
        album,
    })
}

fn get_time_from_string(time_string: impl ToString) -> Option<f32> {
    // [MM:SS.ms]
    let time = time_string.to_string();
    let minutes: u8 = time[1..3].parse().ok()?;
    let seconds: u8 = time[4..6].parse().ok()?;
    let milli_seconds: u8 = time[7..9].parse().ok()?;

    let position: f32 = (minutes as f32 * 60.) + (seconds as f32) + (milli_seconds as f32 / 100.);

    Some(position)
}

#[tokio::main]
async fn main() -> Result<()> {
    // let mut metadata: Option<Metadata> = None;
    let mut lyrics: Option<LyricObject> = None;

    let mut prev_index = usize::MAX;
    loop {
        thread::sleep(Duration::from_secs_f32(0.1));

        let metadata = get_metadata();
        if match (&metadata, &lyrics) {
            // True if metadata and lyrics exist and are different
            // e.g. changing songs
            (Some(metadata), Some(lyrics)) => metadata != &lyrics.metadata,
            // True if lyrics exist but metadata doesn't
            // e.g. no more songs
            (None, Some(_)) => true,
            // True if metadata exists but lyrics don't
            // e.g. just started playing
            (Some(_), None) => true,
            // Else false
            // e.g. still playing same song
            _ => false,
        } {
            // Switch lyrics object to be with new metadata, or no metadata.
            lyrics = match &metadata {
                Some(metadata) => match LyricObject::from_metadata(metadata.clone()).await {
                    // Just return it if it exists
                    Some(lyrics) => Some(lyrics),
                    // Return an empty lyrics object if not
                    None => Some(LyricObject::with_metadata(metadata.clone()))
                },
                None => None,
            }
        }

        if let Some(lyrics) = &lyrics {
            let (index, mut line) = lyrics.get_line_at_time(get_position());

            if prev_index == index { continue; }
            prev_index = index;

            if matches!(is_japanese(&line), IsJapanese::True) {
                line = kakasi::convert(line).romaji;
            }

            println!("{}", line);
        }
    }
}
