use anyhow::Result;
use kakasi::{is_japanese, IsJapanese};
use serde_json::Value;
use std::{process::Command, thread, time::Duration};

const BASE_URL: &str = "https://lrclib.net/api/search";

#[derive(Debug)]
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

#[derive(Debug)]
struct LyricObject {
    metadata: Metadata,
    lyrics: Vec<(f32, String)>,
}

impl LyricObject {
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
            .map(|s| (get_time_from_string(&s[..10]).expect("Couldn't get time somehow"), s[10..].trim().to_string()))
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
    command("playerctl position")
        .trim()
        .parse()
        .expect("Command returned with non-float answer")
}

fn get_metadata() -> Metadata {
    let title = command("playerctl metadata title").trim().to_string();
    let artist = command("playerctl metadata artist").trim().to_string();
    let album = command("playerctl metadata album").trim().to_string();

    Metadata {
        title,
        artist,
        album,
    }
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
    let metadata = get_metadata();

    let mut lyrics = LyricObject::from_metadata(metadata).await.ok_or(anyhow::Error::msg("Couldn't find song"))?;

    eprintln!("Starting loop...");
    let mut prev_index = usize::MAX;
    loop {
        let current_metadata = get_metadata();
        if current_metadata != lyrics.metadata {
            eprintln!("New song detected, loading lyrics");
            lyrics = LyricObject::from_metadata(current_metadata).await.ok_or(anyhow::Error::msg("Couldn't find song"))?;
        }

        let (index, line) = lyrics.get_line_at_time(get_position());
        if index != prev_index {
            if matches!(is_japanese(&line), IsJapanese::True) {
                let conversion = kakasi::convert(&line);
                println!("{}", conversion.romaji)
            } else {
                println!("{}", line);
            }
            prev_index = index;
        }
        
        thread::sleep(Duration::from_secs_f32(0.1));
    }
}
