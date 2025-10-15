use anyhow::Result;
use kakasi::{is_japanese, IsJapanese};
use serde_json::Value;
use std::{cmp::Ordering, process::Command, thread, time::Duration};

const BASE_URL: &str = "https://lrclib.net/api/search";

fn print_err(message: impl ToString) {
    println!("[INFO] {}", message.to_string());
}

#[derive(Debug, Default)]
enum Language {
    Chinese,
    Japanese,

    #[default]
    Other,
}

#[derive(Clone, Debug, Default)]
struct Metadata {
    title: String,
    artist: String,
    album: String,
    duration: Option<f64>,
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
    language: Language,
}

impl LyricObject {
    fn with_metadata(metadata: Metadata) -> Self {
        Self {
            metadata,
            language: Language::Other,
            lyrics: vec![],
        }
    }

    async fn from_metadata(metadata: Metadata) -> Option<Self> {
        let (language, lyrics) = request_lyrics(&metadata).await?;

        Some(Self {
            metadata,
            language,
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

async fn request_lyrics(metadata: &Metadata) -> Option<(Language, Vec<(f32, String)>)> {
    let request = format!(
        "{}?track_name={}&artist_name={}&album_name={}", 
        BASE_URL,
        metadata.title,
        metadata.artist,
        metadata.album,
    );
    print_err(format!("Requesting: {}", request));

    let res = reqwest::get(request).await.ok()?;
    print_err("Response receieved, parsing...");

    let body = res.text().await.ok()?;
    let json: Value = serde_json::from_str(&body).ok()?;

    if !json.is_array() {
        print_err("Not an array");
        return None;
    }

    let mut results = json.as_array()?.clone();
    match metadata.duration {
        Some(duration) => {
            results.sort_by(|a, b| {
                let a_duration = &a["duration"];
                let b_duration = &b["duration"];

                match (a_duration.is_f64(), b_duration.is_f64()) {
                    (true, true) => 
                        (a_duration.as_f64().unwrap() - duration).abs()
                        .total_cmp(
                            &(b_duration.as_f64().unwrap() - duration).abs()
                        ),
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => Ordering::Equal,
                }
            });
        },
        None => {},
    };
    let (language, lyrics) = get_lyrics_from_results(&results)?;
    print_err(format!("Found lyrics in {:?}", language));

    Some((language, lyrics))
}


fn get_lyrics_from_results(results: &Vec<Value>) -> Option<(Language, Vec<(f32, String)>)> {
    for result in results {
        if result["syncedLyrics"].is_string() {
            // Check if the synced lyrics are actually strings.
            let synced_lyrics = if let Some(ref lyrics) = result["syncedLyrics"].as_str() {
                lyrics.to_string()
            } else {
                continue;
            };

            // Create a timestamped list of lyrics, if format wrong then go next.
            let timestamped_lyrics: Vec<(f32, String)> = if let Some(lyrics) = synced_lyrics
                .split('\n')
                .map(|s| s.trim().to_string())
                .filter(|s| s.len() > 0)
                // Map into an option of a tuple.
                // The ? syntax will propogate the None up into the full option
                // so instead of (Option<f32, String) it will be Option<(f32, String)>
                // That's why the Some is at the front, to explicitly declare that it will
                // return an option.
                .map(|s| Some((get_time_from_string(&s[..10])?, s[10..].trim().to_string())))
                // The turbofish is to tell collect to use the option implementation rather than the
                // default vec<> collect method.
                // Then we do ? again to propogate any none values in the lyrics to return
                // none.
                .collect::<Option<_>>()
            {
                lyrics
            } else {
                continue;
            };

            // Check what language the lyrics are in.
            let language = if let Some(lyrics) = result["plainLyrics"].as_str() {
                match is_japanese(lyrics) {
                    IsJapanese::False => Language::Other,
                    IsJapanese::Maybe => Language::Chinese,
                    IsJapanese::True => Language::Japanese,
                }
            } else {
                Language::Other
            };
            
            return Some((language, timestamped_lyrics));
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
    let duration = if let Some(duration) = command("playerctl -p spotify metadata mpris:length").trim().parse::<u32>().ok() {
        Some(duration as f64 / 1e6)
    } else {
        None
    };

    // No title and artist means not searchable
    if title.is_empty() && artist.is_empty() {
        return None; 
    }

    Some(Metadata {
        title,
        artist,
        album,
        duration,
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

fn to_pinyin(line: &str) -> String {
    let mut translated = String::new();
    let mut last_pinyin = false;
    for character in line.chars() {
        let res = match mandarin_to_pinyin::lookup_chars(&[character]) {
            Ok(pinyin) => match pinyin.vec[0] {
                Some(ref result) => result[0].clone(),
                None => character.to_string()
            },
            Err(_) => character.to_string(),
        };

        if last_pinyin { translated += " " };
        translated += &res;
        last_pinyin = res.len() != 1;
    }

    translated.trim().to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize chinese to pinyin map
    mandarin_to_pinyin::init_map(None).expect("Cant be bothered catching this one");

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
                    None => {
                        print_err(format!("Couldn't find lyrics for song {:?}", metadata));
                        Some(LyricObject::with_metadata(metadata.clone()))
                    },
                },
                None => None,
            }
        }

        if let Some(lyrics) = &lyrics {
            let (index, mut line) = lyrics.get_line_at_time(get_position());

            if prev_index == index { continue; }
            prev_index = index;

            match lyrics.language {
                Language::Japanese => line = kakasi::convert(line).romaji,
                Language::Chinese => line = to_pinyin(&line),
                Language::Other => {},
            }

            println!("{}", line);
        }
    }
}
